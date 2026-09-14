//! Durable entry capture orchestration.
//!
//! Capture is deliberately split from context providers and terminal output:
//! the only operation in the write transaction is the Capsule entry mutation.
//! A verified backup and a database-file binding are established before that
//! transaction, and the returned receipt is assembled from values already
//! known at commit rather than from a fallible post-commit detail query.

#![allow(clippy::result_large_err)]

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    backup,
    contracts::{BackupPolicy, CaptureOutcome, CaptureRequest, CommitReceipt},
    db,
    entries::{self, EntryCommit, MutationPoint, NormalizedEntry},
    identity,
};

/// Explicit failure points are available only through an injected hook.  The
/// production convenience functions always use a no-op hook and cannot be
/// sabotaged by an environment variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureHookPoint {
    BeforeBackup,
    AfterBackup,
    BeforeBegin,
    BeforeInsert,
    BeforeFts,
    BeforeResequence,
    BeforeCommit,
    DuringCommit,
    AfterCommit,
}

/// Test/integration seams for fault and interruption matrices.  Implementors
/// may return an error at any explicit point; errors before commit are
/// `not_committed`, a commit API error is `unknown`, and an after-commit error
/// retains the confirmed receipt as `committed`.
pub trait CaptureHooks: Send + Sync {
    fn checkpoint(&self, _point: CaptureHookPoint) -> Result<()> {
        Ok(())
    }
}

impl<F> CaptureHooks for F
where
    F: Fn(CaptureHookPoint) -> Result<()> + Send + Sync,
{
    fn checkpoint(&self, point: CaptureHookPoint) -> Result<()> {
        self(point)
    }
}

#[derive(Debug, Default)]
struct NoopCaptureHooks;

impl CaptureHooks for NoopCaptureHooks {}

/// A durable status that callers can persist in a pending/recovery record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub outcome: CaptureOutcome,
    pub capture_id: String,
    pub reserved_uuid: String,
    pub receipt: Option<CommitReceipt>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureErrorCode {
    InvalidInput,
    DatabaseBusy,
    DatabaseReplaced,
    BackupFailed,
    IdentityConflict,
    CommitUnknown,
    CommittedReceiptUnavailable,
}

impl CaptureStatus {
    pub fn not_committed(request: &CaptureRequest, message: Option<String>) -> Self {
        Self {
            outcome: CaptureOutcome::NotCommitted,
            capture_id: request.capture_id.clone(),
            reserved_uuid: request.reserved_uuid.clone(),
            receipt: None,
            message,
        }
    }

    pub fn committed(request: &CaptureRequest, receipt: CommitReceipt) -> Self {
        Self {
            outcome: CaptureOutcome::Committed,
            capture_id: request.capture_id.clone(),
            reserved_uuid: request.reserved_uuid.clone(),
            receipt: Some(receipt),
            message: None,
        }
    }
}

/// Structured capture failure.  Keeping the outcome beside the error prevents
/// a caller from treating an uncertain commit as an ordinary retry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureError {
    pub code: CaptureErrorCode,
    pub outcome: CaptureOutcome,
    pub capture_id: String,
    pub reserved_uuid: String,
    pub message: String,
    pub receipt: Option<CommitReceipt>,
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CaptureError {}

pub type CaptureResult<T> = std::result::Result<T, CaptureError>;

/// Normalize authored content using the same rules as the shared writer.
/// This function performs no I/O. Serialize the returned structure before
/// hashing it; concatenating fields with user-enterable delimiters is ambiguous.
pub fn normalize_capture_content(
    request: &CaptureRequest,
) -> Result<crate::contracts::NormalizedCaptureContent> {
    let mut normalized = entries::normalize_capture_request(request)?;
    normalized.tags.sort();
    Ok(crate::contracts::NormalizedCaptureContent {
        schema_version: 1,
        text: normalized.text,
        text_plain: normalized.text_plain,
        content_format: normalized.content_format,
        title: normalized.title,
        summary: normalized.summary,
        mood: normalized.mood,
        tags: normalized.tags,
        starred: normalized.starred,
        pinned: normalized.pinned,
        continue_from_uuid: normalized.continue_from_uuid,
    })
}

const CAPTURE_OPERATION_TIMEOUT: Duration = Duration::from_secs(15);

/// Capture with a request whose database and backup policies are already
/// frozen.  No location/weather/provider work is performed here.
pub fn capture_entry(request: CaptureRequest) -> CaptureResult<CommitReceipt> {
    capture_entry_with_hooks(request, &NoopCaptureHooks)
}

