use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::{backup::Backup, Connection};

use crate::{
    db,
    models::{
        BackupCreateRequest, BackupCreateResponse, BackupInfo, BackupListResponse, BackupManifest,
        MutationAudit,
    },
};

const APP_NAME: &str = "capsule-tauri";
const BACKUP_PREFIX: &str = "capsule_backup_";
const BACKUP_DB_EXTENSION: &str = ".db";
const BACKUP_JSON_EXTENSION: &str = "json";

pub fn list_backups() -> Result<BackupListResponse> {
    list_backups_for_database(&db::resolve_database_path())
}

pub fn list_backups_for_database(db_path: &Path) -> Result<BackupListResponse> {
    let backup_directory = db::backup_directory_for_database(db_path);
    let mut backups = Vec::new();

    if backup_directory.exists() {
        for entry in fs::read_dir(&backup_directory)
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

    backups.sort_by(|left, right| right.created_at.cmp(&left.created_at));

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
    let metadata = fs::metadata(db_path)
        .with_context(|| format!("database does not exist: {}", db_path.display()))?;
    let backup_directory = db::backup_directory_for_database(db_path);
    fs::create_dir_all(&backup_directory)
        .with_context(|| format!("failed to create {}", backup_directory.display()))?;

    let now = Utc::now();
    let backup_path = next_available_backup_path(&backup_directory, now);
    let manifest_path = backup_path.with_extension(BACKUP_JSON_EXTENSION);

    let source = db::open_read_only_connection(db_path)?;
    let mut destination = Connection::open(&backup_path)
        .with_context(|| format!("failed to create {}", backup_path.display()))?;
    let backup = Backup::new(&source, &mut destination)?;
    backup.run_to_completion(128, Duration::from_millis(20), None)?;
    drop(backup);
    drop(destination);
    drop(source);

    verify_backup(&backup_path)?;

    let operation = input.operation.unwrap_or_else(|| "manual".to_string());
    let manifest = BackupManifest {
        created_at: now.to_rfc3339(),
        operation,
        app: APP_NAME.to_string(),
        db_path: db::path_to_string(db_path),
        db_size_bytes: metadata.len(),
        backup_path: db::path_to_string(&backup_path),
    };
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;

    let mut backup_info = backup_info_from_path(&backup_path)?;
    backup_info.manifest_path = Some(db::path_to_string(&manifest_path));
    backup_info.operation = Some(manifest.operation);
    backup_info.created_at = Some(manifest.created_at);
    backup_info.verified = true;

    Ok(BackupCreateResponse {
        backup: backup_info,
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

pub fn with_database_backup_for_database<T>(
    db_path: &Path,
    operation: &str,
    write_fn: impl FnOnce(&Path) -> Result<T>,
) -> Result<GuardedWrite<T>> {
    let backup = create_backup_for_database(
        db_path,
        BackupCreateRequest {
            operation: Some(operation.to_string()),
        },
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

pub fn backup_filename_for(timestamp: DateTime<Utc>) -> String {
    format!(
        "{BACKUP_PREFIX}{}{BACKUP_DB_EXTENSION}",
        timestamp.format("%Y%m%d_%H%M%S")
    )
}

fn next_available_backup_path(directory: &Path, timestamp: DateTime<Utc>) -> PathBuf {
    for offset in 0..3600 {
        let candidate = directory.join(backup_filename_for(
            timestamp + chrono::Duration::seconds(offset),
        ));
        if !candidate.exists() {
            return candidate;
        }
    }

    directory.join(backup_filename_for(timestamp))
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

    Ok(())
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
}
