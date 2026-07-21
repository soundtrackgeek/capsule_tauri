use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(test)]
use std::cell::RefCell;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDateTime, Timelike};
use reqwest::blocking::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value as JsonValue};
use url::Url;

const NOMINATIM_BASE_URL: &str = "https://nominatim.openstreetmap.org";
const NOMINATIM_USER_AGENT: &str = "CapsuleJournal/1.0 (https://github.com/capsule-journal)";
const IP_API_URL: &str = "http://ip-api.com/json";
const IPINFO_URL: &str = "https://ipinfo.io/json";
const OPEN_METEO_BASE_URL: &str = "https://api.open-meteo.com/v1";
const OPEN_METEO_ARCHIVE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";
const MET_NORWAY_URL: &str = "https://api.met.no/weatherapi/locationforecast/2.0/compact";
const MET_NORWAY_USER_AGENT: &str =
    "CapsuleExp/1.0 (https://github.com/soundtrackgeek/capsule_exp_ai)";

#[derive(Debug, Clone)]
struct WeatherData {
    temp_c: Option<f64>,
    temp_f: Option<f64>,
    condition: Option<String>,
    icon: Option<String>,
    humidity: Option<i64>,
    wind_kph: Option<f64>,
}

#[derive(Debug, Clone)]
struct GeocodeData {
    place_name: String,
    place_details: Option<String>,
}

#[derive(Debug, Clone)]
struct LocationConfig {
    values: Map<String, JsonValue>,
}