pub fn capture_entry_with_hooks(
    request: CaptureRequest,
    hooks: &dyn CaptureHooks,
) -> CaptureResult<CommitReceipt> {
    capture_entry_with_hooks_for_database(request, hooks)
}

pub fn capture_entry_for_database(request: CaptureRequest) -> CaptureResult<CommitReceipt> {
    capture_entry_with_hooks_for_database(request, &NoopCaptureHooks)
}

pub fn capture_entry_with_hooks_for_database(
    request: CaptureRequest,
    hooks: &dyn CaptureHooks,
) -> CaptureResult<CommitReceipt> {
    let deadline = capture_deadline();
    let normalized = entries::normalize_capture_request(&request)
        .map_err(|error| not_committed(&request, error.to_string()))?;
    ensure_capture_deadline(&request, deadline, "request normalization")?;
    identity::validate_capture_request(&request)
        .map_err(|error| not_committed(&request, error.to_string()))?;
    let expected_identity = identity::bind_request_database(&request).map_err(|error| {
        if request.database_identity.is_some() {
            database_replaced(&request, error.to_string())
        } else {
            not_committed(&request, error.to_string())
        }
    })?;

    // A serialized request may have been created before explicit policy was
    // introduced.  Require callers to rebind it rather than silently reading
    // mutable process settings, even when a matching row is already present.
    let policy = request.backup_policy.clone().ok_or_else(|| {
        not_committed(
            &request,
            "capture request is missing its frozen backup policy; bind it before retrying",
        )
    })?;
    validate_backup_policy(&policy).map_err(|error| not_committed(&request, error.to_string()))?;
    let reserved_uuid = identity::validate_reserved_uuid(&request.reserved_uuid)
        .map_err(|error| not_committed(&request, error.to_string()))?;
    ensure_capture_deadline(&request, deadline, "capture preflight")?;

    // Reconcile before making a backup.  A retry of a known committed UUID
    // must remain successful even if the backup directory is currently
    // unavailable; only a new write needs a fresh verified snapshot.
    match reconcile_normalized_with_timeout(
        &request.database_path,
        &request,
        &normalized,
        &expected_identity,
        Some(remaining_capture_timeout(
            &request,
            deadline,
            "capture preflight",
        )?),
    ) {
        Ok(Reconciliation::Committed(receipt)) => return Ok(receipt),
        Ok(Reconciliation::NotCommitted) => {}
        Err(error) if error.downcast_ref::<ReservedIdentityConflict>().is_some() => {
            return Err(conflict(&request, error.to_string()))
        }
        Err(error)
            if error
                .downcast_ref::<identity::DatabaseBindingError>()
                .is_some() =>
        {
            return Err(database_replaced(&request, error.to_string()))
        }
        Err(error)
            if error.downcast_ref::<backup::MutationBusy>().is_some() || is_sqlite_busy(&error) =>
        {
            return Err(database_busy(&request, error.to_string()))
        }
        Err(error) => return Err(not_committed(&request, error.to_string())),
    }

    ensure_capture_deadline(&request, deadline, "capture backup checkpoint")?;
    hooks
        .checkpoint(CaptureHookPoint::BeforeBackup)
        .map_err(|error| not_committed(&request, error.to_string()))?;
    ensure_capture_deadline(&request, deadline, "capture backup checkpoint")?;

    let path = request.database_path.clone();
    let request_for_closure = request.clone();
    let expected_for_pre_backup = expected_identity.clone();
    let expected_for_closure = expected_identity.clone();
    let normalized_for_closure = normalized.clone();
    let guarded = backup::with_database_backup_for_database_using_policy_with_timeout_and_preflight(
        &path,
        "entry.create",
        remaining_capture_timeout(&request, deadline, "capture backup")?,
        &policy,
        |db_path| identity::validate_database_binding(db_path, &expected_for_pre_backup),
        |db_path| {
            ensure_capture_deadline_anyhow(deadline, "database identity validation")?;
            identity::validate_database_binding(db_path, &expected_for_closure)?;
            hooks
                .checkpoint(CaptureHookPoint::AfterBackup)
                .map_err(|error| anyhow!(error.to_string()))?;
            ensure_capture_deadline_anyhow(deadline, "post-backup checkpoint")?;

            // Legacy ID repairs are part of the same guarded operation.  They
            // run before the immediate creation transaction and cannot race a
            // second Capsule writer because the database sidecar lock is held.
            entries::ensure_entry_ids_for_database_unlocked_with_timeout(
                db_path,
                remaining_capture_timeout_anyhow(deadline, "legacy ID repair")?,
            )?;
            ensure_capture_deadline_anyhow(deadline, "legacy ID repair")?;
            identity::validate_database_binding(db_path, &expected_for_closure)?;

            // A second process may have committed this reserved UUID while the
            // first process was waiting for the lock.  Reconcile under the
            // lock before attempting an INSERT so matching retries resolve to
            // one entry and conflicting requests fail explicitly.
            match reconcile_normalized_with_timeout(
                db_path,
                &request_for_closure,
                &normalized_for_closure,
                &expected_for_closure,
                Some(remaining_capture_timeout_anyhow(
                    deadline,
                    "locked reconciliation",
                )?),
            )? {
                Reconciliation::Committed(receipt) => Ok(CaptureCommit::Existing(receipt)),
                Reconciliation::NotCommitted => {
                    ensure_capture_deadline_anyhow(deadline, "entry transaction")?;
                    let mut mutation_hook = |point: MutationPoint| {
                        hooks.checkpoint(match point {
                            MutationPoint::BeforeBegin => CaptureHookPoint::BeforeBegin,
                            MutationPoint::BeforeInsert => CaptureHookPoint::BeforeInsert,
                            MutationPoint::BeforeFts => CaptureHookPoint::BeforeFts,
                            MutationPoint::BeforeResequence => CaptureHookPoint::BeforeResequence,
                            MutationPoint::BeforeCommit => CaptureHookPoint::BeforeCommit,
                            MutationPoint::DuringCommit => CaptureHookPoint::DuringCommit,
                        })
                    };
                    entries::create_entry_commit_for_capture_with_timeout(
                        db_path,
                        &normalized_for_closure,
                        &reserved_uuid,
                        Some(&expected_for_closure),
                        remaining_capture_timeout_anyhow(deadline, "entry transaction")?,
                        &mut mutation_hook,
                    )
                    .map(CaptureCommit::Created)
                }
            }
        },
    );

    let guarded = match guarded {
        Ok(value) => value,
        Err(error) => {
            if let Some(capture_error) = error.downcast_ref::<CaptureError>() {
                return Err(capture_error.clone());
            }
            if error.downcast_ref::<entries::CommitUncertain>().is_some() {
                return Err(unknown(
                    &request,
                    format!("entry commit outcome is uncertain: {error}"),
                ));
            }
            if error
                .downcast_ref::<entries::ReservedUuidCollision>()
                .is_some()
            {
                return Err(conflict(&request, error.to_string()));
            }
            if error.downcast_ref::<ReservedIdentityConflict>().is_some() {
                return Err(conflict(&request, error.to_string()));
            }
            if error
                .downcast_ref::<identity::DatabaseBindingError>()
                .is_some()
            {
                return Err(database_replaced(&request, error.to_string()));
            }
            if error.downcast_ref::<backup::MutationBusy>().is_some() {
                return Err(database_busy(&request, error.to_string()));
            }
            if is_sqlite_busy(&error) {
                return Err(database_busy(&request, error.to_string()));
            }
            return Err(backup_failed(&request, error.to_string()));
        }
    };

    let (receipt, should_checkpoint) = match guarded.value {
        CaptureCommit::Existing(receipt) => (receipt, false),
        CaptureCommit::Created(commit) => (
            receipt_from_commit(&request, commit, &guarded.audit.backup_path),
            true,
        ),
    };

    if should_checkpoint {
        if let Err(error) = hooks.checkpoint(CaptureHookPoint::AfterCommit) {
            return Err(CaptureError {
                code: CaptureErrorCode::CommittedReceiptUnavailable,
                outcome: CaptureOutcome::Committed,
                capture_id: request.capture_id,
                reserved_uuid: request.reserved_uuid,
                message: format!("entry committed but post-commit receipt work failed: {error}"),
                receipt: Some(receipt),
            });
        }
    }

    Ok(receipt)
}

