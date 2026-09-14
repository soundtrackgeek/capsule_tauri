//! Capsule location compatibility adapters and schema helpers.
//!
//! The actual provider orchestration lives in [`crate::context`].  This module
//! keeps the names used by the desktop and sync adapters stable while making
//! those callers use the same bounded, injectable implementation as `cap`.

use std::{
    cell::RefCell,
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::NaiveDateTime;
use rusqlite::Connection;
use serde_json::{Map, Value as JsonValue};

/// Test-only fixture retained for Capsule's existing backend tests.  It is
/// deliberately scoped to the current thread and is never consulted by the
/// production provider adapters.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct TestAutoCaptureFixture {
    pub latitude: f64,
    pub longitude: f64,
    pub place_name: Option<String>,
    pub source: String,
    pub weather_temp_c: Option<f64>,
    pub weather_condition: Option<String>,
}

thread_local! {
    static TEST_AUTO_CAPTURE: RefCell<Option<TestAutoCaptureFixture>> = const { RefCell::new(None) };
}

#[doc(hidden)]
pub fn set_test_auto_capture_fixture(fixture: Option<TestAutoCaptureFixture>) {
    TEST_AUTO_CAPTURE.with(|slot| *slot.borrow_mut() = fixture);
}

pub(crate) fn test_auto_capture_fixture() -> Option<TestAutoCaptureFixture> {
    TEST_AUTO_CAPTURE.with(|slot| slot.borrow().clone())
}

/// Preserve the desktop command's historical boolean result while routing the
/// work through the shared context service.  A `true` result means that a
/// location row was persisted; weather may still be unavailable.
pub fn auto_capture_location(db_path: &Path, entry_uuid: &str) -> Result<bool> {
    auto_capture_location_with_config_path(db_path, entry_uuid, None)
}

/// Explicit-config variant used by non-desktop callers.  The path is read
/// exactly and no process environment is consulted when it is supplied.
pub fn auto_capture_location_with_config_path(
    db_path: &Path,
    entry_uuid: &str,
    config_path: Option<&Path>,
) -> Result<bool> {
    let settings = load_context_settings(db_path, config_path)?;
    if !settings.valid {
        // An inherited malformed config must not silently grant permission to
        // send coordinates.  The old desktop adapter represented this as a
        // skipped capture, so retain that behavior here.
        return Ok(false);
    }

    let request = crate::context::ContextRequest::capture(db_path, entry_uuid, settings.policy);
    let result = crate::context::ContextService::default().capture(&request)?;
    Ok(result.location.is_some())
}

/// Report whether mobile rows still have missing context.  This is a strict
/// read-only query and never creates the location schema or a backup.
pub fn has_pending_mobile_location_enrichment(db_path: &Path) -> Result<bool> {
    let connection = crate::db::open_read_only_connection(db_path)?;
    if !table_exists(&connection, "plugin_entry_locations")? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT EXISTS (
                SELECT 1
                FROM plugin_entry_locations
                WHERE LOWER(TRIM(source)) = 'mobile'
                  AND (
                    place_name IS NULL OR TRIM(place_name) = ''
                    OR weather_temp_c IS NULL
                    OR weather_condition IS NULL OR TRIM(weather_condition) = ''
                    OR weather_fetched_at IS NULL OR TRIM(weather_fetched_at) = ''
                  )
            )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(Into::into)
}

/// Enrich pending mobile rows without reserving a second backup.  The sync
/// workflow already wraps its entire operation in Capsule's verified backup
/// helper; explicit `cap enrich` uses the context module's backup wrapper.
pub fn enrich_pending_mobile_locations(db_path: &Path) -> Result<usize> {
    crate::context::enrich_pending_mobile_locations(db_path)
}

/// The small settings snapshot consumed by the context service.  Location
/// settings are intentionally flat because that is Capsule's persisted shape.
#[derive(Debug, Clone)]
pub struct ContextSettings {
    pub policy: crate::contracts::ContextPolicy,
    pub valid: bool,
}

