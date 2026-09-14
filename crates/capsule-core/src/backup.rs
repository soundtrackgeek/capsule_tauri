use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process, thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::{
    backup::{Backup, StepResult},
    Connection,
};

use crate::{
    db,
    models::{
        BackupCreateRequest, BackupCreateResponse, BackupInfo, BackupListResponse, BackupManifest,
        BackupRestorePreview, BackupRestorePreviewRequest, BackupRestoreRequest,
        BackupRestoreResponse, MutationAudit,
    },
};

const APP_NAME: &str = "capsule-tauri";
const BACKUP_PREFIX: &str = "capsule_backup_";
const BACKUP_DB_EXTENSION: &str = ".db";
const BACKUP_JSON_EXTENSION: &str = "json";
const MUTATION_LOCK_WAIT: Duration = Duration::from_secs(15);
const MUTATION_LOCK_POLL: Duration = Duration::from_millis(20);

#[derive(Debug)]
pub struct MutationBusy(pub String);

impl std::fmt::Display for MutationBusy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for MutationBusy {}

pub fn list_backups() -> Result<BackupListResponse> {
    list_backups_for_database(&db::resolve_database_path())
}

pub fn list_backups_for_database(db_path: &Path) -> Result<BackupListResponse> {
    let backup_directory = db::backup_directory_for_database(db_path);
    let backups = backup_infos_in_directory(&backup_directory)?;

    Ok(BackupListResponse {
        backups,
        backup_directory: db::path_to_string(&backup_directory),
    })
}

pub fn create_backup(input: BackupCreateRequest) -> Result<BackupCreateResponse> {
    let db_path = db::resolve_database_path();
    create_backup_for_database(&db_path, input)
}

pub fn create_backup_for_database(
    db_path: &Path,
    input: BackupCreateRequest,
) -> Result<BackupCreateResponse> {
    let _lock = acquire_mutation_lock(db_path)?;
    create_backup_for_database_unlocked(db_path, input)
}

fn create_backup_for_database_unlocked(
    db_path: &Path,
    input: BackupCreateRequest,
) -> Result<BackupCreateResponse> {
    let policy = crate::contracts::BackupPolicy::new(
        db::backup_directory_for_database(db_path),
        db::backup_retention_count_for_database(db_path),
    );
    with_backup_directory_lock(&policy, || {
        create_backup_with_policy_locked(db_path, input, &policy)
    })
}

fn create_backup_with_policy_locked(
    db_path: &Path,
    input: BackupCreateRequest,
    policy: &crate::contracts::BackupPolicy,
) -> Result<BackupCreateResponse> {
    create_backup_with_policy_locked_until(db_path, input, policy, None)
}

/// Create and publish a verified backup, optionally bounded by a caller's
/// absolute deadline.  The default APIs retain their historical unbounded
/// backup stepping; capture supplies a deadline so a busy source cannot leave
/// the database sidecar held forever waiting on SQLite.
fn create_backup_with_policy_locked_until(
    db_path: &Path,
    input: BackupCreateRequest,
    policy: &crate::contracts::BackupPolicy,
    deadline: Option<(Instant, Duration)>,
) -> Result<BackupCreateResponse> {
    validate_backup_policy(policy)?;
    check_backup_deadline(deadline, "backup preparation")?;
    let metadata = fs::metadata(db_path)
        .with_context(|| format!("database does not exist: {}", db_path.display()))?;
    let backup_directory = policy.directory.as_path();
    fs::create_dir_all(backup_directory)
        .with_context(|| format!("failed to create {}", backup_directory.display()))?;
    check_backup_deadline(deadline, "backup reservation")?;

    let now = Utc::now();
    let reservation = reserve_backup_path(backup_directory, now)?;
    let backup_path = reservation.final_path.clone();
    let temporary_path = reservation.temporary_path.clone();
    let manifest_path = backup_path.with_extension(BACKUP_JSON_EXTENSION);

    let result = (|| -> Result<BackupCreateResponse> {
        let source = match deadline {
            Some((deadline, _)) => db::open_read_only_connection_with_timeout(
                db_path,
                remaining_backup_deadline(deadline),
            )?,
            None => db::open_read_only_connection(db_path)?,
        };
        let mut destination = Connection::open(&temporary_path)
            .with_context(|| format!("failed to create {}", temporary_path.display()))?;
        if let Some((deadline, timeout)) = deadline {
            check_backup_deadline(Some((deadline, timeout)), "backup connection setup")?;
            // Keep each SQLite backup step short so the loop can re-check the
            // absolute deadline instead of allowing a single busy handler to
            // consume the whole remaining budget after prior work elapsed.
            source.busy_timeout(MUTATION_LOCK_POLL)?;
            destination.busy_timeout(MUTATION_LOCK_POLL)?;
        }
        let backup = Backup::new(&source, &mut destination)?;
        match deadline {
            Some((deadline, timeout)) => {
                run_backup_to_completion_until(&backup, deadline, timeout)?;
            }
            None => backup.run_to_completion(128, Duration::from_millis(20), None)?,
        }
        drop(backup);
        drop(destination);
        drop(source);

        match deadline {
            Some((deadline, timeout)) => {
                verify_backup_until(&temporary_path, deadline, timeout)?;
            }
            None => verify_backup(&temporary_path)?,
        }
        check_backup_deadline(deadline, "backup publication")?;
        fs::rename(&temporary_path, &backup_path).with_context(|| {
            format!(
                "failed to publish verified backup {}",
                backup_path.display()
            )
        })?;
        remove_if_exists(&reservation.marker_path)?;
        check_backup_deadline(deadline, "backup manifest")?;

        let operation = input.operation.unwrap_or_else(|| "manual".to_string());
        let manifest = BackupManifest {
            created_at: now.to_rfc3339(),
            operation,
            app: APP_NAME.to_string(),
            db_path: db::path_to_string(db_path),
            db_size_bytes: metadata.len(),
            backup_path: db::path_to_string(&backup_path),
        };
        write_manifest_atomically(&manifest_path, &manifest)?;
        check_backup_deadline(deadline, "backup retention")?;

        let mut backup_info = backup_info_from_path(&backup_path)?;
        backup_info.manifest_path = Some(db::path_to_string(&manifest_path));
        backup_info.operation = Some(manifest.operation);
        backup_info.created_at = Some(manifest.created_at);
        backup_info.verified = true;

        apply_backup_retention(backup_directory, policy.retention_count)?;
        check_backup_deadline(deadline, "backup completion")?;

        Ok(BackupCreateResponse {
            backup: backup_info,
        })
    })();

    if result.is_err() {
        let _ = remove_if_exists(&temporary_path);
        let _ = remove_if_exists(&backup_path);
        let _ = remove_if_exists(&reservation.marker_path);
        let _ = remove_if_exists(&manifest_path);
    }
    result
}

