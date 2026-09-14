//! Pure provider request construction and response interpretation.
//!
//! No function in this module knows anything about journal text, tags or
//! titles.  Network access is supplied by [`crate::context::HttpClient`], so
//! tests can exercise malformed, delayed and cancelled responses without
//! touching the network.

use std::fmt;

use chrono::{DateTime, Duration as ChronoDuration, NaiveDateTime, Timelike, Utc};
use serde_json::{json, Value as JsonValue};
use url::Url;

use crate::context::{Cancellation, ContextDeadline, HttpClient, HttpRequest, HttpResponse};

pub const NOMINATIM_BASE_URL: &str = "https://nominatim.openstreetmap.org";
pub const NOMINATIM_USER_AGENT: &str = "CapsuleJournal/1.0 (https://github.com/capsule-journal)";
pub const IP_API_URL: &str = "http://ip-api.com/json";
pub const IPINFO_URL: &str = "https://ipinfo.io/json";
pub const OPEN_METEO_BASE_URL: &str = "https://api.open-meteo.com/v1";
pub const OPEN_METEO_ARCHIVE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";
pub const MET_NORWAY_URL: &str = "https://api.met.no/weatherapi/locationforecast/2.0/compact";
pub const MET_NORWAY_USER_AGENT: &str =
    "CapsuleExp/1.0 (https://github.com/soundtrackgeek/capsule_exp_ai)";

