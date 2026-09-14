//! Capture identity validation and database binding guards.
//!
//! A capture's UUID is reserved before any optional context work starts.  The
//! UUID, rather than entry text, is the idempotency key.  This module keeps the
//! validation intentionally independent from SQLite writes so callers can
//! reconcile an uncertain result through a read-only connection.

use std::{fs, path::Path};

use anyhow::{anyhow, Result};

use crate::{contracts::CaptureRequest, db::FileIdentity};

const MAX_CAPTURE_ID_BYTES: usize = 255;
const MAX_RESERVED_UUID_BYTES: usize = 255;

#[derive(Debug)]
pub struct DatabaseBindingError(pub String);

impl std::fmt::Display for DatabaseBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DatabaseBindingError {}

/// Validate a capture request's identity and immutable database destination.
/// Text and metadata normalization is performed by `entries`; this function
/// only rejects identity values that could not safely be retried.
pub fn validate_capture_request(request: &CaptureRequest) -> Result<()> {
    validate_capture_id(&request.capture_id)?;
    validate_reserved_uuid(&request.reserved_uuid)?;
    if request.database_path.as_os_str().is_empty() {
        return Err(anyhow!("capture database path is required"));
    }
    if !request.database_path.is_absolute() {
        return Err(anyhow!(
            "capture database path must be absolute so retries remain bound"
        ));
    }
    if request.text.trim().is_empty() {
        return Err(anyhow!("capture text is required"));
    }
    if let Some(parent) = request.continue_from_uuid.as_deref() {
        if parent != parent.trim()
            || parent.trim().is_empty()
            || parent.chars().all(|character| character.is_ascii_digit())
            || parent
                .chars()
                .any(|character| character.is_control() || character.is_whitespace())
        {
            return Err(anyhow!(
                "capture continuation must be a stable parent UUID, not a numeric alias"
            ));
        }
    }
    Ok(())
}

/// Capture IDs are local recovery handles, not SQL identifiers.  Permit
/// punctuation useful to callers while rejecting whitespace/control bytes and
/// unbounded values.
pub fn validate_capture_id(value: &str) -> Result<String> {
    if value != value.trim() {
        return Err(anyhow!("capture ID must not have surrounding whitespace"));
    }
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("capture ID is required"));
    }
    if value.len() > MAX_CAPTURE_ID_BYTES {
        return Err(anyhow!("capture ID exceeds {MAX_CAPTURE_ID_BYTES} bytes"));
    }
    if value
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(anyhow!(
            "capture ID must not contain whitespace or control characters"
        ));
    }
    Ok(value.to_string())
}