pub fn preview_restore_backup(input: BackupRestorePreviewRequest) -> Result<BackupRestorePreview> {
    let db_path = db::resolve_database_path();
    preview_restore_backup_for_database(&db_path, input)
}

pub fn preview_restore_backup_for_database(
    db_path: &Path,
    input: BackupRestorePreviewRequest,
) -> Result<BackupRestorePreview> {
    let backup_path = validate_restore_backup_path(db_path, &input.backup_path)?;
    verify_backup(&backup_path)?;

    let metadata = fs::metadata(&backup_path)
        .with_context(|| format!("failed to inspect {}", backup_path.display()))?;
    let db_modified_at = metadata.modified().ok().map(db::system_time_to_iso);
    let connection = db::open_read_only_connection(&backup_path)?;
    let schema_summary = db::inspect_schema(&connection)?;
    let entry_count = if schema_summary.has_entries_table {
        Some(count_table_rows(&connection, "entries")?)
    } else {
        None
    };
    let tag_count = if schema_summary.has_tags_table {
        Some(count_table_rows(&connection, "tags")?)
    } else {
        None
    };

    let mut warnings = Vec::new();
    if !schema_summary.missing_core_tables.is_empty() {
        warnings.push(format!(
            "Missing core tables: {}",
            schema_summary.missing_core_tables.join(", ")
        ));
    }

    Ok(BackupRestorePreview {
        backup: backup_info_from_path(&backup_path)?,
        db_size_bytes: metadata.len(),
        db_modified_at,
        schema_summary,
        entry_count,
        tag_count,
        warnings,
    })
}

pub fn restore_backup(input: BackupRestoreRequest) -> Result<BackupRestoreResponse> {
    let db_path = db::resolve_database_path();
    restore_backup_for_database(&db_path, input)
}

pub fn restore_backup_for_database(
    db_path: &Path,
    input: BackupRestoreRequest,
) -> Result<BackupRestoreResponse> {
    let _lock = acquire_mutation_lock(db_path)?;
    restore_backup_for_database_unlocked(db_path, input)
}

fn restore_backup_for_database_unlocked(
    db_path: &Path,
    input: BackupRestoreRequest,
) -> Result<BackupRestoreResponse> {
    if input.confirmation.as_deref() != Some("RESTORE") {
        return Err(anyhow!("Restore confirmation must be RESTORE."));
    }

    let original_identity = db::FileIdentity::for_path(db_path);
    let backup_path = validate_restore_backup_path(db_path, &input.backup_path)?;
    verify_backup(&backup_path)?;
    let restored_from = backup_info_from_path(&backup_path)?;

    let safety_backup = create_backup_for_database_unlocked(
        db_path,
        BackupCreateRequest {
            operation: Some("backup.restore.safety".to_string()),
        },
    )
    .context("failed to create a safety backup before restore")?
    .backup;

    crate::identity::validate_database_binding(db_path, &original_identity)
        .context("active database changed while preparing restore")?;

    let backup_directory = db::backup_directory_for_database(db_path);
    let temp_restore_path = backup_directory.join("capsule_restore_pending.db");
    if temp_restore_path.exists() {
        fs::remove_file(&temp_restore_path).with_context(|| {
            format!(
                "failed to remove stale restore file {}",
                temp_restore_path.display()
            )
        })?;
    }

    fs::copy(&backup_path, &temp_restore_path).with_context(|| {
        format!(
            "failed to stage restore file {}",
            temp_restore_path.display()
        )
    })?;
    verify_backup(&temp_restore_path)?;

    checkpoint_database(db_path);
    remove_if_exists(&sidecar_path(db_path, "-wal"))?;
    remove_if_exists(&sidecar_path(db_path, "-shm"))?;
    remove_if_exists(db_path)?;
    fs::rename(&temp_restore_path, db_path).with_context(|| {
        format!(
            "failed to replace {} with {}",
            db_path.display(),
            temp_restore_path.display()
        )
    })?;

    verify_backup(db_path)?;
    let status = db::database_status_for_path(db_path.to_path_buf())?;

    Ok(BackupRestoreResponse {
        restored_from,
        safety_backup,
        completed_at: Utc::now().to_rfc3339(),
        status,
    })
}