/// Read-only reconciliation for a pending capture.  It never repairs IDs,
/// creates a backup, or deduplicates by text; only the reserved UUID can match.
pub fn reconcile_capture_for_database(request: &CaptureRequest) -> CaptureResult<CaptureStatus> {
    identity::validate_capture_request(request)
        .map_err(|error| not_committed(request, error.to_string()))?;
    let normalized = entries::normalize_capture_request(request)
        .map_err(|error| not_committed(request, error.to_string()))?;
    let expected = identity::bind_request_database(request).map_err(|error| {
        if request.database_identity.is_some() {
            database_replaced(request, error.to_string())
        } else {
            not_committed(request, error.to_string())
        }
    })?;
    let reconciliation =
        reconcile_normalized(&request.database_path, request, &normalized, &expected).map_err(
            |error| {
                if error.downcast_ref::<ReservedIdentityConflict>().is_some() {
                    conflict(request, error.to_string())
                } else if error
                    .downcast_ref::<identity::DatabaseBindingError>()
                    .is_some()
                {
                    database_replaced(request, error.to_string())
                } else if error.downcast_ref::<backup::MutationBusy>().is_some()
                    || is_sqlite_busy(&error)
                {
                    database_busy(request, error.to_string())
                } else {
                    not_committed(request, error.to_string())
                }
            },
        )?;
    match reconciliation {
        Reconciliation::Committed(receipt) => Ok(CaptureStatus::committed(request, receipt)),
        Reconciliation::NotCommitted => Ok(CaptureStatus::not_committed(request, None)),
    }
}