/// Capsule-compatible externally reserved UUIDs use the familiar `entry_`
/// prefix and a conservative ASCII suffix.  Existing journal UUIDs such as
/// `entry_root` remain valid; no UUID is ever regenerated during retry.
pub fn validate_reserved_uuid(value: &str) -> Result<String> {
    if value != value.trim() {
        return Err(anyhow!(
            "reserved entry UUID must not have surrounding whitespace"
        ));
    }
    let value = value.trim();
    if value.len() > MAX_RESERVED_UUID_BYTES {
        return Err(anyhow!(
            "reserved entry UUID exceeds {MAX_RESERVED_UUID_BYTES} bytes"
        ));
    }
    if !value.starts_with("entry_") || value.len() <= "entry_".len() {
        return Err(anyhow!(
            "reserved entry UUID must use the Capsule-compatible entry_ prefix"
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(anyhow!(
            "reserved entry UUID may contain only ASCII letters, digits, '_' or '-'"
        ));
    }
    Ok(value.to_string())
}

/// Freeze a database binding at request creation.  Callers should persist the
/// returned identity with their pending record and pass it back on retries.
pub fn freeze_database_identity(path: &Path) -> Result<FileIdentity> {
    if !path.exists() {
        return Err(anyhow!(
            "capture database does not exist: {}",
            path.display()
        ));
    }
    let identity = FileIdentity::for_path(path);
    if identity.stable_id.is_none() {
        return Err(anyhow!(
            "capture database has no stable filesystem identity: {}",
            path.display()
        ));
    }
    Ok(identity)
}

/// Fail closed when the destination disappeared, was replaced at the same
/// path, or no longer matches the canonical path captured with the request.
/// Mutable size/mtime observations are deliberately ignored by
/// `FileIdentity::same_file`; a stable OS file ID is authoritative when one is
/// available.
pub fn validate_database_binding(path: &Path, expected: &FileIdentity) -> Result<()> {
    if !path.exists() {
        return Err(binding_error(format!(
            "capture database disappeared: {}",
            path.display()
        )));
    }

    let current = FileIdentity::for_path(path);
    if !same_canonical_path(&current.canonical_path, &expected.canonical_path) {
        return Err(binding_error(format!(
            "capture database path changed from {} to {}",
            expected.canonical_path, current.canonical_path
        )));
    }

    if !expected.same_file(&current) {
        return Err(binding_error(format!(
            "capture database file identity changed at {}",
            expected.canonical_path
        )));
    }

    // If the original platform exposed a stable ID, a retry without one is
    // not verifiable and must not write to the path merely because it matches.
    if expected.stable_id.is_none() || current.stable_id.is_none() {
        return Err(binding_error(format!(
            "capture database identity cannot be revalidated at {}",
            expected.canonical_path
        )));
    }
    Ok(())
}

/// Validate the request and either use its frozen identity or bind it to the
/// current file for a first attempt.  The returned identity is then used for
/// every pre-backup and pre-commit check.
pub fn bind_request_database(request: &CaptureRequest) -> Result<FileIdentity> {
    validate_capture_request(request)?;
    match request.database_identity.as_ref() {
        Some(identity) => {
            validate_database_binding(&request.database_path, identity)?;
            Ok(identity.clone())
        }
        None => Err(anyhow!(
            "capture request is missing its frozen database identity; bind it before retrying"
        )),
    }
}

fn same_canonical_path(left: &str, right: &str) -> bool {
    normalize_path(left) == normalize_path(right)
}

fn normalize_path(value: &str) -> String {
    value.replace('/', "\\").to_lowercase()
}

fn binding_error(message: String) -> anyhow::Error {
    anyhow::Error::new(DatabaseBindingError(message))
}

/// Read-only helper used by diagnostics and tests to ensure the file still
/// exists without opening it as a writable SQLite handle.
pub fn database_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};
    use std::path::PathBuf;

    fn request(path: PathBuf) -> CaptureRequest {
        CaptureRequest::new(
            "A note",
            "cap-1",
            "entry_reserved",
            path,
            FixedOffset::east_opt(2 * 60 * 60)
                .unwrap()
                .with_ymd_and_hms(2026, 9, 14, 18, 42, 0)
                .unwrap(),
        )
    }

    #[test]
    fn reserved_uuid_validation_keeps_capsule_prefix_and_rejects_sqlish_values() {
        assert_eq!(
            validate_reserved_uuid("entry_client-01").unwrap(),
            "entry_client-01"
        );
        assert!(validate_reserved_uuid("client-01").is_err());
        assert!(validate_reserved_uuid("entry_bad space").is_err());
    }

    #[test]
    fn binding_detects_same_path_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("capsule.db");
        let replacement = directory.path().join("replacement.db");
        fs::write(&path, b"old").unwrap();
        fs::write(&replacement, b"new").unwrap();
        let expected = freeze_database_identity(&path).unwrap();
        fs::remove_file(&path).unwrap();
        fs::rename(&replacement, &path).unwrap();
        assert!(validate_database_binding(&path, &expected).is_err());
    }

    #[test]
    fn request_validation_rejects_relative_database_binding() {
        let request = request(PathBuf::from("journal.db"));
        assert!(validate_capture_request(&request).is_err());
    }
}