#[derive(Debug, Clone)]
pub struct GuardedWrite<T> {
    pub value: T,
    pub audit: MutationAudit,
}

pub fn with_database_backup<T>(
    operation: &str,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<GuardedWrite<T>> {
    let db_path = db::resolve_database_path();
    with_database_backup_for_database(&db_path, operation, write_fn)
}

/// Run a caller-owned database mutation while holding Capsule's cross-process
/// lock.  This intentionally does not create a backup or read path settings;
/// context persistence and other narrowly scoped callers can use it when
/// their own already-verified snapshot/transaction boundary must include
/// identity validation and the `BEGIN IMMEDIATE` write.
pub fn with_mutation_lock_for_database<T>(
    db_path: &Path,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<T> {
    let _lock = acquire_mutation_lock(db_path)?;
    write_fn(db_path)
}

/// Run a caller-owned database mutation while holding Capsule's cross-process
/// lock, bounding lock acquisition by `timeout`.  The timeout applies only to
/// acquiring the lock; the caller remains responsible for checking any wider
/// operation deadline before and after its write closure.
pub fn with_mutation_lock_for_database_with_timeout<T>(
    db_path: &Path,
    timeout: Duration,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<T> {
    let _lock = acquire_mutation_lock_with_timeout(db_path, timeout)?;
    write_fn(db_path)
}

pub fn with_database_backup_for_database<T>(
    db_path: &Path,
    operation: &str,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<GuardedWrite<T>> {
    let _lock = acquire_mutation_lock(db_path)?;
    let policy = crate::contracts::BackupPolicy::new(
        db::backup_directory_for_database(db_path),
        db::backup_retention_count_for_database(db_path),
    );
    validate_backup_policy(&policy)?;
    fs::create_dir_all(&policy.directory)
        .with_context(|| format!("failed to create {}", policy.directory.display()))?;
    let _backup_lock = acquire_backup_directory_lock(&policy.directory)?;
    let backup = create_backup_with_policy_locked(
        db_path,
        BackupCreateRequest {
            operation: Some(operation.to_string()),
        },
        &policy,
    )
    .with_context(|| format!("backup failed before {operation}"))?;
    let backup_path = backup.backup.path;
    let value = write_fn(db_path)?;

    Ok(GuardedWrite {
        value,
        audit: MutationAudit {
            backup_path,
            operation: operation.to_string(),
            completed_at: Utc::now().to_rfc3339(),
        },
    })
}

/// Backup-guarded mutation using a policy captured by the caller.  This is
/// the headless boundary: no environment or path-settings lookup occurs after
/// the policy is supplied.
pub fn with_database_backup_for_database_using_policy<T>(
    db_path: &Path,
    operation: &str,
    policy: &crate::contracts::BackupPolicy,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<GuardedWrite<T>> {
    validate_backup_policy(policy)?;
    let _lock = acquire_mutation_lock(db_path)?;
    // Keep the backup-directory lock through the guarded write as well as
    // backup creation.  Distinct databases may intentionally share one
    // directory; retaining this lock prevents either process from pruning or
    // replacing the other's just-published snapshot before its audit is
    // returned.
    fs::create_dir_all(&policy.directory)
        .with_context(|| format!("failed to create {}", policy.directory.display()))?;
    let _backup_lock = acquire_backup_directory_lock(&policy.directory)?;
    let backup = create_backup_with_policy_locked(
        db_path,
        BackupCreateRequest {
            operation: Some(operation.to_string()),
        },
        policy,
    )
    .with_context(|| format!("backup failed before {operation}"))?;
    let backup_path = backup.backup.path;
    let value = write_fn(db_path)?;

    Ok(GuardedWrite {
        value,
        audit: MutationAudit {
            backup_path,
            operation: operation.to_string(),
            completed_at: Utc::now().to_rfc3339(),
        },
    })
}

/// Backup-guarded mutation using a caller-captured policy and a bounded lock
/// acquisition budget.  The budget is shared by the database and backup
/// directory locks, so a context worker can pass its remaining coordination
/// deadline without accidentally waiting once per lock.
pub fn with_database_backup_for_database_using_policy_with_timeout<T>(
    db_path: &Path,
    operation: &str,
    timeout: Duration,
    policy: &crate::contracts::BackupPolicy,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<GuardedWrite<T>> {
    with_database_backup_for_database_using_policy_with_timeout_and_preflight(
        db_path,
        operation,
        timeout,
        policy,
        |_| Ok(()),
        write_fn,
    )
}

/// Backup-guarded mutation with one bounded operation budget and a callback
/// that runs after both cross-process locks are acquired but before any
/// snapshot is created.  Capture uses this boundary to revalidate the frozen
/// database identity after lock contention, preventing a replaced database
/// from being backed up before the write is rejected.
pub fn with_database_backup_for_database_using_policy_with_timeout_and_preflight<T>(
    db_path: &Path,
    operation: &str,
    timeout: Duration,
    policy: &crate::contracts::BackupPolicy,
    pre_backup_fn: impl FnOnce(&Path) -> Result<()>,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<GuardedWrite<T>> {
    validate_backup_policy(policy)?;
    let deadline = lock_deadline(timeout);
    let _lock = acquire_mutation_lock_until(db_path, deadline, timeout)?;
    // Keep the backup-directory lock through the guarded write as well as
    // backup creation, just like the unbounded-budget policy helper.
    fs::create_dir_all(&policy.directory)
        .with_context(|| format!("failed to create {}", policy.directory.display()))?;
    let _backup_lock = acquire_backup_directory_lock_until(&policy.directory, deadline, timeout)?;
    check_backup_deadline(Some((deadline, timeout)), "pre-backup validation")?;
    pre_backup_fn(db_path)?;
    check_backup_deadline(Some((deadline, timeout)), "backup start")?;
    let backup = create_backup_with_policy_locked_until(
        db_path,
        BackupCreateRequest {
            operation: Some(operation.to_string()),
        },
        policy,
        Some((deadline, timeout)),
    )
    .with_context(|| format!("backup failed before {operation}"))?;
    let backup_path = backup.backup.path;
    check_backup_deadline(Some((deadline, timeout)), "guarded mutation start")?;
    let value = write_fn(db_path)?;

    Ok(GuardedWrite {
        value,
        audit: MutationAudit {
            backup_path,
            operation: operation.to_string(),
            completed_at: Utc::now().to_rfc3339(),
        },
    })
}

pub fn backup_filename_for(timestamp: DateTime<Utc>) -> String {
    format!(
        "{BACKUP_PREFIX}{}{BACKUP_DB_EXTENSION}",
        timestamp.format("%Y%m%d_%H%M%S")
    )
}

#[derive(Debug, Clone)]
struct BackupPathReservation {
    final_path: PathBuf,
    temporary_path: PathBuf,
    marker_path: PathBuf,
}

fn reserve_backup_path(
    directory: &Path,
    timestamp: DateTime<Utc>,
) -> Result<BackupPathReservation> {
    // Names remain Capsule-compatible (second precision), while create_new
    // makes the reservation itself atomic if another process is already
    // producing a backup in this directory.
    for offset in 0..86_400_i64 {
        let candidate = directory.join(backup_filename_for(
            timestamp + chrono::Duration::seconds(offset),
        ));
        // A published backup owns its second-precision name permanently.
        // Check the final path before creating a reservation marker; without
        // this guard a later call could reserve the same name and overwrite
        // the existing snapshot when running on platforms where rename()
        // replaces its destination.
        let manifest = candidate.with_extension(BACKUP_JSON_EXTENSION);
        if candidate.exists() || manifest.exists() {
            continue;
        }
        let marker = PathBuf::from(format!("{}.reserve", candidate.to_string_lossy()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
        {
            Ok(_) => {
                let file_name = candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| anyhow!("backup path is not valid UTF-8"))?;
                let temporary = directory.join(format!(".{file_name}.tmp-{}", unique_token()));
                match OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)
                {
                    Ok(_) => {
                        return Ok(BackupPathReservation {
                            final_path: candidate,
                            temporary_path: temporary,
                            marker_path: marker,
                        })
                    }
                    Err(error) => {
                        let _ = remove_if_exists(&marker);
                        if error.kind() == ErrorKind::AlreadyExists {
                            continue;
                        }
                        return Err(error).with_context(|| {
                            format!(
                                "failed to reserve temporary backup path {}",
                                temporary.display()
                            )
                        });
                    }
                }
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to reserve backup path {}", candidate.display())
                })
            }
        }
    }

    Err(anyhow!(
        "unable to reserve a unique Capsule backup name in {}",
        directory.display()
    ))
}

fn with_backup_directory_lock<T>(
    policy: &crate::contracts::BackupPolicy,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    validate_backup_policy(policy)?;
    fs::create_dir_all(&policy.directory)
        .with_context(|| format!("failed to create {}", policy.directory.display()))?;
    let _lock = acquire_backup_directory_lock(&policy.directory)?;
    operation()
}

fn apply_backup_retention(backup_directory: &Path, retention_count: usize) -> Result<()> {
    let retention_count = retention_count.clamp(1, db::MAX_BACKUP_RETENTION_COUNT);
    let backups = backup_infos_in_directory(backup_directory)?;
    if backups.len() <= retention_count {
        return Ok(());
    }

    for backup in backups.into_iter().skip(retention_count) {
        let backup_path = PathBuf::from(&backup.path);
        let manifest_path = backup
            .manifest_path
            .map(PathBuf::from)
            .unwrap_or_else(|| backup_path.with_extension(BACKUP_JSON_EXTENSION));
        remove_if_exists(&backup_path)?;
        remove_if_exists(&manifest_path)?;
    }

    Ok(())
}

fn backup_infos_in_directory(backup_directory: &Path) -> Result<Vec<BackupInfo>> {
    let mut backups = Vec::new();

    if backup_directory.exists() {
        for entry in fs::read_dir(backup_directory)
            .with_context(|| format!("failed to read {}", backup_directory.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !is_capsule_backup_db(&path) {
                continue;
            }

            backups.push(backup_info_from_path(&path)?);
        }
    }

    backups.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.path.cmp(&left.path))
    });

    Ok(backups)
}

fn verify_backup(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("backup was not created: {}", path.display()))?;
    if metadata.len() == 0 {
        return Err(anyhow!("backup is empty: {}", path.display()));
    }

    let connection = db::open_read_only_connection(path)?;
    let schema = db::inspect_schema(&connection)?;
    if !schema.has_entries_table {
        return Err(anyhow!(
            "backup verification failed because entries table was not found"
        ));
    }

    let integrity =
        connection.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))?;
    if !integrity.eq_ignore_ascii_case("ok") {
        return Err(anyhow!(
            "backup verification failed integrity_check: {integrity}"
        ));
    }

    let mut foreign_keys = connection.prepare("PRAGMA foreign_key_check")?;
    let mut rows = foreign_keys.query([])?;
    if rows.next()?.is_some() {
        return Err(anyhow!(
            "backup verification failed because foreign-key violations were found"
        ));
    }

    Ok(())
}