#[derive(Debug)]
enum CaptureCommit {
    Created(EntryCommit),
    Existing(CommitReceipt),
}

#[derive(Debug)]
enum Reconciliation {
    Committed(CommitReceipt),
    NotCommitted,
}

#[derive(Debug)]
struct ReservedIdentityConflict(String);

impl std::fmt::Display for ReservedIdentityConflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ReservedIdentityConflict {}

fn reconcile_normalized(
    db_path: &std::path::Path,
    request: &CaptureRequest,
    normalized: &NormalizedEntry,
    expected: &db::FileIdentity,
) -> Result<Reconciliation> {
    reconcile_normalized_with_timeout(db_path, request, normalized, expected, None)
}

fn reconcile_normalized_with_timeout(
    db_path: &std::path::Path,
    request: &CaptureRequest,
    normalized: &NormalizedEntry,
    expected: &db::FileIdentity,
    timeout: Option<Duration>,
) -> Result<Reconciliation> {
    identity::validate_database_binding(db_path, expected)?;
    let deadline = timeout.and_then(|value| Instant::now().checked_add(value));
    let connection = match timeout {
        Some(timeout) => db::open_read_only_connection_with_timeout(db_path, timeout)?,
        None => db::open_read_only_connection(db_path)?,
    };
    let _busy_guard = deadline
        .map(|value| db::SqliteDeadline::install(&connection, value))
        .transpose()?;
    let row = connection
        .query_row(
            "SELECT id, created_at, text, text_plain, content_format, title, summary, mood,
                    COALESCE(starred, 0), COALESCE(pinned, 0), COALESCE(hidden, 0)
             FROM entries WHERE uuid = ?1 LIMIT 1",
            [&request.reserved_uuid],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, i64>(8)? != 0,
                    row.get::<_, i64>(9)? != 0,
                    row.get::<_, i64>(10)? != 0,
                ))
            },
        )
        .optional()?;
    let Some((
        entry_id,
        created_at,
        text,
        text_plain,
        format,
        title,
        summary,
        mood,
        starred,
        pinned,
        hidden,
    )) = row
    else {
        return Ok(Reconciliation::NotCommitted);
    };

    let mut stored_tags = Vec::new();
    if let Some(entry_id) = entry_id {
        if table_exists(&connection, "entry_tags")? && table_exists(&connection, "tags")? {
            let mut statement = connection.prepare(
                "SELECT lower(t.name)
             FROM entry_tags et JOIN tags t ON t.id = et.tag_id
             WHERE et.entry_id = ?1 ORDER BY lower(t.name)",
            )?;
            stored_tags = statement
                .query_map([entry_id], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
        }
    }
    let mut requested_tags = normalized.tags.clone();
    requested_tags.sort();

    let stored_parent = if table_exists(&connection, "entry_continuations")? {
        connection
            .query_row(
                "SELECT parent_entry_uuid FROM entry_continuations WHERE child_entry_uuid = ?1",
                [&request.reserved_uuid],
                |row| row.get::<_, String>(0),
            )
            .optional()?
    } else {
        None
    };
    let requested_parent = match normalized.continue_from_uuid.as_deref() {
        Some(parent) => Some(resolve_parent_uuid(&connection, parent)?),
        None => None,
    };

    let fields_match = created_at == normalized.created_at
        && text == normalized.text
        && text_plain == normalized.text_plain
        && format == normalized.content_format
        && title == normalized.title
        && summary == normalized.summary
        && mood == normalized.mood
        && stored_tags == requested_tags
        && starred == normalized.starred
        && pinned == normalized.pinned
        && !hidden
        && stored_parent == requested_parent;
    if !fields_match {
        return Err(anyhow::Error::new(ReservedIdentityConflict(format!(
            "reserved entry UUID {} already belongs to different normalized content",
            request.reserved_uuid
        ))));
    }

    Ok(Reconciliation::Committed(CommitReceipt {
        uuid: request.reserved_uuid.clone(),
        capture_id: request.capture_id.clone(),
        display_number: entry_id,
        saved_at: None,
        backup_path: None,
        backup_operation: None,
        committed: true,
    }))
}

