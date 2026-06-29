use std::{
    env, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};

use crate::{
    backup,
    models::{DatabaseStatus, SchemaSummary, SecurityMode, SecurityStatus},
};

pub fn database_status() -> Result<DatabaseStatus> {
    database_status_for_path(resolve_database_path())
}

pub fn database_status_for_path(path: PathBuf) -> Result<DatabaseStatus> {
    let db_path = path_to_string(&path);
    let mut warnings = Vec::new();
    let metadata = fs::metadata(&path).ok();
    let db_exists = metadata.is_some();
    let db_size_bytes = metadata.as_ref().map(|item| item.len()).unwrap_or(0);
    let db_modified_at = metadata
        .as_ref()
        .and_then(|item| item.modified().ok())
        .map(system_time_to_iso);

    if !db_exists {
        warnings.push("Database file does not exist.".to_string());
    }

    let backup_response = backup::list_backups_for_database(&path)?;
    let backup_count = Some(backup_response.backups.len());
    let last_backup_path = backup_response
        .backups
        .first()
        .map(|backup| backup.path.clone());

    if backup_response.backups.is_empty() {
        warnings.push("No Capsule backups were found next to the active database.".to_string());
    }

    if !db_exists {
        return Ok(DatabaseStatus {
            db_path,
            db_exists,
            db_size_bytes,
            db_modified_at,
            readable: false,
            schema_summary: SchemaSummary::empty(),
            entry_count: None,
            tag_count: None,
            backup_count,
            last_backup_path,
            security: SecurityStatus {
                mode: SecurityMode::Unknown,
                locked: false,
                readable: false,
                message: Some("The configured database path does not exist.".to_string()),
            },
            warnings,
        });
    }

    let connection = match open_read_only_connection(&path) {
        Ok(connection) => connection,
        Err(error) => {
            warnings.push(format!("Unable to open database read-only: {error}"));
            return Ok(DatabaseStatus {
                db_path,
                db_exists,
                db_size_bytes,
                db_modified_at,
                readable: false,
                schema_summary: SchemaSummary::empty(),
                entry_count: None,
                tag_count: None,
                backup_count,
                last_backup_path,
                security: SecurityStatus {
                    mode: SecurityMode::Unknown,
                    locked: true,
                    readable: false,
                    message: Some(
                        "The database could not be opened by standard SQLite.".to_string(),
                    ),
                },
                warnings,
            });
        }
    };

    let schema_summary = inspect_schema(&connection)?;
    if !schema_summary.has_entries_table {
        warnings.push("The entries table was not detected.".to_string());
    }

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

    Ok(DatabaseStatus {
        db_path,
        db_exists,
        db_size_bytes,
        db_modified_at,
        readable: true,
        schema_summary,
        entry_count,
        tag_count,
        backup_count,
        last_backup_path,
        security: SecurityStatus {
            mode: SecurityMode::Plain,
            locked: false,
            readable: true,
            message: None,
        },
        warnings,
    })
}

pub fn resolve_database_path() -> PathBuf {
    if let Some(path) = env_value("CAPSULE_DB_PATH") {
        return PathBuf::from(path);
    }

    if let Some(home) = env_value("CAPSULE_HOME") {
        return PathBuf::from(home).join("capsule.db");
    }

    if let Some(profile) = env_value("USERPROFILE") {
        return PathBuf::from(profile).join(".capsule").join("capsule.db");
    }

    PathBuf::from(r"C:\Users\jtill\.capsule\capsule.db")
}

pub fn backup_directory_for_database(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn open_read_only_connection(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", path.display()))?;
    connection.busy_timeout(std::time::Duration::from_millis(15_000))?;
    connection.pragma_update(None, "query_only", "ON")?;
    Ok(connection)
}

pub fn inspect_schema(connection: &Connection) -> Result<SchemaSummary> {
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name COLLATE NOCASE",
    )?;
    let table_iter = statement.query_map([], |row| row.get::<_, String>(0))?;
    let detected_tables: Vec<String> = table_iter.collect::<rusqlite::Result<Vec<_>>>()?;
    let has_entries_table = detected_tables.iter().any(|table| table == "entries");
    let has_tags_table = detected_tables.iter().any(|table| table == "tags");
    let has_fts_table = detected_tables.iter().any(|table| table == "entries_fts");
    let missing_core_tables = ["entries", "tags"]
        .into_iter()
        .filter(|table| !detected_tables.iter().any(|detected| detected == table))
        .map(str::to_string)
        .collect::<Vec<_>>();

    Ok(SchemaSummary {
        table_count: detected_tables.len(),
        detected_tables,
        has_entries_table,
        has_tags_table,
        has_fts_table,
        missing_core_tables,
    })
}

fn count_table_rows(connection: &Connection, table: &str) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    connection
        .query_row(&sql, [], |row| row.get::<_, i64>(0))
        .with_context(|| format!("failed to count rows in {table}"))
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn system_time_to_iso(value: SystemTime) -> String {
    DateTime::<Utc>::from(value).to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn database_status_detects_core_tables_and_counts() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        let connection = Connection::open(&db_path).expect("open temp db");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT);
                CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT);
                CREATE TABLE entries_fts (text);
                INSERT INTO entries (text) VALUES ('first'), ('second');
                INSERT INTO tags (name) VALUES ('work');
                ",
            )
            .expect("create fixture");
        drop(connection);

        let status = database_status_for_path(db_path).expect("status");

        assert!(status.db_exists);
        assert!(status.readable);
        assert_eq!(status.entry_count, Some(2));
        assert_eq!(status.tag_count, Some(1));
        assert!(status.schema_summary.has_entries_table);
        assert!(status.schema_summary.has_tags_table);
        assert!(status.schema_summary.has_fts_table);
    }

    #[test]
    fn database_status_reports_missing_database_without_error() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("missing.db");

        let status = database_status_for_path(db_path).expect("status");

        assert!(!status.db_exists);
        assert!(!status.readable);
        assert_eq!(status.entry_count, None);
        assert!(!status.warnings.is_empty());
    }
}