fn verify_backup_until(path: &Path, deadline: Instant, timeout: Duration) -> Result<()> {
    check_backup_deadline(Some((deadline, timeout)), "backup verification")?;
    let metadata = fs::metadata(path)
        .with_context(|| format!("backup was not created: {}", path.display()))?;
    if metadata.len() == 0 {
        return Err(anyhow!("backup is empty: {}", path.display()));
    }

    let connection =
        db::open_read_only_connection_with_timeout(path, remaining_backup_deadline(deadline))?;
    check_backup_deadline(Some((deadline, timeout)), "backup schema verification")?;
    let schema = db::inspect_schema(&connection)?;
    if !schema.has_entries_table {
        return Err(anyhow!(
            "backup verification failed because entries table was not found"
        ));
    }

    check_backup_deadline(Some((deadline, timeout)), "backup integrity verification")?;
    connection.busy_timeout(remaining_backup_deadline(deadline))?;
    let integrity =
        connection.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))?;
    if !integrity.eq_ignore_ascii_case("ok") {
        return Err(anyhow!(
            "backup verification failed integrity_check: {integrity}"
        ));
    }

    check_backup_deadline(Some((deadline, timeout)), "backup foreign-key verification")?;
    connection.busy_timeout(remaining_backup_deadline(deadline))?;
    let mut foreign_keys = connection.prepare("PRAGMA foreign_key_check")?;
    let mut rows = foreign_keys.query([])?;
    if rows.next()?.is_some() {
        return Err(anyhow!(
            "backup verification failed because foreign-key violations were found"
        ));
    }

    Ok(())
}