fn resolve_parent_uuid(connection: &Connection, identifier: &str) -> Result<String> {
    connection
        .query_row(
            "SELECT uuid FROM entries WHERE uuid = ?1 OR CAST(id AS TEXT) = ?1 LIMIT 1",
            [identifier],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("continuation parent entry not found: {identifier}"))
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn receipt_from_commit(
    request: &CaptureRequest,
    commit: EntryCommit,
    backup_path: &str,
) -> CommitReceipt {
    CommitReceipt {
        uuid: commit.uuid,
        capture_id: request.capture_id.clone(),
        display_number: Some(commit.entry_id),
        saved_at: Some(commit.committed_at),
        backup_path: Some(PathBuf::from(backup_path)),
        backup_operation: Some("entry.create".to_string()),
        committed: true,
    }
}

fn capture_deadline() -> Instant {
    Instant::now()
        .checked_add(CAPTURE_OPERATION_TIMEOUT)
        .unwrap_or_else(Instant::now)
}

fn remaining_capture_timeout(
    request: &CaptureRequest,
    deadline: Instant,
    phase: &str,
) -> CaptureResult<Duration> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining == Duration::ZERO {
        return Err(database_busy(
            request,
            format!(
                "capture operation exceeded its {} deadline during {phase}",
                capture_timeout_description()
            ),
        ));
    }
    Ok(remaining)
}

fn remaining_capture_timeout_anyhow(deadline: Instant, phase: &str) -> Result<Duration> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining == Duration::ZERO {
        return Err(anyhow::Error::new(backup::MutationBusy(format!(
            "capture operation exceeded its {} deadline during {phase}",
            capture_timeout_description()
        ))));
    }
    Ok(remaining)
}

fn ensure_capture_deadline(
    request: &CaptureRequest,
    deadline: Instant,
    phase: &str,
) -> CaptureResult<()> {
    remaining_capture_timeout(request, deadline, phase).map(|_| ())
}

fn ensure_capture_deadline_anyhow(deadline: Instant, phase: &str) -> Result<()> {
    remaining_capture_timeout_anyhow(deadline, phase).map(|_| ())
}

fn capture_timeout_description() -> String {
    if CAPTURE_OPERATION_TIMEOUT.as_secs() > 0 && CAPTURE_OPERATION_TIMEOUT.subsec_millis() == 0 {
        format!("{} seconds", CAPTURE_OPERATION_TIMEOUT.as_secs())
    } else {
        format!("{} ms", CAPTURE_OPERATION_TIMEOUT.as_millis())
    }
}

fn validate_backup_policy(policy: &BackupPolicy) -> Result<()> {
    if policy.directory.as_os_str().is_empty() || !policy.directory.is_absolute() {
        return Err(anyhow!("backup policy directory must be an absolute path"));
    }
    if policy.retention_count == 0 || policy.retention_count > db::MAX_BACKUP_RETENTION_COUNT {
        return Err(anyhow!(
            "backup retention count must be between 1 and {}",
            db::MAX_BACKUP_RETENTION_COUNT
        ));
    }
    Ok(())
}

fn not_committed(request: &CaptureRequest, message: impl Into<String>) -> CaptureError {
    CaptureError {
        code: CaptureErrorCode::InvalidInput,
        outcome: CaptureOutcome::NotCommitted,
        capture_id: request.capture_id.clone(),
        reserved_uuid: request.reserved_uuid.clone(),
        message: message.into(),
        receipt: None,
    }
}

fn database_replaced(request: &CaptureRequest, message: impl Into<String>) -> CaptureError {
    CaptureError {
        code: CaptureErrorCode::DatabaseReplaced,
        outcome: CaptureOutcome::NotCommitted,
        capture_id: request.capture_id.clone(),
        reserved_uuid: request.reserved_uuid.clone(),
        message: message.into(),
        receipt: None,
    }
}

fn database_busy(request: &CaptureRequest, message: impl Into<String>) -> CaptureError {
    CaptureError {
        code: CaptureErrorCode::DatabaseBusy,
        outcome: CaptureOutcome::NotCommitted,
        capture_id: request.capture_id.clone(),
        reserved_uuid: request.reserved_uuid.clone(),
        message: message.into(),
        receipt: None,
    }
}

fn backup_failed(request: &CaptureRequest, message: impl Into<String>) -> CaptureError {
    CaptureError {
        code: CaptureErrorCode::BackupFailed,
        outcome: CaptureOutcome::NotCommitted,
        capture_id: request.capture_id.clone(),
        reserved_uuid: request.reserved_uuid.clone(),
        message: message.into(),
        receipt: None,
    }
}

