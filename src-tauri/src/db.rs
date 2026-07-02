use std::{
    env, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::{
    backup,
    models::{DatabaseStatus, SchemaSummary, SecurityMode, SecurityStatus},
};

const MVP_DATABASE_PATH: &str = r"C:\Users\jtill\.capsule\capsule.db";
const PATH_SETTINGS_ENV: &str = "CAPSULE_PATH_SETTINGS_PATH";
const BACKUP_DIRECTORY_ENV: &str = "CAPSULE_BACKUP_DIR";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPathSettings {
    pub database_path: Option<String>,
    pub image_media_root: Option<String>,
    pub backup_directory: Option<String>,
    pub sync_path: Option<String>,
    pub github_gist_id: Option<String>,
    pub github_gist_token: Option<String>,
    pub auto_sync_enabled: Option<bool>,
    pub auto_sync_interval_minutes: Option<i64>,
}

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
        warnings.push("No Capsule backups were found in the active backup directory.".to_string());
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
    resolve_database_path_from_parts(
        env_value("CAPSULE_DB_PATH"),
        read_local_path_settings().database_path,
        env_value("USERPROFILE"),
        env_value("CAPSULE_HOME"),
        Path::new(MVP_DATABASE_PATH),
        Path::exists,
    )
}

fn resolve_database_path_from_parts(
    capsule_db_path: Option<String>,
    local_database_path: Option<String>,
    userprofile: Option<String>,
    capsule_home: Option<String>,
    mvp_database_path: &Path,
    path_exists: impl Fn(&Path) -> bool,
) -> PathBuf {
    if let Some(path) = capsule_db_path {
        return PathBuf::from(path);
    }

    if let Some(path) = local_database_path {
        return PathBuf::from(path);
    }

    if path_exists(mvp_database_path) {
        return mvp_database_path.to_path_buf();
    }

    if let Some(profile) = userprofile {
        let profile_path = PathBuf::from(profile).join(".capsule").join("capsule.db");
        if path_exists(&profile_path) {
            return profile_path;
        }
    }

    if let Some(home) = capsule_home {
        return PathBuf::from(home).join("capsule.db");
    }

    mvp_database_path.to_path_buf()
}

pub fn backup_directory_for_database(path: &Path) -> PathBuf {
    if is_active_database_path(path) {
        if let Some(path) = env_value(BACKUP_DIRECTORY_ENV) {
            return PathBuf::from(path);
        }
        if let Some(path) = read_local_path_settings().backup_directory {
            return PathBuf::from(path);
        }
    }

    database_directory_for_database(path)
}

pub fn database_directory_for_database(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn local_image_media_root_for_database(path: &Path) -> Option<PathBuf> {
    if is_active_database_path(path) {
        return read_local_path_settings()
            .image_media_root
            .map(PathBuf::from);
    }
    None
}

pub fn is_active_database_path(path: &Path) -> bool {
    comparable_path(path) == comparable_path(&resolve_database_path())
}

pub fn local_path_settings_path() -> PathBuf {
    if let Some(path) = env_value(PATH_SETTINGS_ENV) {
        return PathBuf::from(path);
    }

    if let Some(app_data) = env_value("APPDATA") {
        return PathBuf::from(app_data)
            .join("Capsule")
            .join("path_settings.json");
    }

    if let Some(profile) = env_value("USERPROFILE") {
        return PathBuf::from(profile)
            .join(".capsule")
            .join("path_settings.json");
    }

    PathBuf::from("path_settings.json")
}

pub fn local_github_gist_sync_cache_path() -> PathBuf {
    local_path_settings_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gist_sync")
}

pub fn read_local_path_settings() -> LocalPathSettings {
    try_read_local_path_settings().unwrap_or_default()
}

pub fn try_read_local_path_settings() -> Result<LocalPathSettings> {
    let path = local_path_settings_path();
    if !path.exists() {
        return Ok(LocalPathSettings::default());
    }

    let raw = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut settings: LocalPathSettings = serde_json::from_slice(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    settings.normalize();
    Ok(settings)
}

pub fn write_local_path_settings(settings: &LocalPathSettings) -> Result<()> {
    let path = local_path_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut settings = settings.clone();
    settings.normalize();
    fs::write(&path, serde_json::to_vec_pretty(&settings)?)
        .with_context(|| format!("failed to write {}", path.display()))
}

pub fn open_read_only_connection(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", path.display()))?;
    connection.busy_timeout(std::time::Duration::from_millis(15_000))?;
    connection.pragma_update(None, "query_only", "ON")?;
    Ok(connection)
}

pub fn open_read_write_connection(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open {}", path.display()))?;
    connection.busy_timeout(std::time::Duration::from_millis(15_000))?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
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

fn comparable_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn system_time_to_iso(value: SystemTime) -> String {
    DateTime::<Utc>::from(value).to_rfc3339()
}

impl LocalPathSettings {
    fn normalize(&mut self) {
        self.database_path = normalize_path_setting(self.database_path.take());
        self.image_media_root = normalize_path_setting(self.image_media_root.take());
        self.backup_directory = normalize_path_setting(self.backup_directory.take());
        self.sync_path = normalize_path_setting(self.sync_path.take());
        self.github_gist_id = normalize_path_setting(self.github_gist_id.take());
        self.github_gist_token = normalize_path_setting(self.github_gist_token.take());
        self.auto_sync_interval_minutes = self
            .auto_sync_interval_minutes
            .map(|minutes| minutes.clamp(1, 24 * 60));
    }
}

fn normalize_path_setting(value: Option<String>) -> Option<String> {
    value
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
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

    #[test]
    fn resolver_prefers_mvp_database_over_capsule_home() {
        let mvp_path = PathBuf::from(r"C:\Users\jtill\.capsule\capsule.db");
        let resolved = resolve_database_path_from_parts(
            None,
            None,
            Some(r"C:\Users\jtill".to_string()),
            Some(r"C:\Users\jtill\OneDrive\.capsule".to_string()),
            &mvp_path,
            |path| path == mvp_path,
        );

        assert_eq!(resolved, mvp_path);
    }

    #[test]
    fn resolver_allows_explicit_capsule_db_path_override() {
        let mvp_path = PathBuf::from(r"C:\Users\jtill\.capsule\capsule.db");
        let override_path = r"D:\fixture\capsule.db";
        let resolved = resolve_database_path_from_parts(
            Some(override_path.to_string()),
            None,
            Some(r"C:\Users\jtill".to_string()),
            Some(r"C:\Users\jtill\OneDrive\.capsule".to_string()),
            &mvp_path,
            |_| true,
        );

        assert_eq!(resolved, PathBuf::from(override_path));
    }
}