fn run_backup_to_completion_until(
    backup: &Backup<'_, '_>,
    deadline: Instant,
    timeout: Duration,
) -> Result<()> {
    loop {
        check_backup_deadline(Some((deadline, timeout)), "backup stepping")?;
        match backup.step(128)? {
            StepResult::Done => return Ok(()),
            StepResult::More | StepResult::Busy | StepResult::Locked => {
                check_backup_deadline(Some((deadline, timeout)), "backup stepping")?;
                let remaining = remaining_backup_deadline(deadline);
                thread::sleep(MUTATION_LOCK_POLL.min(remaining));
            }
            _ => return Err(anyhow!("backup returned an unsupported step result")),
        }
    }
}

fn remaining_backup_deadline(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
}

fn check_backup_deadline(deadline: Option<(Instant, Duration)>, phase: &str) -> Result<()> {
    if let Some((deadline, timeout)) = deadline {
        if Instant::now() >= deadline {
            return Err(anyhow::Error::new(MutationBusy(format!(
                "backup operation exceeded its {} deadline during {phase}",
                lock_timeout_description(timeout)
            ))));
        }
    }
    Ok(())
}

fn write_manifest_atomically(path: &Path, manifest: &BackupManifest) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("manifest path has no parent: {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("manifest path is not valid UTF-8: {}", path.display()))?;
    let temporary = parent.join(format!(".{file_name}.tmp-{}", unique_token()));
    let bytes = serde_json::to_vec_pretty(manifest)?;
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("failed to create {}", temporary.display()))?;
        file.write_all(&bytes)
            .with_context(|| format!("failed to write {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to flush {}", temporary.display()))?;
        drop(file);
        fs::rename(&temporary, path)
            .with_context(|| format!("failed to publish {}", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = remove_if_exists(&temporary);
    }
    result
}