/// Load the effective flat `location.*` policy without exposing unrelated
/// configuration values.  An explicit path is authoritative and never falls
/// through to environment or sibling candidates; inherited malformed files
/// return `valid = false`, while a missing inherited file uses Capsule defaults.
pub fn load_context_settings(
    db_path: &Path,
    config_path: Option<&Path>,
) -> Result<ContextSettings> {
    let (values, valid) = match config_path {
        Some(path) => match try_load_config(path) {
            Some(values) => (values, true),
            None => (Map::new(), false),
        },
        None => {
            let mut found = None;
            let mut invalid = false;
            for path in config_path_candidates(db_path) {
                if !path.exists() {
                    continue;
                }
                match read_config_object(&path) {
                    Ok(values) => {
                        found = Some(values);
                        break;
                    }
                    Err(_) => {
                        // An environment-selected config is an explicit
                        // authority and must not fall through to a sibling;
                        // an unreadable/malformed inherited sibling is also
                        // unsafe to treat as permission to send location.
                        invalid = true;
                        break;
                    }
                }
            }
            match found {
                Some(values) => (values, true),
                None if invalid => (Map::new(), false),
                // A missing inherited config means Capsule defaults apply.
                None => (Map::new(), true),
            }
        }
    };

    let policy = crate::contracts::ContextPolicy {
        database_path: db_path.to_path_buf(),
        auto_capture: bool_value(&values, "location.auto_capture", true),
        use_default_location: bool_value(&values, "location.use_default_location", false),
        default_location_name: string_value(&values, "location.default_location_name"),
        auto_capture_method: string_value(&values, "location.auto_capture_method")
            .or_else(|| Some("ip".to_string())),
        weather_provider: string_value(&values, "location.weather_provider")
            .or_else(|| Some("open_meteo".to_string())),
        geocoding_cache_hours: string_value(&values, "location.geocoding_cache_hours")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|hours| *hours > 0),
        ..crate::contracts::ContextPolicy::default()
    };

    Ok(ContextSettings { policy, valid })
}

fn config_path_candidates(db_path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(value) = env::var("CAPSULE_CONFIG_PATH") {
        if !value.trim().is_empty() {
            paths.push(PathBuf::from(value));
        }
    }
    paths.push(crate::db::database_directory_for_database(db_path).join("config.json"));
    dedupe_paths(paths)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.to_string_lossy().to_lowercase()))
        .collect()
}

fn try_load_config(path: &Path) -> Option<Map<String, JsonValue>> {
    read_config_object(path).ok()
}

fn read_config_object(path: &Path) -> std::result::Result<Map<String, JsonValue>, String> {
    let raw = fs::read(path).map_err(|error| error.to_string())?;
    let value = serde_json::from_slice::<JsonValue>(&raw).map_err(|error| error.to_string())?;
    match value {
        JsonValue::Object(values) => Ok(values),
        _ => Err("configuration root must be a JSON object".to_string()),
    }
}

fn bool_value(values: &Map<String, JsonValue>, key: &str, default: bool) -> bool {
    match values.get(key) {
        Some(JsonValue::Bool(value)) => *value,
        Some(JsonValue::String(value)) => {
            matches!(
                value.trim().to_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            )
        }
        Some(JsonValue::Number(value)) => value.as_i64().unwrap_or(0) != 0,
        _ => default,
    }
}

fn string_value(values: &Map<String, JsonValue>, key: &str) -> Option<String> {
    let value = values.get(key)?;
    let value = match value {
        JsonValue::String(value) => value.clone(),
        JsonValue::Bool(value) => value.to_string(),
        JsonValue::Number(value) => value.to_string(),
        _ => return None,
    };
    (!value.trim().is_empty()).then_some(value.trim().to_string())
}

pub(crate) fn ensure_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS plugin_entry_locations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_uuid TEXT NOT NULL UNIQUE,
            latitude REAL NOT NULL,
            longitude REAL NOT NULL,
            place_name TEXT,
            place_details TEXT,
            source TEXT NOT NULL DEFAULT 'auto',
            weather_temp_c REAL,
            weather_temp_f REAL,
            weather_condition TEXT,
            weather_icon TEXT,
            weather_humidity INTEGER,
            weather_wind_kph REAL,
            weather_fetched_at TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (entry_uuid) REFERENCES entries(uuid) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS plugin_location_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            latitude REAL NOT NULL,
            longitude REAL NOT NULL,
            place_name TEXT NOT NULL,
            place_details TEXT,
            reverse_geocoded_at TEXT NOT NULL,
            UNIQUE(latitude, longitude)
        );
        CREATE TABLE IF NOT EXISTS sync_location_tombstones (
            entry_uuid TEXT NOT NULL PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_locations_entry_uuid
            ON plugin_entry_locations(entry_uuid);
        CREATE INDEX IF NOT EXISTS idx_plugin_location_cache_coords
            ON plugin_location_cache(latitude, longitude);
        ",
    )?;

    add_missing_column(
        connection,
        "plugin_entry_locations",
        "place_details",
        "TEXT",
    )?;
    add_missing_column(
        connection,
        "plugin_entry_locations",
        "source",
        "TEXT NOT NULL DEFAULT 'auto'",
    )?;
    add_missing_column(connection, "plugin_entry_locations", "weather_icon", "TEXT")?;
    add_missing_column(
        connection,
        "plugin_entry_locations",
        "weather_humidity",
        "INTEGER",
    )?;
    add_missing_column(
        connection,
        "plugin_entry_locations",
        "weather_wind_kph",
        "REAL",
    )?;
    add_missing_column(
        connection,
        "plugin_entry_locations",
        "weather_fetched_at",
        "TEXT",
    )?;
    add_missing_column(connection, "plugin_location_cache", "place_details", "TEXT")?;
    Ok(())
}