#[derive(Debug, Clone)]
struct PendingMobileLocation {
    entry_uuid: String,
    latitude: f64,
    longitude: f64,
    entry_created_at: String,
    place_name: Option<String>,
    place_details: Option<String>,
    weather_temp_c: Option<f64>,
    weather_temp_f: Option<f64>,
    weather_condition: Option<String>,
    weather_icon: Option<String>,
    weather_humidity: Option<i64>,
    weather_wind_kph: Option<f64>,
    weather_fetched_at: Option<String>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct TestAutoCaptureFixture {
    pub latitude: f64,
    pub longitude: f64,
    pub place_name: Option<String>,
    pub source: String,
    pub weather_temp_c: Option<f64>,
    pub weather_condition: Option<String>,
}

#[cfg(test)]
thread_local! {
    static TEST_AUTO_CAPTURE: RefCell<Option<TestAutoCaptureFixture>> = const { RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_test_auto_capture_fixture(fixture: Option<TestAutoCaptureFixture>) {
    TEST_AUTO_CAPTURE.with(|slot| {
        *slot.borrow_mut() = fixture;
    });
}

pub(crate) fn has_pending_mobile_location_enrichment(db_path: &Path) -> Result<bool> {
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

pub(crate) fn enrich_pending_mobile_locations(db_path: &Path) -> Result<usize> {
    let mut connection = crate::db::open_read_write_connection(db_path)?;
    ensure_schema(&connection)?;
    let candidates = pending_mobile_locations(&connection)?;
    if candidates.is_empty() {
        return Ok(0);
    }

    let config = LocationConfig::load(db_path);
    let mut enriched = 0;
    for candidate in candidates {
        match enrich_mobile_location(&mut connection, &candidate, &config) {
            Ok(true) => enriched += 1,
            Ok(false) => {}
            Err(error) => eprintln!(
                "[Location] Failed to enrich mobile location for {}: {error}",
                candidate.entry_uuid
            ),
        }
    }
    Ok(enriched)
}

fn pending_mobile_locations(connection: &Connection) -> Result<Vec<PendingMobileLocation>> {
    let mut statement = connection.prepare(
        "SELECT
            pel.entry_uuid,
            pel.latitude,
            pel.longitude,
            e.created_at,
            pel.place_name,
            pel.place_details,
            pel.weather_temp_c,
            pel.weather_temp_f,
            pel.weather_condition,
            pel.weather_icon,
            pel.weather_humidity,
            pel.weather_wind_kph,
            pel.weather_fetched_at
         FROM plugin_entry_locations pel
         JOIN entries e ON e.uuid = pel.entry_uuid
         WHERE LOWER(TRIM(pel.source)) = 'mobile'
           AND (
             pel.place_name IS NULL OR TRIM(pel.place_name) = ''
             OR pel.weather_temp_c IS NULL
             OR pel.weather_condition IS NULL OR TRIM(pel.weather_condition) = ''
             OR pel.weather_fetched_at IS NULL OR TRIM(pel.weather_fetched_at) = ''
           )",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(PendingMobileLocation {
            entry_uuid: row.get(0)?,
            latitude: row.get(1)?,
            longitude: row.get(2)?,
            entry_created_at: row.get(3)?,
            place_name: row.get(4)?,
            place_details: row.get(5)?,
            weather_temp_c: row.get(6)?,
            weather_temp_f: row.get(7)?,
            weather_condition: row.get(8)?,
            weather_icon: row.get(9)?,
            weather_humidity: row.get(10)?,
            weather_wind_kph: row.get(11)?,
            weather_fetched_at: row.get(12)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn enrich_mobile_location(
    connection: &mut Connection,
    candidate: &PendingMobileLocation,
    config: &LocationConfig,
) -> Result<bool> {
    let place_missing = is_blank(candidate.place_name.as_deref());
    let weather_missing = candidate.weather_temp_c.is_none()
        || is_blank(candidate.weather_condition.as_deref())
        || is_blank(candidate.weather_fetched_at.as_deref());
    let geocode = if place_missing {
        resolve_mobile_place(connection, candidate.latitude, candidate.longitude, config)?
    } else {
        None
    };
    let weather = if weather_missing {
        resolve_mobile_weather(
            candidate.latitude,
            candidate.longitude,
            &candidate.entry_created_at,
            config,
        )
    } else {
        None
    };

    if geocode.is_none() && weather.is_none() {
        return Ok(false);
    }

    let place_name = geocode
        .as_ref()
        .map(|value| value.place_name.clone())
        .or_else(|| candidate.place_name.clone());
    let place_details = geocode
        .as_ref()
        .and_then(|value| value.place_details.clone())
        .or_else(|| candidate.place_details.clone());
    let weather_fetched_at = weather
        .as_ref()
        .map(|_| Local::now().format("%Y-%m-%d %H:%M").to_string())
        .or_else(|| candidate.weather_fetched_at.clone());
    let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let tx = connection.transaction()?;
    tx.execute(
        "UPDATE plugin_entry_locations
         SET place_name = ?2,
             place_details = ?3,
             weather_temp_c = ?4,
             weather_temp_f = ?5,
             weather_condition = ?6,
             weather_icon = ?7,
             weather_humidity = ?8,
             weather_wind_kph = ?9,
             weather_fetched_at = ?10,
             created_at = ?11
         WHERE entry_uuid = ?1",
        params![
            candidate.entry_uuid,
            place_name,
            place_details,
            weather
                .as_ref()
                .and_then(|value| value.temp_c)
                .or(candidate.weather_temp_c),
            weather
                .as_ref()
                .and_then(|value| value.temp_f)
                .or(candidate.weather_temp_f),
            weather
                .as_ref()
                .and_then(|value| value.condition.clone())
                .or_else(|| candidate.weather_condition.clone()),
            weather
                .as_ref()
                .and_then(|value| value.icon.clone())
                .or_else(|| candidate.weather_icon.clone()),
            weather
                .as_ref()
                .and_then(|value| value.humidity)
                .or(candidate.weather_humidity),
            weather
                .as_ref()
                .and_then(|value| value.wind_kph)
                .or(candidate.weather_wind_kph),
            weather_fetched_at,
            created_at,
        ],
    )?;
    tx.commit()?;
    Ok(true)
}

fn resolve_mobile_place(
    connection: &Connection,
    latitude: f64,
    longitude: f64,
    config: &LocationConfig,
) -> Result<Option<GeocodeData>> {
    #[cfg(test)]
    if let Some(fixture) = TEST_AUTO_CAPTURE.with(|slot| slot.borrow().clone()) {
        return Ok(fixture.place_name.map(|place_name| GeocodeData {
            place_name,
            place_details: None,
        }));
    }

    let (place_name, place_details) =
        resolve_precise_place_name(connection, latitude, longitude, config)?;
    Ok(place_name.map(|place_name| GeocodeData {
        place_name,
        place_details,
    }))
}

fn resolve_mobile_weather(
    latitude: f64,
    longitude: f64,
    entry_created_at: &str,
    config: &LocationConfig,
) -> Option<WeatherData> {
    #[cfg(test)]
    if let Some(fixture) = TEST_AUTO_CAPTURE.with(|slot| slot.borrow().clone()) {
        return fixture.weather_temp_c.map(|temp_c| WeatherData {
            temp_c: Some(temp_c),
            temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
            condition: fixture.weather_condition,
            icon: None,
            humidity: None,
            wind_kph: None,
        });
    }

    if let Some(target) = parse_entry_time(entry_created_at) {
        return open_meteo_historical_weather(latitude, longitude, target);
    }
    get_weather(latitude, longitude, None, config)
}

pub(crate) fn auto_capture_location(db_path: &Path, entry_uuid: &str) -> Result<bool> {
    let config = LocationConfig::load(db_path);
    if !config.bool_value("location.auto_capture", true) {
        return Ok(false);
    }

    #[cfg(test)]
    if let Some(fixture) = TEST_AUTO_CAPTURE.with(|slot| slot.borrow().clone()) {
        let weather = fixture.weather_temp_c.map(|temp_c| WeatherData {
            temp_c: Some(temp_c),
            temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
            condition: fixture.weather_condition,
            icon: None,
            humidity: None,
            wind_kph: None,
        });
        attach_location(
            db_path,
            entry_uuid,
            fixture.latitude,
            fixture.longitude,
            fixture.place_name,
            fixture.source,
            weather,
            None,
        )?;
        return Ok(true);
    }

    if config.bool_value("location.use_default_location", false) {
        if let Some(place_name) = config.string_value("location.default_location_name") {
            if let Some((lat, lon)) = geocode_place(&place_name) {
                let weather = get_weather_for_entry(db_path, entry_uuid, lat, lon, &config)?;
                attach_location(
                    db_path,
                    entry_uuid,
                    lat,
                    lon,
                    Some(place_name),
                    "default".to_string(),
                    weather,
                    None,
                )?;
                return Ok(true);
            }
            return Ok(false);
        }
    }

    if config
        .string_value("location.auto_capture_method")
        .unwrap_or_else(|| "ip".to_string())
        .to_lowercase()
        != "ip"
    {
        return Ok(false);
    }

    let Some((lat, lon)) = get_location_from_ip() else {
        return Ok(false);
    };
    let weather = get_weather_for_entry(db_path, entry_uuid, lat, lon, &config)?;
    attach_location(
        db_path,
        entry_uuid,
        lat,
        lon,
        None,
        "ip".to_string(),
        weather,
        None,
    )?;
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn attach_location(
    db_path: &Path,
    entry_uuid: &str,
    lat: f64,
    lon: f64,
    place_name: Option<String>,
    source: String,
    weather: Option<WeatherData>,
    place_details: Option<String>,
) -> Result<()> {
    let mut connection = crate::db::open_read_write_connection(db_path)?;
    ensure_schema(&connection)?;
    let entry_created_at = connection
        .query_row(
            "SELECT created_at FROM entries WHERE uuid = ?1",
            [entry_uuid],
            |row| row.get::<_, String>(0),
        )
        .with_context(|| format!("Entry with UUID '{entry_uuid}' does not exist."))?;

    let config = LocationConfig::load(db_path);
    let (place_name, place_details) = match place_name {
        Some(value) => (Some(value), place_details),
        None => resolve_place_name(&connection, lat, lon, &config)?,
    };
    let weather = match weather {
        Some(value) => Some(value),
        None => get_weather(lat, lon, Some(&entry_created_at), &config),
    };
    let created_at = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let weather_fetched_at = weather
        .as_ref()
        .map(|_| Local::now().format("%Y-%m-%d %H:%M").to_string());

    let tx = connection.transaction()?;
    tx.execute(
        "DELETE FROM plugin_entry_locations WHERE entry_uuid = ?1",
        [entry_uuid],
    )?;
    tx.execute(
        "DELETE FROM sync_location_tombstones WHERE entry_uuid = ?1",
        [entry_uuid],
    )?;
    tx.execute(
        "INSERT INTO plugin_entry_locations (
            entry_uuid, latitude, longitude, place_name, place_details, source,
            weather_temp_c, weather_temp_f, weather_condition, weather_icon,
            weather_humidity, weather_wind_kph, weather_fetched_at, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            entry_uuid,
            lat,
            lon,
            place_name,
            place_details,
            source,
            weather.as_ref().and_then(|item| item.temp_c),
            weather.as_ref().and_then(|item| item.temp_f),
            weather.as_ref().and_then(|item| item.condition.clone()),
            weather.as_ref().and_then(|item| item.icon.clone()),
            weather.as_ref().and_then(|item| item.humidity),
            weather.as_ref().and_then(|item| item.wind_kph),
            weather_fetched_at,
            created_at,
        ],
    )?;
    tx.commit()?;
    Ok(())
}

fn get_weather_for_entry(
    db_path: &Path,
    entry_uuid: &str,
    lat: f64,
    lon: f64,
    config: &LocationConfig,
) -> Result<Option<WeatherData>> {
    let connection = crate::db::open_read_only_connection(db_path)?;
    let entry_created_at = connection
        .query_row(
            "SELECT created_at FROM entries WHERE uuid = ?1",
            [entry_uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(get_weather(lat, lon, entry_created_at.as_deref(), config))
}

fn ensure_schema(connection: &Connection) -> Result<()> {
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
    let sql = format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {definition}");
    connection.execute(&sql, [])?;
    Ok(())
}

fn table_columns(connection: &Connection, table_name: &str) -> Result<HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool> {
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

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

fn resolve_place_name(
    connection: &Connection,
    lat: f64,
    lon: f64,
    config: &LocationConfig,
) -> Result<(Option<String>, Option<String>)> {
    if let Some(cached) = cached_geocode(connection, lat, lon, config)? {
        return Ok((Some(cached.place_name), cached.place_details));
    }

    let Some(geocode) = reverse_geocode(lat, lon) else {
        return Ok((None, None));
    };
    cache_geocode(connection, lat, lon, &geocode)?;
    Ok((Some(geocode.place_name), geocode.place_details))
}

fn resolve_precise_place_name(
    connection: &Connection,
    lat: f64,
    lon: f64,
    config: &LocationConfig,
) -> Result<(Option<String>, Option<String>)> {
    if let Some(cached) = cached_geocode(connection, lat, lon, config)? {
        return Ok((Some(cached.place_name), cached.place_details));
    }

    let Some(geocode) = reverse_geocode_at_zoom(lat, lon, 16, true) else {
        return Ok((None, None));
    };
    cache_geocode(connection, lat, lon, &geocode)?;
    Ok((Some(geocode.place_name), geocode.place_details))
}

fn cached_geocode(
    connection: &Connection,
    lat: f64,
    lon: f64,
    config: &LocationConfig,
) -> Result<Option<GeocodeData>> {
    let cutoff = (Local::now() - chrono::Duration::hours(config.cache_hours()))
        .format("%Y-%m-%d %H:%M")
        .to_string();
    connection
        .query_row(
            "SELECT place_name, place_details
             FROM plugin_location_cache
             WHERE latitude = ?1 AND longitude = ?2 AND reverse_geocoded_at >= ?3",
            params![round4(lat), round4(lon), cutoff],
            |row| {
                Ok(GeocodeData {
                    place_name: row.get(0)?,
                    place_details: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn cache_geocode(connection: &Connection, lat: f64, lon: f64, geocode: &GeocodeData) -> Result<()> {
    connection.execute(
        "INSERT OR REPLACE INTO plugin_location_cache
         (latitude, longitude, place_name, place_details, reverse_geocoded_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            round4(lat),
            round4(lon),
            geocode.place_name,
            geocode.place_details,
            Local::now().format("%Y-%m-%d %H:%M").to_string(),
        ],
    )?;
    Ok(())
}

fn geocode_place(place_name: &str) -> Option<(f64, f64)> {
    let client = http_client().ok()?;
    let url = url_with_params(
        &format!("{NOMINATIM_BASE_URL}/search"),
        vec![
            ("q", place_name.to_string()),
            ("format", "json".to_string()),
            ("limit", "1".to_string()),
            ("addressdetails", "1".to_string()),
        ],
    )?;
    let response = client
        .get(url)
        .header("User-Agent", NOMINATIM_USER_AGENT)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<JsonValue>()
        .ok()?;
    let first = response.as_array()?.first()?;
    let lat = string_or_number(first.get("lat")?)?.parse::<f64>().ok()?;
    let lon = string_or_number(first.get("lon")?)?.parse::<f64>().ok()?;
    if lat == 0.0 && lon == 0.0 {
        None
    } else {
        Some((lat, lon))
    }
}

fn reverse_geocode(lat: f64, lon: f64) -> Option<GeocodeData> {
    reverse_geocode_at_zoom(lat, lon, 10, false)
}

fn reverse_geocode_at_zoom(lat: f64, lon: f64, zoom: i64, precise: bool) -> Option<GeocodeData> {
    let client = http_client().ok()?;
    let url = url_with_params(
        &format!("{NOMINATIM_BASE_URL}/reverse"),
        vec![
            ("lat", lat.to_string()),
            ("lon", lon.to_string()),
            ("format", "json".to_string()),
            ("addressdetails", "1".to_string()),
            ("zoom", zoom.to_string()),
            ("accept-language", "en".to_string()),
        ],
    )?;
    let response = client
        .get(url)
        .header("User-Agent", NOMINATIM_USER_AGENT)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<JsonValue>()
        .ok()?;
    if response.get("error").is_some() {
        return None;
    }

    let address = response
        .get("address")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let place_name = if precise {
        build_precise_place_name(&address)
    } else {
        build_place_name(&address)
    };
    let place_details = json!({
        "place_name": place_name,
        "city": address_value(&address, &["city", "town", "village", "municipality"]),
        "state": address_value(&address, &["state", "province", "region"]),
        "country": address.get("country").and_then(JsonValue::as_str),
        "country_code": address
            .get("country_code")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_uppercase(),
        "display_name": response.get("display_name").and_then(JsonValue::as_str).unwrap_or_default(),
        "raw": address,
    });
    Some(GeocodeData {
        place_name,
        place_details: Some(place_details.to_string()),
    })
}

fn get_location_from_ip() -> Option<(f64, f64)> {
    ip_api_location().or_else(ipinfo_location)
}

fn ip_api_location() -> Option<(f64, f64)> {
    let client = http_client().ok()?;
    let url = url_with_params(IP_API_URL, vec![("fields", "status,lat,lon".to_string())])?;
    let response = client
        .get(url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<JsonValue>()
        .ok()?;
    if response.get("status").and_then(JsonValue::as_str) != Some("success") {
        return None;
    }
    Some((
        response.get("lat")?.as_f64()?,
        response.get("lon")?.as_f64()?,
    ))
}

fn ipinfo_location() -> Option<(f64, f64)> {
    let client = http_client().ok()?;
    let response = client
        .get(IPINFO_URL)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<JsonValue>()
        .ok()?;
    let loc = response.get("loc")?.as_str()?;
    let (lat, lon) = loc.split_once(',')?;
    Some((lat.parse().ok()?, lon.parse().ok()?))
}

fn get_weather(
    lat: f64,
    lon: f64,
    entry_created_at: Option<&str>,
    config: &LocationConfig,
) -> Option<WeatherData> {
    match config
        .string_value("location.weather_provider")
        .unwrap_or_else(|| "open_meteo".to_string())
        .to_lowercase()
        .as_str()
    {
        "met_norway" => met_norway_weather(lat, lon),
        _ => open_meteo_weather(lat, lon, entry_created_at),
    }
}

fn open_meteo_weather(lat: f64, lon: f64, entry_created_at: Option<&str>) -> Option<WeatherData> {
    if let Some(target) = entry_created_at.and_then(parse_entry_time) {
        if Local::now().naive_local().signed_duration_since(target) > chrono::Duration::hours(3) {
            return open_meteo_historical_weather(lat, lon, target);
        }
    }
    open_meteo_current_weather(lat, lon)
}

fn open_meteo_historical_weather(
    lat: f64,
    lon: f64,
    mut target: NaiveDateTime,
) -> Option<WeatherData> {
    if target.minute() >= 30 {
        target += chrono::Duration::hours(1);
    }
    target = target.with_minute(0)?.with_second(0)?.with_nanosecond(0)?;
    let target_date = target.format("%Y-%m-%d").to_string();
    let target_hour = target.format("%Y-%m-%dT%H:00").to_string();
    let days_diff = Local::now()
        .date_naive()
        .signed_duration_since(target.date())
        .num_days();
    let base_url = if days_diff >= 90 {
        OPEN_METEO_ARCHIVE_URL.to_string()
    } else {
        format!("{OPEN_METEO_BASE_URL}/forecast")
    };
    let client = http_client().ok()?;
    let url = url_with_params(
        &base_url,
        vec![
            ("latitude", round4(lat).to_string()),
            ("longitude", round4(lon).to_string()),
            ("start_date", target_date.clone()),
            ("end_date", target_date),
            (
                "hourly",
                "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,is_day"
                    .to_string(),
            ),
            ("timezone", "auto".to_string()),
        ],
    )?;
    let response = client
        .get(url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<JsonValue>()
        .ok()?;
    let hourly = response.get("hourly")?;
    let times = hourly.get("time")?.as_array()?;
    let idx = times
        .iter()
        .position(|value| value.as_str() == Some(target_hour.as_str()))
        .unwrap_or(0);
    let temp_c = hourly_value(hourly, "temperature_2m", idx)?.as_f64()?;
    let weather_code = hourly_value(hourly, "weather_code", idx)
        .and_then(JsonValue::as_i64)
        .unwrap_or(0);
    let humidity = number_to_i64(hourly_value(hourly, "relative_humidity_2m", idx));
    let wind_kph = hourly_value(hourly, "wind_speed_10m", idx).and_then(JsonValue::as_f64);
    let is_day = hourly_value(hourly, "is_day", idx)
        .and_then(JsonValue::as_i64)
        .unwrap_or(1);
    let (condition, icon) = open_meteo_code(weather_code, is_day);
    Some(WeatherData {
        temp_c: Some(round1(temp_c)),
        temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
        condition: Some(condition.to_string()),
        icon: Some(icon.to_string()),
        humidity,
        wind_kph: wind_kph.map(round1),
    })
}

fn open_meteo_current_weather(lat: f64, lon: f64) -> Option<WeatherData> {
    let client = http_client().ok()?;
    let url = url_with_params(
        &format!("{OPEN_METEO_BASE_URL}/forecast"),
        vec![
            ("latitude", round4(lat).to_string()),
            ("longitude", round4(lon).to_string()),
            (
                "current",
                "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,is_day"
                    .to_string(),
            ),
            ("timezone", "auto".to_string()),
        ],
    )?;
    let response = client
        .get(url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<JsonValue>()
        .ok()?;
    let current = response.get("current")?;
    let temp_c = current.get("temperature_2m")?.as_f64()?;
    let weather_code = current
        .get("weather_code")
        .and_then(JsonValue::as_i64)
        .unwrap_or(0);
    let is_day = current
        .get("is_day")
        .and_then(JsonValue::as_i64)
        .unwrap_or(1);
    let (condition, icon) = open_meteo_code(weather_code, is_day);
    Some(WeatherData {
        temp_c: Some(round1(temp_c)),
        temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
        condition: Some(condition.to_string()),
        icon: Some(icon.to_string()),
        humidity: number_to_i64(current.get("relative_humidity_2m")),
        wind_kph: current
            .get("wind_speed_10m")
            .and_then(JsonValue::as_f64)
            .map(round1),
    })
}

fn met_norway_weather(lat: f64, lon: f64) -> Option<WeatherData> {
    let client = http_client().ok()?;
    let url = url_with_params(
        MET_NORWAY_URL,
        vec![
            ("lat", round4(lat).to_string()),
            ("lon", round4(lon).to_string()),
        ],
    )?;
    let response = client
        .get(url)
        .header("User-Agent", MET_NORWAY_USER_AGENT)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<JsonValue>()
        .ok()?;
    let first = response
        .get("properties")?
        .get("timeseries")?
        .as_array()?
        .first()?;
    let data = first.get("data")?;
    let instant = data.get("instant")?.get("details")?;
    let temp_c = instant.get("air_temperature")?.as_f64()?;
    let humidity = number_to_i64(instant.get("relative_humidity"));
    let wind_kph = instant
        .get("wind_speed")
        .and_then(JsonValue::as_f64)
        .map(|value| round1(value * 3.6));
    let symbol_code = data
        .get("next_1_hours")
        .and_then(|value| value.get("summary"))
        .and_then(|value| value.get("symbol_code"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let base_symbol = symbol_code.split('_').next().unwrap_or_default();
    let mut icon = met_norway_icon(base_symbol).to_string();
    if symbol_code.contains("_night") && matches!(icon.as_str(), "clear-day" | "partly-cloudy-day")
    {
        icon = icon.replace("-day", "-night");
    }

    Some(WeatherData {
        temp_c: Some(round1(temp_c)),
        temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
        condition: Some(met_norway_condition(base_symbol)),
        icon: Some(icon),
        humidity,
        wind_kph,
    })
}

fn open_meteo_code(code: i64, is_day: i64) -> (&'static str, &'static str) {
    let (condition, mut icon) = match code {
        0 => ("Clear", "clear-day"),
        1 => ("Mainly clear", "clear-day"),
        2 => ("Partly cloudy", "partly-cloudy-day"),
        3 => ("Overcast", "cloudy"),
        45 | 48 => ("Fog", "fog"),
        51 => ("Light drizzle", "drizzle"),
        53 => ("Moderate drizzle", "drizzle"),
        55 => ("Dense drizzle", "drizzle"),
        56 | 57 => ("Freezing drizzle", "sleet"),
        61 => ("Slight rain", "rain"),
        63 => ("Moderate rain", "rain"),
        65 => ("Heavy rain", "rain"),
        66 | 67 => ("Freezing rain", "sleet"),
        71 => ("Slight snow", "snow"),
        73 => ("Moderate snow", "snow"),
        75 => ("Heavy snow", "snow"),
        77 => ("Snow grains", "snow"),
        80 => ("Slight rain showers", "rain"),
        81 => ("Moderate rain showers", "rain"),
        82 => ("Violent rain showers", "rain"),
        85 => ("Slight snow showers", "snow"),
        86 => ("Heavy snow showers", "snow"),
        95 => ("Thunderstorm", "thunderstorm"),
        96 => ("Thunderstorm with hail", "thunderstorm"),
        99 => ("Thunderstorm with heavy hail", "thunderstorm"),
        _ => ("Unknown", "unknown"),
    };
    if is_day == 0 {
        icon = match icon {
            "clear-day" => "clear-night",
            "partly-cloudy-day" => "partly-cloudy-night",
            other => other,
        };
    }
    (condition, icon)
}

fn met_norway_icon(symbol: &str) -> &'static str {
    match symbol {
        "clearsky" | "fair" => "clear-day",
        "partlycloudy" => "partly-cloudy-day",
        "cloudy" => "cloudy",
        "fog" => "fog",
        value if value.contains("sleet") => "sleet",
        value if value.contains("snow") => "snow",
        value if value.contains("thunder") => "thunderstorm",
        value if value.contains("rain") => "rain",
        _ => "unknown",
    }
}

fn met_norway_condition(symbol: &str) -> String {
    match symbol {
        "fair" => "Mainly clear".to_string(),
        "clearsky" => "Clear".to_string(),
        "partlycloudy" => "Partly cloudy".to_string(),
        _ => title_case(&symbol.replace("andthunder", " & thunder")),
    }
}

fn parse_entry_time(value: &str) -> Option<NaiveDateTime> {
    let normalized = value.trim().replace('T', " ");
    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(&normalized, format) {
            return Some(parsed);
        }
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Local).naive_local())
}

fn hourly_value<'a>(hourly: &'a JsonValue, key: &str, idx: usize) -> Option<&'a JsonValue> {
    hourly.get(key)?.as_array()?.get(idx)
}

fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client")
}

fn url_with_params(base_url: &str, params: Vec<(&str, String)>) -> Option<Url> {
    let mut url = Url::parse(base_url).ok()?;
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, &value);
        }
    }
    Some(url)
}

impl LocationConfig {
    fn load(db_path: &Path) -> Self {
        for path in config_path_candidates(db_path) {
            let Ok(raw) = fs::read(&path) else {
                continue;
            };
            let Ok(JsonValue::Object(values)) = serde_json::from_slice::<JsonValue>(&raw) else {
                continue;
            };
            return Self { values };
        }
        Self { values: Map::new() }
    }

    fn bool_value(&self, key: &str, default: bool) -> bool {
        match self.values.get(key) {
            Some(JsonValue::Bool(value)) => *value,
            Some(JsonValue::String(value)) => {
                matches!(value.to_lowercase().as_str(), "true" | "1" | "yes" | "on")
            }
            Some(JsonValue::Number(value)) => value.as_i64().unwrap_or(0) != 0,
            _ => default,
        }
    }

    fn string_value(&self, key: &str) -> Option<String> {
        let value = self.values.get(key)?;
        let text = match value {
            JsonValue::String(value) => value.clone(),
            JsonValue::Bool(value) => value.to_string(),
            JsonValue::Number(value) => value.to_string(),
            _ => return None,
        };
        let text = text.trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn cache_hours(&self) -> i64 {
        self.string_value("location.geocoding_cache_hours")
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(720)
    }
}

fn config_path_candidates(db_path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(value) = env::var("CAPSULE_CONFIG_PATH") {
        let value = value.trim();
        if !value.is_empty() {
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

fn build_place_name(address: &JsonValue) -> String {
    let mut parts = Vec::new();
    if let Some(city) = address_value(
        address,
        &["city", "town", "village", "municipality", "suburb"],
    ) {
        parts.push(city);
    }
    let state = address_value(address, &["state", "province", "region"]);
    if let Some(state) = state.as_deref() {
        parts.push(us_state_abbrev(state).unwrap_or(state).to_string());
    }
    let country = address
        .get("country")
        .and_then(JsonValue::as_str)
        .map(str::to_string);
    let country_code = address
        .get("country_code")
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_uppercase();
    if let Some(country) = country {
        if country_code != "US" {
            parts.push(country);
        } else if state.is_none() && parts.is_empty() {
            parts.push("USA".to_string());
        }
    }
    if parts.is_empty() {
        "Unknown location".to_string()
    } else {
        parts.join(", ")
    }
}

fn build_precise_place_name(address: &JsonValue) -> String {
    let mut parts = Vec::new();
    if let Some(place) = address_value(address, &["road", "pedestrian", "neighbourhood", "suburb"])
    {
        push_unique_place_part(&mut parts, place);
    }
    if let Some(city) = address_value(address, &["city", "town", "village", "municipality"]) {
        push_unique_place_part(&mut parts, city);
    }
    if let Some(country) = address.get("country").and_then(JsonValue::as_str) {
        push_unique_place_part(&mut parts, country.to_string());
    }
    if parts.is_empty() {
        build_place_name(address)
    } else {
        parts.join(", ")
    }
}

fn push_unique_place_part(parts: &mut Vec<String>, value: String) {
    if !parts.iter().any(|part| part.eq_ignore_ascii_case(&value)) {
        parts.push(value);
    }
}

fn address_value(address: &JsonValue, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| address.get(*key).and_then(JsonValue::as_str))
        .map(str::to_string)
}

fn us_state_abbrev(value: &str) -> Option<&'static str> {
    match value {
        "Alabama" => Some("AL"),
        "Alaska" => Some("AK"),
        "Arizona" => Some("AZ"),
        "Arkansas" => Some("AR"),
        "California" => Some("CA"),
        "Colorado" => Some("CO"),
        "Connecticut" => Some("CT"),
        "Delaware" => Some("DE"),
        "Florida" => Some("FL"),
        "Georgia" => Some("GA"),
        "Hawaii" => Some("HI"),
        "Idaho" => Some("ID"),
        "Illinois" => Some("IL"),
        "Indiana" => Some("IN"),
        "Iowa" => Some("IA"),
        "Kansas" => Some("KS"),
        "Kentucky" => Some("KY"),
        "Louisiana" => Some("LA"),
        "Maine" => Some("ME"),
        "Maryland" => Some("MD"),
        "Massachusetts" => Some("MA"),
        "Michigan" => Some("MI"),
        "Minnesota" => Some("MN"),
        "Mississippi" => Some("MS"),
        "Missouri" => Some("MO"),
        "Montana" => Some("MT"),
        "Nebraska" => Some("NE"),
        "Nevada" => Some("NV"),
        "New Hampshire" => Some("NH"),
        "New Jersey" => Some("NJ"),
        "New Mexico" => Some("NM"),
        "New York" => Some("NY"),
        "North Carolina" => Some("NC"),
        "North Dakota" => Some("ND"),
        "Ohio" => Some("OH"),
        "Oklahoma" => Some("OK"),
        "Oregon" => Some("OR"),
        "Pennsylvania" => Some("PA"),
        "Rhode Island" => Some("RI"),
        "South Carolina" => Some("SC"),
        "South Dakota" => Some("SD"),
        "Tennessee" => Some("TN"),
        "Texas" => Some("TX"),
        "Utah" => Some("UT"),
        "Vermont" => Some("VT"),
        "Virginia" => Some("VA"),
        "Washington" => Some("WA"),
        "West Virginia" => Some("WV"),
        "Wisconsin" => Some("WI"),
        "Wyoming" => Some("WY"),
        "District of Columbia" => Some("DC"),
        _ => None,
    }
}

fn string_or_number(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn number_to_i64(value: Option<&JsonValue>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_f64().map(|item| item.round() as i64))
    })
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