fn unique_token() -> String {
    let nanos = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000);
    format!("{}-{nanos}", process::id())
}

fn validate_backup_policy(policy: &crate::contracts::BackupPolicy) -> Result<()> {
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

struct MutationLock {
    file: Option<File>,
}

impl Drop for MutationLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

/// Serialize backup + mutation + retention/restore operations across
/// processes.  SQLite WAL serializes writers only after the backup phase; this
/// lock closes that pre-backup race while retaining the existing 15-second
/// bounded wait policy.
fn acquire_mutation_lock(db_path: &Path) -> Result<MutationLock> {
    acquire_mutation_lock_with_timeout(db_path, MUTATION_LOCK_WAIT)
}

fn acquire_mutation_lock_with_timeout(db_path: &Path, timeout: Duration) -> Result<MutationLock> {
    let deadline = lock_deadline(timeout);
    acquire_mutation_lock_until(db_path, deadline, timeout)
}

fn acquire_mutation_lock_until(
    db_path: &Path,
    deadline: Instant,
    timeout: Duration,
) -> Result<MutationLock> {
    let canonical = fs::canonicalize(db_path).unwrap_or_else(|_| db_path.to_path_buf());
    let parent = canonical
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    let path = PathBuf::from(format!("{}.capsule-lock", canonical.to_string_lossy()));
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("failed to open mutation lock {}", path.display()))?;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(MutationLock { file: Some(file) }),
            Err(std::fs::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::Error::new(MutationBusy(format!(
                        "database mutation is busy (lock held for more than {})",
                        lock_timeout_description(timeout)
                    ))));
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                thread::sleep(MUTATION_LOCK_POLL.min(remaining));
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to acquire database mutation lock {}",
                        path.display()
                    )
                })
            }
        }
    }
}

/// Serialize publication and retention when independent databases share a
/// backup directory.  The sidecar is deliberately persistent; the OS lock is
/// the ownership mechanism, so a process crash cannot leave an orphaned lock
/// state that blocks future work.
fn acquire_backup_directory_lock(directory: &Path) -> Result<MutationLock> {
    acquire_backup_directory_lock_with_timeout(directory, MUTATION_LOCK_WAIT)
}

fn acquire_backup_directory_lock_with_timeout(
    directory: &Path,
    timeout: Duration,
) -> Result<MutationLock> {
    let deadline = lock_deadline(timeout);
    acquire_backup_directory_lock_until(directory, deadline, timeout)
}

fn acquire_backup_directory_lock_until(
    directory: &Path,
    deadline: Instant,
    timeout: Duration,
) -> Result<MutationLock> {
    let canonical = fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf());
    let path = PathBuf::from(format!(
        "{}.capsule-backup-lock",
        canonical.to_string_lossy()
    ));
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("failed to open backup directory lock {}", path.display()))?;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(MutationLock { file: Some(file) }),
            Err(std::fs::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(anyhow::Error::new(MutationBusy(format!(
                        "backup directory is busy (lock held for more than {})",
                        lock_timeout_description(timeout)
                    ))));
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                thread::sleep(MUTATION_LOCK_POLL.min(remaining));
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(error).with_context(|| {
                    format!("failed to acquire backup directory lock {}", path.display())
                })
            }
        }
    }
}

fn lock_deadline(timeout: Duration) -> Instant {
    Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now)
}

fn lock_timeout_description(timeout: Duration) -> String {
    if timeout.as_secs() > 0 && timeout.subsec_millis() == 0 {
        format!("{} seconds", timeout.as_secs())
    } else {
        format!("{} ms", timeout.as_millis())
    }
}

fn backup_info_from_path(path: &Path) -> Result<BackupInfo> {
    let metadata = fs::metadata(path)?;
    let manifest_path = path.with_extension(BACKUP_JSON_EXTENSION);
    let manifest = read_manifest(&manifest_path).ok();
    let created_at = manifest
        .as_ref()
        .map(|item| item.created_at.clone())
        .or_else(|| parse_created_at_from_filename(path));
    let operation = manifest.as_ref().map(|item| item.operation.clone());

    Ok(BackupInfo {
        path: db::path_to_string(path),
        manifest_path: manifest_path
            .exists()
            .then(|| db::path_to_string(&manifest_path)),
        created_at,
        size_bytes: metadata.len(),
        operation,
        verified: metadata.len() > 0,
    })
}

fn read_manifest(path: &Path) -> Result<BackupManifest> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn is_capsule_backup_db(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            name.starts_with(BACKUP_PREFIX)
                && name.ends_with(BACKUP_DB_EXTENSION)
                && name.len()
                    == BACKUP_PREFIX.len() + "YYYYMMDD_HHMMSS".len() + BACKUP_DB_EXTENSION.len()
        })
        .unwrap_or(false)
}