fn unknown(request: &CaptureRequest, message: impl Into<String>) -> CaptureError {
    CaptureError {
        code: CaptureErrorCode::CommitUnknown,
        outcome: CaptureOutcome::Unknown,
        capture_id: request.capture_id.clone(),
        reserved_uuid: request.reserved_uuid.clone(),
        message: message.into(),
        receipt: None,
    }
}

fn conflict(request: &CaptureRequest, message: impl Into<String>) -> CaptureError {
    CaptureError {
        code: CaptureErrorCode::IdentityConflict,
        outcome: CaptureOutcome::NotCommitted,
        capture_id: request.capture_id.clone(),
        reserved_uuid: request.reserved_uuid.clone(),
        message: message.into(),
        receipt: None,
    }
}

fn is_sqlite_busy(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<rusqlite::Error>()
            .map(|sqlite_error| match sqlite_error {
                rusqlite::Error::SqliteFailure(code, _) => {
                    matches!(
                        code.extended_code,
                        rusqlite::ffi::SQLITE_BUSY
                            | rusqlite::ffi::SQLITE_LOCKED
                            | rusqlite::ffi::SQLITE_BUSY_SNAPSHOT
                            | rusqlite::ffi::SQLITE_LOCKED_SHAREDCACHE
                    )
                }
                _ => false,
            })
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};
    use rusqlite::Connection;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, CaptureRequest) {
        let root = tempfile::tempdir().unwrap();
        let db_path = root.path().join("capsule.db");
        let backup_path = root.path().join("backups");
        fs::create_dir_all(&backup_path).unwrap();
        Connection::open(&db_path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE entries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT, uuid TEXT UNIQUE,
                    created_at TEXT NOT NULL, updated_at TEXT, text TEXT NOT NULL,
                    text_plain TEXT NOT NULL DEFAULT '', content_format TEXT NOT NULL DEFAULT 'plain',
                    title TEXT, summary TEXT, mood TEXT, starred INTEGER DEFAULT 0,
                    pinned INTEGER DEFAULT 0, hidden INTEGER DEFAULT 0
                 );
                 CREATE TABLE tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE);
                 CREATE TABLE entry_tags (entry_id INTEGER NOT NULL, tag_id INTEGER NOT NULL,
                    PRIMARY KEY(entry_id, tag_id));
                 CREATE TABLE entry_continuations (child_entry_uuid TEXT PRIMARY KEY,
                    parent_entry_uuid TEXT NOT NULL, updated_at TEXT);
                 CREATE VIRTUAL TABLE entries_fts USING fts5(text);",
            )
            .unwrap();
        let identity = identity::freeze_database_identity(&db_path).unwrap();
        let created_at = FixedOffset::east_opt(2 * 60 * 60)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 14, 18, 42, 0)
            .unwrap();
        let request = CaptureRequest::new(
            "A durable note",
            "cap-test-1",
            "entry_reserved_1",
            db_path,
            created_at,
        )
        .with_backup_policy(BackupPolicy::new(backup_path, 5));
        let mut request = request;
        request.database_identity = Some(identity);
        (root, request)
    }

    #[test]
    fn canonical_content_matches_shared_normalization_without_io_or_invocation_identity() {
        let mut request = CaptureRequest::new(
            "  body\r\n  next  ",
            "first",
            "entry_first",
            "missing.db".into(),
            FixedOffset::east_opt(3600)
                .unwrap()
                .with_ymd_and_hms(2026, 9, 14, 12, 0, 0)
                .unwrap(),
        );
        request.title = Some("  title  ".into());
        request.summary = Some("  ".into());
        request.tags = vec![" ÅNGST ".into(), "work".into(), "ångst".into()];
        let normalized = normalize_capture_content(&request).unwrap();
        assert_eq!(normalized.text, "  body\n  next  ");
        assert_eq!(normalized.text_plain, "body next");
        assert_eq!(normalized.title.as_deref(), Some("title"));
        assert!(normalized.summary.is_none());
        assert_eq!(normalized.tags, ["work", "ångst"]);
        request.created_at += chrono::Duration::days(2);
        request.capture_id = "second".into();
        request.reserved_uuid = "entry_second".into();
        request.database_path = "another-missing.db".into();
        request.tags.reverse();
        assert_eq!(normalize_capture_content(&request).unwrap(), normalized);
    }

    #[test]
    fn canonical_serialization_keeps_authored_delimiters_unambiguous() {
        let (_root, mut first) = fixture();
        first.title = Some("alpha\u{1f}beta".into());
        first.summary = Some("gamma".into());
        first.tags = vec!["a,b".into()];
        let mut second = first.clone();
        second.title = Some("alpha".into());
        second.summary = Some("beta\u{1f}gamma".into());
        second.tags = vec!["a".into(), "b".into()];
        assert_ne!(
            serde_json::to_vec(&normalize_capture_content(&first).unwrap()).unwrap(),
            serde_json::to_vec(&normalize_capture_content(&second).unwrap()).unwrap()
        );
    }

    #[test]
    fn capture_returns_receipt_without_post_commit_detail_query() {
        let (_root, request) = fixture();
        let receipt = capture_entry_for_database(request.clone()).unwrap();
        assert!(receipt.committed);
        assert_eq!(receipt.uuid, "entry_reserved_1");
        assert_eq!(receipt.display_number, Some(1));
        assert!(receipt.backup_path.unwrap().exists());
        let connection = Connection::open(&request.database_path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM entries", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn repeated_matching_reserved_uuid_reconciles_without_duplicate() {
        let (_root, request) = fixture();
        let first = capture_entry_for_database(request.clone()).unwrap();
        let second = capture_entry_for_database(request.clone()).unwrap();
        assert_eq!(first.uuid, second.uuid);
        assert_eq!(first.display_number, second.display_number);
        let connection = Connection::open(&request.database_path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM entries", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn read_only_reconciliation_returns_committed_without_invented_time() {
        let (_root, request) = fixture();
        capture_entry_for_database(request.clone()).unwrap();
        let status = reconcile_capture_for_database(&request).unwrap();
        assert_eq!(status.outcome, CaptureOutcome::Committed);
        let receipt = status.receipt.expect("reconciled receipt");
        assert_eq!(receipt.uuid, request.reserved_uuid);
        assert!(receipt.saved_at.is_none());
        assert!(receipt.backup_path.is_none());
    }

    #[test]
    fn read_only_reconciliation_preserves_null_legacy_entry_number() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("legacy.db");
        let backup_path = root.path().join("backups");
        let connection = Connection::open(&db_path).expect("open legacy db");
        connection
            .execute_batch(
                "CREATE TABLE entries (
                    id INTEGER,
                    uuid TEXT PRIMARY KEY,
                    created_at TEXT,
                    updated_at TEXT,
                    text TEXT,
                    text_plain TEXT,
                    content_format TEXT,
                    title TEXT,
                    summary TEXT,
                    mood TEXT,
                    starred INTEGER,
                    pinned INTEGER,
                    hidden INTEGER
                );
                INSERT INTO entries (
                    id, uuid, created_at, updated_at, text, text_plain,
                    content_format, starred, pinned, hidden
                ) VALUES (
                    NULL, 'entry_legacy', '2026-09-14 18:42:00',
                    '2026-09-14 18:42:00', 'A durable note', 'A durable note',
                    'markdown', 0, 0, 0
                );",
            )
            .expect("legacy fixture");
        drop(connection);

        let created_at = FixedOffset::east_opt(2 * 60 * 60)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 14, 18, 42, 0)
            .unwrap();
        let mut request = CaptureRequest::new(
            "A durable note",
            "cap-legacy",
            "entry_legacy",
            db_path.clone(),
            created_at,
        )
        .with_backup_policy(BackupPolicy::new(backup_path, 5));
        request.database_identity = Some(identity::freeze_database_identity(&db_path).unwrap());

        let status = reconcile_capture_for_database(&request).expect("reconcile legacy row");
        assert_eq!(status.outcome, CaptureOutcome::Committed);
        assert_eq!(status.receipt.unwrap().display_number, None);
    }

    #[test]
    fn same_text_with_different_reserved_uuid_survives() {
        let (_root, request) = fixture();
        capture_entry_for_database(request.clone()).unwrap();
        let mut second = request.clone();
        second.reserved_uuid = "entry_reserved_2".to_string();
        second.capture_id = "cap-test-2".to_string();
        capture_entry_for_database(second).unwrap();
        let connection = Connection::open(&request.database_path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM entries", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }

    #[test]
    fn conflicting_reserved_uuid_is_explicit() {
        let (_root, request) = fixture();
        capture_entry_for_database(request.clone()).unwrap();
        let mut conflicting = request;
        conflicting.text = "different".to_string();
        let error = capture_entry_for_database(conflicting).unwrap_err();
        assert_eq!(error.outcome, CaptureOutcome::NotCommitted);
        assert!(error.message.contains("different normalized content"));
    }

    #[test]
    fn after_commit_fault_keeps_confirmed_receipt() {
        let (_root, request) = fixture();
        let hook = |point| {
            if point == CaptureHookPoint::AfterCommit {
                Err(anyhow!("receipt sink failed"))
            } else {
                Ok(())
            }
        };
        let error = capture_entry_with_hooks_for_database(request, &hook).unwrap_err();
        assert_eq!(error.outcome, CaptureOutcome::Committed);
        assert!(error.receipt.is_some());
    }

    #[test]
    fn missing_frozen_policy_is_not_committed() {
        let (_root, mut request) = fixture();
        request.backup_policy = None;
        let error = capture_entry_for_database(request).unwrap_err();
        assert_eq!(error.outcome, CaptureOutcome::NotCommitted);
    }

    #[test]
    fn missing_policy_is_rejected_even_when_row_is_already_committed() {
        let (_root, request) = fixture();
        capture_entry_for_database(request.clone()).expect("initial capture");

        let mut retry = request;
        retry.backup_policy = None;
        let error = capture_entry_for_database(retry).expect_err("policy must be rebound");
        assert_eq!(error.code, CaptureErrorCode::InvalidInput);
        assert_eq!(error.outcome, CaptureOutcome::NotCommitted);
        assert!(error.message.contains("frozen backup policy"));
    }

    #[test]
    fn fault_checkpoints_before_commit_leave_no_entry() {
        let points = [
            CaptureHookPoint::BeforeBackup,
            CaptureHookPoint::AfterBackup,
            CaptureHookPoint::BeforeBegin,
            CaptureHookPoint::BeforeInsert,
            CaptureHookPoint::BeforeFts,
            CaptureHookPoint::BeforeResequence,
            CaptureHookPoint::BeforeCommit,
        ];

        for point in points {
            let (_root, request) = fixture();
            let hook = move |candidate| {
                if candidate == point {
                    Err(anyhow!("injected capture fault at {point:?}"))
                } else {
                    Ok(())
                }
            };
            let error = capture_entry_with_hooks_for_database(request.clone(), &hook)
                .expect_err("fault must stop before a confirmed commit");
            assert_eq!(error.outcome, CaptureOutcome::NotCommitted);
            let connection = Connection::open(&request.database_path).unwrap();
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM entries", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap(),
                0,
                "fault at {point:?} left a row behind"
            );
            let backup_dir = request.backup_policy.as_ref().unwrap().directory.clone();
            let leftovers = fs::read_dir(backup_dir)
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    name.contains(".tmp-") || name.ends_with(".reserve")
                })
                .count();
            assert_eq!(leftovers, 0, "fault at {point:?} leaked a reservation");
        }
    }

    #[test]
    fn commit_boundary_fault_is_unknown_until_reconciled() {
        let (_root, request) = fixture();
        let hook = |point| {
            if point == CaptureHookPoint::DuringCommit {
                Err(anyhow!("simulated interruption at commit boundary"))
            } else {
                Ok(())
            }
        };
        let error = capture_entry_with_hooks_for_database(request.clone(), &hook)
            .expect_err("commit boundary interruption");
        assert_eq!(error.code, CaptureErrorCode::CommitUnknown);
        assert_eq!(error.outcome, CaptureOutcome::Unknown);
        let status = reconcile_capture_for_database(&request).expect("reconcile unknown");
        assert_eq!(status.outcome, CaptureOutcome::NotCommitted);
    }

    #[test]
    fn replacement_at_same_path_is_rejected_before_backup_or_write() {
        let (root, request) = fixture();
        let replacement = root.path().join("replacement.db");
        fs::copy(&request.database_path, &replacement).unwrap();
        fs::remove_file(&request.database_path).unwrap();
        fs::rename(replacement, &request.database_path).unwrap();

        let error = capture_entry_for_database(request.clone()).expect_err("replacement guard");
        assert_eq!(error.code, CaptureErrorCode::DatabaseReplaced);
        assert_eq!(error.outcome, CaptureOutcome::NotCommitted);
        let connection = Connection::open(&request.database_path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM entries", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
    }

    #[test]
    fn concurrent_captures_keep_distinct_entries_tags_and_fts() {
        use std::sync::{Arc, Barrier};

        let (_root, request) = fixture();
        let barrier = Arc::new(Barrier::new(4));
        let mut handles = Vec::new();
        for index in 0..4 {
            let mut request = request.clone();
            request.capture_id = format!("cap-concurrent-{index}");
            request.reserved_uuid = format!("entry_concurrent_{index}");
            request.text = format!("concurrent note {index}");
            request.tags = vec![format!("tag-{index}")];
            let barrier = barrier.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                capture_entry_for_database(request)
            }));
        }

        for handle in handles {
            handle.join().unwrap().expect("concurrent capture");
        }

        let connection = Connection::open(&request.database_path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM entries", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            4
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM entries_fts", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            4
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM entry_tags", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            4
        );
    }
}
