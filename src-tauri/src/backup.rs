use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::{backup::Backup, Connection};

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

pub fn preview_restore_backup(input: BackupRestorePreviewRequest) -> Result<BackupRestorePreview> {
    let db_path = db::resolve_database_path();
    preview_restore_backup_for_database(&db_path, input)
}

pub(crate) fn preview_restore_backup_for_database(
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

pub(crate) fn restore_backup_for_database(
    db_path: &Path,
    input: BackupRestoreRequest,
) -> Result<BackupRestoreResponse> {
    if input.confirmation.as_deref() != Some("RESTORE") {
        return Err(anyhow!("Restore confirmation must be RESTORE."));
    }

    let backup_path = validate_restore_backup_path(db_path, &input.backup_path)?;
    verify_backup(&backup_path)?;
    let restored_from = backup_info_from_path(&backup_path)?;

    let safety_backup = create_backup_for_database(
        db_path,
        BackupCreateRequest {
            operation: Some("backup.restore.safety".to_string()),
        },
    )
    .context("failed to create a safety backup before restore")?
    .backup;

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

pub fn open_backup_folder() -> Result<()> {
    let backup_directory = db::backup_directory_for_database(&db::resolve_database_path());
    fs::create_dir_all(&backup_directory)
        .with_context(|| format!("failed to create {}", backup_directory.display()))?;
    Command::new("explorer.exe")
        .arg(&backup_directory)
        .spawn()
        .with_context(|| format!("failed to open {}", backup_directory.display()))?;
    Ok(())
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