#[derive(Debug, Clone, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinates {
    pub fn new(latitude: f64, longitude: f64) -> Option<Self> {
        (latitude.is_finite()
            && longitude.is_finite()
            && (-90.0..=90.0).contains(&latitude)
            && (-180.0..=180.0).contains(&longitude))
        .then_some(Self {
            latitude,
            longitude,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeocodeData {
    pub coordinates: Coordinates,
    pub place_name: String,
    pub place_details: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeatherData {
    pub temp_c: Option<f64>,
    pub temp_f: Option<f64>,
    pub condition: Option<String>,
    pub icon: Option<String>,
    pub humidity: Option<i64>,
    pub wind_kph: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    Cancelled,
    DeadlineExceeded,
    Http(String),
    Malformed(String),
    Unavailable(String),
}

impl ProviderError {
    pub fn message(&self) -> String {
        match self {
            Self::Cancelled => "context capture cancelled".to_string(),
            Self::DeadlineExceeded => "context capture deadline exceeded".to_string(),
            Self::Http(message) | Self::Malformed(message) | Self::Unavailable(message) => {
                message.clone()
            }
        }
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for ProviderError {}

pub fn forward_geocode(
    http: &dyn HttpClient,
    place: &str,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<GeocodeData>, ProviderError> {
    let Some(url) = url_with_params(
        &format!("{NOMINATIM_BASE_URL}/search"),
        vec![
            ("q", place.to_string()),
            ("format", "json".to_string()),
            ("limit", "1".to_string()),
            ("addressdetails", "1".to_string()),
        ],
    ) else {
        return Err(ProviderError::Malformed(
            "invalid geocoding URL".to_string(),
        ));
    };
    let response = request_json(
        http,
        HttpRequest::new(url, vec![("User-Agent", NOMINATIM_USER_AGENT)]),
        deadline,
        cancellation,
    )?;
    let Some(first) = response.as_array().and_then(|items| items.first()) else {
        return Ok(None);
    };
    let Some(coordinates) = coordinates_from_json(first) else {
        return Err(ProviderError::Malformed(
            "geocoder returned invalid coordinates".to_string(),
        ));
    };
    let address = first.get("address").cloned().unwrap_or_else(|| json!({}));
    let place_name = first
        .get("display_name")
        .and_then(JsonValue::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| build_place_name(&address));
    let place_details = place_details(&address, first.get("display_name"));
    Ok(Some(GeocodeData {
        coordinates,
        place_name,
        place_details,
    }))
}

pub fn reverse_geocode(
    http: &dyn HttpClient,
    coordinates: &Coordinates,
    precise: bool,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<GeocodeData>, ProviderError> {
    let zoom = if precise { 16 } else { 10 };
    let Some(url) = url_with_params(
        &format!("{NOMINATIM_BASE_URL}/reverse"),
        vec![
            ("lat", coordinates.latitude.to_string()),
            ("lon", coordinates.longitude.to_string()),
            ("format", "json".to_string()),
            ("addressdetails", "1".to_string()),
            ("zoom", zoom.to_string()),
            ("accept-language", "en".to_string()),
        ],
    ) else {
        return Err(ProviderError::Malformed(
            "invalid reverse-geocoding URL".to_string(),
        ));
    };
    let response = request_json(
        http,
        HttpRequest::new(url, vec![("User-Agent", NOMINATIM_USER_AGENT)]),
        deadline,
        cancellation,
    )?;
    if response.get("error").is_some() {
        return Ok(None);
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
    Ok(Some(GeocodeData {
        coordinates: coordinates.clone(),
        place_name,
        place_details: place_details(&address, response.get("display_name")),
    }))
}

pub fn location_from_ip(
    http: &dyn HttpClient,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<Coordinates>, ProviderError> {
    // The second service is an intentional fallback, not a second weather
    // attempt.  Both requests share the same deadline and cancellation token.
    match ip_api_location(http, deadline, cancellation) {
        Ok(Some(coordinates)) => Ok(Some(coordinates)),
        Ok(None) | Err(ProviderError::Http(_)) | Err(ProviderError::Malformed(_)) => {
            if deadline.expired() {
                return Err(ProviderError::DeadlineExceeded);
            }
            ipinfo_location(http, deadline, cancellation)
        }
        Err(error) => Err(error),
    }
}

fn ip_api_location(
    http: &dyn HttpClient,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<Coordinates>, ProviderError> {
    let Some(url) = url_with_params(IP_API_URL, vec![("fields", "status,lat,lon".to_string())])
    else {
        return Err(ProviderError::Malformed(
            "invalid IP location URL".to_string(),
        ));
    };
    let response = request_json(
        http,
        HttpRequest::new(url, Vec::new()),
        deadline,
        cancellation,
    )?;
    if response.get("status").and_then(JsonValue::as_str) != Some("success") {
        return Ok(None);
    }
    Ok(coordinates_from_json(&response))
}

fn ipinfo_location(
    http: &dyn HttpClient,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<Coordinates>, ProviderError> {
    let response = request_json(
        http,
        HttpRequest::new(IPINFO_URL.to_string(), Vec::new()),
        deadline,
        cancellation,
    )?;
    let Some(loc) = response.get("loc").and_then(JsonValue::as_str) else {
        return Ok(None);
    };
    let Some((latitude, longitude)) = loc.split_once(',') else {
        return Err(ProviderError::Malformed(
            "IP location returned an invalid loc value".to_string(),
        ));
    };
    let Ok(latitude) = latitude.trim().parse::<f64>() else {
        return Err(ProviderError::Malformed(
            "IP location returned an invalid latitude".to_string(),
        ));
    };
    let Ok(longitude) = longitude.trim().parse::<f64>() else {
        return Err(ProviderError::Malformed(
            "IP location returned an invalid longitude".to_string(),
        ));
    };
    Ok(Coordinates::new(latitude, longitude))
}

pub fn weather(
    http: &dyn HttpClient,
    provider: &str,
    coordinates: &Coordinates,
    entry_created_at: Option<&str>,
    now: DateTime<Utc>,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<WeatherData>, ProviderError> {
    match normalize_provider(provider).as_str() {
        "met_norway" => met_norway_weather(http, coordinates, deadline, cancellation),
        _ => open_meteo_weather(
            http,
            coordinates,
            entry_created_at,
            now,
            deadline,
            cancellation,
        ),
    }
}

pub fn normalize_provider(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "met" | "met.no" | "met_norway" | "met-norway" => "met_norway".to_string(),
        _ => "open_meteo".to_string(),
    }
}

fn open_meteo_weather(
    http: &dyn HttpClient,
    coordinates: &Coordinates,
    entry_created_at: Option<&str>,
    now: DateTime<Utc>,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<WeatherData>, ProviderError> {
    let target = entry_created_at.and_then(parse_entry_time);
    if target.is_some_and(|target| {
        now.naive_utc().signed_duration_since(target) > ChronoDuration::hours(3)
    }) {
        return open_meteo_historical_weather(
            http,
            coordinates,
            target.expect("checked above"),
            now,
            deadline,
            cancellation,
        );
    }
    open_meteo_current_weather(http, coordinates, deadline, cancellation)
}

fn open_meteo_historical_weather(
    http: &dyn HttpClient,
    coordinates: &Coordinates,
    mut target: NaiveDateTime,
    now: DateTime<Utc>,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<WeatherData>, ProviderError> {
    if target.minute() >= 30 {
        target += ChronoDuration::hours(1);
    }
    target = target
        .with_minute(0)
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .ok_or_else(|| ProviderError::Malformed("invalid historical weather time".to_string()))?;
    let target_date = target.format("%Y-%m-%d").to_string();
    let target_hour = target.format("%Y-%m-%dT%H:00").to_string();
    let days_diff = now
        .date_naive()
        .signed_duration_since(target.date())
        .num_days();
    let base_url = if days_diff >= 90 {
        OPEN_METEO_ARCHIVE_URL.to_string()
    } else {
        format!("{OPEN_METEO_BASE_URL}/forecast")
    };
    let Some(url) = url_with_params(
        &base_url,
        vec![
            ("latitude", round4(coordinates.latitude).to_string()),
            ("longitude", round4(coordinates.longitude).to_string()),
            ("start_date", target_date.clone()),
            ("end_date", target_date),
            (
                "hourly",
                "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,is_day"
                    .to_string(),
            ),
            ("timezone", "auto".to_string()),
        ],
    ) else {
        return Err(ProviderError::Malformed(
            "invalid historical weather URL".to_string(),
        ));
    };
    let response = request_json(
        http,
        HttpRequest::new(url, Vec::new()),
        deadline,
        cancellation,
    )?;
    let Some(hourly) = response.get("hourly") else {
        return Ok(None);
    };
    let Some(times) = hourly.get("time").and_then(JsonValue::as_array) else {
        return Err(ProviderError::Malformed(
            "Open-Meteo historical response has no time series".to_string(),
        ));
    };
    let index = times
        .iter()
        .position(|value| value.as_str() == Some(target_hour.as_str()))
        .unwrap_or(0);
    let Some(temp_c) = hourly_value(hourly, "temperature_2m", index).and_then(number_to_f64) else {
        return Ok(None);
    };
    let weather_code = hourly_value(hourly, "weather_code", index)
        .and_then(number_to_i64)
        .unwrap_or(0);
    let humidity = hourly_value(hourly, "relative_humidity_2m", index).and_then(number_to_i64);
    let wind_kph = hourly_value(hourly, "wind_speed_10m", index).and_then(number_to_f64);
    let is_day = hourly_value(hourly, "is_day", index)
        .and_then(number_to_i64)
        .unwrap_or(1);
    Ok(Some(weather_data(
        temp_c,
        weather_code,
        is_day,
        humidity,
        wind_kph,
    )))
}

fn open_meteo_current_weather(
    http: &dyn HttpClient,
    coordinates: &Coordinates,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<WeatherData>, ProviderError> {
    let Some(url) = url_with_params(
        &format!("{OPEN_METEO_BASE_URL}/forecast"),
        vec![
            ("latitude", round4(coordinates.latitude).to_string()),
            ("longitude", round4(coordinates.longitude).to_string()),
            (
                "current",
                "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,is_day"
                    .to_string(),
            ),
            ("timezone", "auto".to_string()),
        ],
    ) else {
        return Err(ProviderError::Malformed(
            "invalid current weather URL".to_string(),
        ));
    };
    let response = request_json(
        http,
        HttpRequest::new(url, Vec::new()),
        deadline,
        cancellation,
    )?;
    let Some(current) = response.get("current") else {
        return Ok(None);
    };
    let Some(temp_c) = current.get("temperature_2m").and_then(number_to_f64) else {
        return Ok(None);
    };
    let weather_code = current
        .get("weather_code")
        .and_then(number_to_i64)
        .unwrap_or(0);
    let is_day = current.get("is_day").and_then(number_to_i64).unwrap_or(1);
    Ok(Some(weather_data(
        temp_c,
        weather_code,
        is_day,
        current.get("relative_humidity_2m").and_then(number_to_i64),
        current.get("wind_speed_10m").and_then(number_to_f64),
    )))
}

fn met_norway_weather(
    http: &dyn HttpClient,
    coordinates: &Coordinates,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<WeatherData>, ProviderError> {
    let Some(url) = url_with_params(
        MET_NORWAY_URL,
        vec![
            ("lat", round4(coordinates.latitude).to_string()),
            ("lon", round4(coordinates.longitude).to_string()),
        ],
    ) else {
        return Err(ProviderError::Malformed(
            "invalid MET Norway URL".to_string(),
        ));
    };
    let response = request_json(
        http,
        HttpRequest::new(url, vec![("User-Agent", MET_NORWAY_USER_AGENT)]),
        deadline,
        cancellation,
    )?;
    let Some(first) = response
        .get("properties")
        .and_then(|value| value.get("timeseries"))
        .and_then(JsonValue::as_array)
        .and_then(|items| items.first())
    else {
        return Ok(None);
    };
    let Some(instant) = first
        .get("data")
        .and_then(|value| value.get("instant"))
        .and_then(|value| value.get("details"))
    else {
        return Ok(None);
    };
    let Some(temp_c) = instant.get("air_temperature").and_then(number_to_f64) else {
        return Ok(None);
    };
    let humidity = instant.get("relative_humidity").and_then(number_to_i64);
    let wind_kph = instant
        .get("wind_speed")
        .and_then(number_to_f64)
        .map(|value| round1(value * 3.6));
    let symbol_code = first
        .get("data")
        .and_then(|value| value.get("next_1_hours"))
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
    Ok(Some(WeatherData {
        temp_c: Some(round1(temp_c)),
        temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
        condition: Some(met_norway_condition(base_symbol)),
        icon: Some(icon),
        humidity,
        wind_kph,
    }))
}

fn weather_data(
    temp_c: f64,
    weather_code: i64,
    is_day: i64,
    humidity: Option<i64>,
    wind_kph: Option<f64>,
) -> WeatherData {
    let (condition, icon) = open_meteo_code(weather_code, is_day);
    WeatherData {
        temp_c: Some(round1(temp_c)),
        temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
        condition: Some(condition.to_string()),
        icon: Some(icon.to_string()),
        humidity,
        wind_kph: wind_kph.map(round1),
    }
}

fn request_json(
    http: &dyn HttpClient,
    request: HttpRequest,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<JsonValue, ProviderError> {
    if cancellation.is_cancelled() {
        return Err(ProviderError::Cancelled);
    }
    let timeout = deadline
        .remaining()
        .ok_or(ProviderError::DeadlineExceeded)?;
    let response: HttpResponse = http.get(request, timeout).map_err(ProviderError::Http)?;
    if cancellation.is_cancelled() {
        return Err(ProviderError::Cancelled);
    }
    if deadline.expired() {
        return Err(ProviderError::DeadlineExceeded);
    }
    if !(200..300).contains(&response.status) {
        return Err(ProviderError::Http(format!(
            "provider returned HTTP status {}",
            response.status
        )));
    }
    serde_json::from_slice(&response.body)
        .map_err(|error| ProviderError::Malformed(format!("provider JSON was invalid: {error}")))
}

fn coordinates_from_json(value: &JsonValue) -> Option<Coordinates> {
    let latitude = value.get("lat").and_then(number_to_f64)?;
    let longitude = value
        .get("lon")
        .or_else(|| value.get("lng"))
        .and_then(number_to_f64)?;
    Coordinates::new(latitude, longitude)
}

fn place_details(address: &JsonValue, display_name: Option<&JsonValue>) -> Option<String> {
    let details = json!({
        "city": address_value(address, &["city", "town", "village", "municipality"]),
        "state": address_value(address, &["state", "province", "region"]),
        "country": address.get("country").and_then(JsonValue::as_str),
        "country_code": address
            .get("country_code")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_uppercase(),
        "display_name": display_name.and_then(JsonValue::as_str).unwrap_or_default(),
        "raw": address,
    });
    Some(details.to_string())
}

pub(crate) fn open_meteo_code(code: i64, is_day: i64) -> (&'static str, &'static str) {
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

pub(crate) fn parse_entry_time(value: &str) -> Option<NaiveDateTime> {
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
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.naive_local())
}

pub(crate) fn build_place_name(address: &JsonValue) -> String {
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

pub(crate) fn build_precise_place_name(address: &JsonValue) -> String {
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

fn hourly_value<'a>(hourly: &'a JsonValue, key: &str, index: usize) -> Option<&'a JsonValue> {
    hourly.get(key)?.as_array()?.get(index)
}

fn number_to_f64(value: &JsonValue) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|item| item.parse::<f64>().ok()))
        .filter(|value| value.is_finite())
}

fn number_to_i64(value: &JsonValue) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| number_to_f64(value).map(|item| item.round() as i64))
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

fn url_with_params(base_url: &str, params: Vec<(&str, String)>) -> Option<String> {
    let mut url = Url::parse(base_url).ok()?;
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, &value);
        }
    }
    Some(url.to_string())
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
    fn coordinates_reject_nan_and_out_of_range_values() {
        assert!(Coordinates::new(f64::NAN, 0.0).is_none());
        assert!(Coordinates::new(91.0, 0.0).is_none());
        assert!(Coordinates::new(69.65, 18.96).is_some());
    }
}