fn parse_created_at_from_filename(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let stamp = file_name
        .strip_prefix(BACKUP_PREFIX)?
        .strip_suffix(BACKUP_DB_EXTENSION)?;
    let naive = NaiveDateTime::parse_from_str(stamp, "%Y%m%d_%H%M%S").ok()?;
    Some(Utc.from_utc_datetime(&naive).to_rfc3339())
}

fn validate_restore_backup_path(db_path: &Path, backup_path: &str) -> Result<PathBuf> {
    let backup_path = PathBuf::from(backup_path);
    if !is_capsule_backup_db(&backup_path) {
        return Err(anyhow!(
            "Restore only accepts Capsule backup files named capsule_backup_YYYYMMDD_HHMMSS.db."
        ));
    }

    let expected_directory = db::backup_directory_for_database(db_path)
        .canonicalize()
        .with_context(|| {
            format!(
                "failed to resolve backup directory for {}",
                db_path.display()
            )
        })?;
    let backup_path = backup_path
        .canonicalize()
        .with_context(|| format!("backup file does not exist: {}", backup_path.display()))?;
    let backup_parent = backup_path
        .parent()
        .context("backup path does not have a parent directory")?
        .canonicalize()?;

    if backup_parent != expected_directory {
        return Err(anyhow!(
            "Restore only accepts backups from the active database backup directory."
        ));
    }

    Ok(backup_path)
}

fn count_table_rows(connection: &Connection, table: &str) -> Result<i64> {
    let table = match table {
        "entries" => "entries",
        "tags" => "tags",
        other => return Err(anyhow!("unsupported table count: {other}")),
    };
    let sql = format!("SELECT COUNT(*) FROM {table}");
    Ok(connection.query_row(&sql, [], |row| row.get::<_, i64>(0))?)
}