fn add_missing_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> Result<()> {
    let columns = table_columns(connection, table_name)?;
    if columns.contains(column_name) {
        return Ok(());
    }
    connection.execute(
        &format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {definition}"),
        [],
    )?;
    Ok(())
}

pub(crate) fn table_columns(connection: &Connection, table_name: &str) -> Result<HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
}

pub(crate) fn table_exists(connection: &Connection, table_name: &str) -> Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS (
                SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
            )",
            [table_name],
            |row| row.get::<_, bool>(0),
        )
        .map_err(Into::into)
}

pub(crate) fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

// These helpers remain available to the module's historical unit tests and to
// provider fixtures without exposing provider implementation details publicly.
#[allow(dead_code)]
pub(crate) fn open_meteo_code(code: i64, is_day: i64) -> (&'static str, &'static str) {
    crate::providers::open_meteo_code(code, is_day)
}

#[allow(dead_code)]
pub(crate) fn build_place_name(address: &JsonValue) -> String {
    crate::providers::build_place_name(address)
}

#[allow(dead_code)]
pub(crate) fn build_precise_place_name(address: &JsonValue) -> String {
    crate::providers::build_precise_place_name(address)
}

#[allow(dead_code)]
fn parse_entry_time(value: &str) -> Option<NaiveDateTime> {
    crate::providers::parse_entry_time(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn open_meteo_codes_match_capsule_labels() {
        assert_eq!(open_meteo_code(3, 1), ("Overcast", "cloudy"));
        assert_eq!(open_meteo_code(0, 0), ("Clear", "clear-night"));
        assert_eq!(open_meteo_code(61, 1), ("Slight rain", "rain"));
    }

    #[test]
    fn builds_capsule_place_names() {
        let address = json!({
            "city": "Seattle",
            "state": "Washington",
            "country": "United States",
            "country_code": "us"
        });
        assert_eq!(build_place_name(&address), "Seattle, WA");

        let address = json!({
            "municipality": "Tromso",
            "country": "Norway",
            "country_code": "no"
        });
        assert_eq!(build_place_name(&address), "Tromso, Norway");

        let address = json!({
            "road": "Utsikten",
            "suburb": "Stakkevollan",
            "city": "Tromso",
            "country": "Norway",
            "country_code": "no"
        });
        assert_eq!(
            build_precise_place_name(&address),
            "Utsikten, Tromso, Norway"
        );
    }

    #[test]
    fn malformed_explicit_config_is_not_treated_as_permission() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, b"not-json").expect("config");
        let settings = load_context_settings(&db_path, Some(&config_path)).expect("settings");
        assert!(!settings.valid);
        assert!(!settings.policy.use_default_location);
    }

    #[test]
    fn missing_inherited_config_uses_capsule_defaults() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let settings =
            load_context_settings(&temp_dir.path().join("capsule.db"), None).expect("settings");
        assert!(settings.valid);
        assert!(settings.policy.auto_capture);
        assert_eq!(settings.policy.auto_capture_method.as_deref(), Some("ip"));
    }

    #[test]
    fn malformed_inherited_config_disables_context_instead_of_granting_network() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        fs::write(temp_dir.path().join("config.json"), b"not-json").expect("config");
        let settings = load_context_settings(&db_path, None).expect("settings");
        assert!(!settings.valid);
        assert!(settings.policy.auto_capture);
    }

    #[test]
    fn pending_query_is_read_only_when_schema_is_absent() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        let connection = Connection::open(&db_path).expect("db");
        connection
            .execute_batch(
                "CREATE TABLE entries (uuid TEXT PRIMARY KEY, created_at TEXT NOT NULL);",
            )
            .expect("entries");
        drop(connection);
        assert!(!has_pending_mobile_location_enrichment(&db_path).expect("query"));
        assert!(!db_path.with_extension("db-wal").exists());
    }
}