fn checkpoint_database(path: &Path) {
    if let Ok(connection) = db::open_read_write_connection(path) {
        let _ = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{}", path.to_string_lossy(), suffix))
}

fn remove_if_exists(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn backup_filename_uses_capsule_compatible_timestamp() {
        let timestamp = Utc
            .with_ymd_and_hms(2026, 6, 29, 12, 0, 0)
            .single()
            .expect("timestamp");

        assert_eq!(
            backup_filename_for(timestamp),
            "capsule_backup_20260629_120000.db"
        );
    }

    #[test]
    fn list_backups_filters_to_capsule_backup_pattern() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        fs::write(
            temp_dir.path().join("capsule_backup_20260629_120000.db"),
            b"db",
        )
        .expect("backup");
        fs::write(
            temp_dir.path().join("capsule_backup_20260629_120000.bak"),
            b"bak",
        )
        .expect("ignored bak");
        fs::write(temp_dir.path().join("custom_backup.db"), b"custom").expect("ignored custom");

        let response = list_backups_for_database(&db_path).expect("list");

        assert_eq!(response.backups.len(), 1);
        assert!(response.backups[0]
            .path
            .ends_with("capsule_backup_20260629_120000.db"));
    }

    #[test]
    fn create_backup_writes_verified_database_and_manifest() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT);
                INSERT INTO entries (text) VALUES ('safe');
                ",
            )
            .expect("fixture");
        drop(connection);

        let response = create_backup_for_database(
            &db_path,
            BackupCreateRequest {
                operation: Some("test.backup".to_string()),
            },
        )
        .expect("backup response");
        let backup_path = PathBuf::from(&response.backup.path);
        let manifest_path = PathBuf::from(response.backup.manifest_path.expect("manifest path"));

        assert!(backup_path.exists());
        assert!(manifest_path.exists());
        assert!(response.backup.verified);
        assert_eq!(response.backup.operation.as_deref(), Some("test.backup"));

        let backup_connection = Connection::open(&backup_path).expect("open backup");
        let count = backup_connection
            .query_row("SELECT COUNT(*) FROM entries", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count backup entries");
        assert_eq!(count, 1);
    }

    #[test]
    fn create_backup_prunes_oldest_backup_and_manifest() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT);
                INSERT INTO entries (text) VALUES ('safe');
                ",
            )
            .expect("fixture");
        drop(connection);

        let mut first_backup_path = PathBuf::new();
        let mut first_manifest_path = PathBuf::new();

        for index in 0..=db::DEFAULT_BACKUP_RETENTION_COUNT {
            let response = create_backup_for_database(
                &db_path,
                BackupCreateRequest {
                    operation: Some(format!("test.backup.{index}")),
                },
            )
            .expect("backup response");

            if index == 0 {
                first_backup_path = PathBuf::from(&response.backup.path);
                first_manifest_path =
                    PathBuf::from(response.backup.manifest_path.expect("manifest path"));
            }
        }

        let response = list_backups_for_database(&db_path).expect("list backups");
        assert_eq!(response.backups.len(), db::DEFAULT_BACKUP_RETENTION_COUNT);
        assert!(!first_backup_path.exists());
        assert!(!first_manifest_path.exists());
    }

    #[test]
    fn write_guard_does_not_run_write_when_backup_fails() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let missing_db_path = temp_dir.path().join("missing.db");
        let mut write_ran = false;

        let result = with_database_backup_for_database(&missing_db_path, "entry.create", |_| {
            write_ran = true;
            Ok(())
        });

        assert!(result.is_err());
        assert!(!write_ran);
    }

    #[test]
    fn timed_mutation_lock_honors_caller_budget() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        Connection::open(&db_path).expect("open db");

        let held = acquire_mutation_lock(&db_path).expect("hold mutation lock");
        let started = Instant::now();
        let mut write_ran = false;
        let result = with_mutation_lock_for_database_with_timeout(
            &db_path,
            Duration::from_millis(80),
            |_| {
                write_ran = true;
                Ok(())
            },
        );

        assert!(result.is_err());
        assert!(!write_ran);
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(result
            .expect_err("held lock must time out")
            .downcast_ref::<MutationBusy>()
            .is_some());
        drop(held);
    }

    #[test]
    fn explicit_policy_publishes_distinct_verified_backups_and_retains_count() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        let backup_directory = temp_dir.path().join("shared-backups");
        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT);
                INSERT INTO entries (text) VALUES ('safe');
                ",
            )
            .expect("fixture");
        drop(connection);
        let policy = crate::contracts::BackupPolicy::new(&backup_directory, 2);

        let mut published = Vec::new();
        for index in 0..4 {
            let guarded = with_database_backup_for_database_using_policy(
                &db_path,
                &format!("test.explicit.{index}"),
                &policy,
                |_| Ok(()),
            )
            .expect("explicit backup");
            let path = PathBuf::from(&guarded.audit.backup_path);
            assert!(path.exists());
            let connection =
                Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .expect("open verified backup");
            assert_eq!(
                connection
                    .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
            published.push(path);

            let leftovers = fs::read_dir(&backup_directory)
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    name.contains(".tmp-") || name.ends_with(".reserve")
                })
                .count();
            assert_eq!(leftovers, 0);
        }

        let backups = backup_infos_in_directory(&backup_directory).expect("list explicit backups");
        assert_eq!(backups.len(), 2);
        assert_eq!(
            published
                .iter()
                .filter(|path| path.exists())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            2
        );
    }

    #[test]
    fn shared_backup_directory_lock_keeps_concurrent_databases_distinct() {
        use std::sync::{Arc, Barrier};

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let backup_directory = temp_dir.path().join("shared-backups");
        let mut requests = Vec::new();
        for index in 0..2 {
            let db_path = temp_dir.path().join(format!("capsule-{index}.db"));
            let connection = Connection::open(&db_path).expect("open db");
            connection
                .execute_batch(
                    "CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT);
                     INSERT INTO entries (text) VALUES ('safe');",
                )
                .expect("fixture");
            drop(connection);
            requests.push(db_path);
        }
        let policy = crate::contracts::BackupPolicy::new(&backup_directory, 10);
        let barrier = Arc::new(Barrier::new(2));
        let handles = requests
            .into_iter()
            .map(|db_path| {
                let barrier = barrier.clone();
                let policy = policy.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    with_database_backup_for_database_using_policy(
                        &db_path,
                        "test.shared",
                        &policy,
                        |_| Ok(()),
                    )
                    .expect("shared backup")
                })
            })
            .collect::<Vec<_>>();
        let responses = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        let paths = responses
            .iter()
            .map(|response| response.audit.backup_path.clone())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(paths.len(), 2);
        assert_eq!(
            backup_infos_in_directory(&backup_directory).unwrap().len(),
            2
        );
        for path in paths {
            assert!(PathBuf::from(path).exists());
        }
    }

    #[test]
    fn restore_backup_replaces_database_after_safety_backup() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        let active_connection = Connection::open(&db_path).expect("open active db");
        active_connection
            .execute_batch(
                "
                CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT);
                INSERT INTO entries (text) VALUES ('current');
                ",
            )
            .expect("active fixture");
        drop(active_connection);

        let backup_path = temp_dir.path().join("capsule_backup_20260629_120000.db");
        let backup_connection = Connection::open(&backup_path).expect("open backup db");
        backup_connection
            .execute_batch(
                "
                CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT);
                INSERT INTO entries (text) VALUES ('restored');
                ",
            )
            .expect("backup fixture");
        drop(backup_connection);

        let preview = preview_restore_backup_for_database(
            &db_path,
            BackupRestorePreviewRequest {
                backup_path: db::path_to_string(&backup_path),
            },
        )
        .expect("preview");
        assert_eq!(preview.entry_count, Some(1));

        let response = restore_backup_for_database(
            &db_path,
            BackupRestoreRequest {
                backup_path: db::path_to_string(&backup_path),
                confirmation: Some("RESTORE".to_string()),
            },
        )
        .expect("restore");

        assert!(PathBuf::from(response.safety_backup.path).exists());
        let restored_connection = Connection::open(&db_path).expect("open restored db");
        let text = restored_connection
            .query_row("SELECT text FROM entries", [], |row| {
                row.get::<_, String>(0)
            })
            .expect("restored text");
        assert_eq!(text, "restored");
    }
}
