//! Bounded location and weather capture shared by Capsule clients.
//!
//! Provider collection is deliberately separated from SQLite persistence.  A
//! capture can therefore return an immediate, structured unavailable result
//! when a provider is slow or malformed, while a confirmed entry remains
//! committed.  HTTP, time, cancellation and optional weather-cache behavior
//! are injected for deterministic CLI and desktop tests.

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use reqwest::blocking::Client;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::{
    backup,
    contracts::{ContextLocation, ContextPolicy, ContextResult, ContextStatus, WeatherObservation},
    location, providers,
};

const MAX_CONTEXT_DEADLINE_MS: u64 = 8_000;
const WEATHER_CACHE_MAX_AGE_SECONDS: i64 = 15 * 60;

/// A GET request handed to an injected HTTP implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

impl HttpRequest {
    pub fn new(url: impl Into<String>, headers: Vec<(&str, &str)>) -> Self {
        Self {
            url: url.into(),
            headers: headers
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }
}

/// The minimum response surface needed by the existing JSON providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Injectable HTTP transport.  Implementations must honor `timeout` and
/// should return promptly when the deadline has elapsed.
pub trait HttpClient: Send + Sync {
    fn get(
        &self,
        request: HttpRequest,
        timeout: Duration,
    ) -> std::result::Result<HttpResponse, String>;
}

/// Production blocking transport.  Each request receives the remaining
/// invocation budget rather than the old independent ten-second timeout.
#[derive(Clone)]
pub struct ReqwestHttpClient {
    client: Client,
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        // Building a client cannot perform network I/O.  Keep a fallible
        // constructor for callers that want to surface unusual TLS failures,
        // while the default remains convenient for desktop adapters.
        let client = Client::builder().build().unwrap_or_else(|_| Client::new());
        Self { client }
    }
}

impl ReqwestHttpClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .build()
                .context("failed to build context HTTP client")?,
        })
    }
}

impl HttpClient for ReqwestHttpClient {
    fn get(
        &self,
        request: HttpRequest,
        timeout: Duration,
    ) -> std::result::Result<HttpResponse, String> {
        let mut builder = self.client.get(request.url).timeout(timeout);
        for (key, value) in request.headers {
            builder = builder.header(key, value);
        }
        let response = builder.send().map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        let body = response
            .bytes()
            .map_err(|error| error.to_string())?
            .to_vec();
        Ok(HttpResponse { status, body })
    }
}

/// Clock abstraction used for both timestamps and the monotonic deadline.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn monotonic_ms(&self) -> u64;
}

#[derive(Debug)]
pub struct SystemClock {
    started: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn monotonic_ms(&self) -> u64 {
        self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }
}

/// A deterministic clock useful to provider fixtures and integration tests.
#[derive(Debug)]
pub struct ManualClock {
    now: Mutex<DateTime<Utc>>,
    monotonic: AtomicU64,
}

impl ManualClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
            monotonic: AtomicU64::new(0),
        }
    }

    pub fn advance(&self, duration: Duration) {
        let millis = duration.as_millis().min(u128::from(u64::MAX)) as u64;
        self.monotonic.fetch_add(millis, Ordering::Relaxed);
        if let Ok(mut now) = self.now.lock() {
            *now += ChronoDuration::milliseconds(millis as i64);
        }
    }
}

impl Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
            .lock()
            .map(|value| *value)
            .unwrap_or_else(|_| Utc::now())
    }

    fn monotonic_ms(&self) -> u64 {
        self.monotonic.load(Ordering::Relaxed)
    }
}

/// Cancellation abstraction checked before and after every provider request
/// and before persistence.
pub trait Cancellation: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct NeverCancel;

impl Cancellation for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug, Default)]
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

impl Cancellation for CancellationToken {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Exact resolved-place/provider key supplied to an optional weather cache.
#[derive(Debug, Clone)]
pub struct WeatherCacheKey {
    pub latitude: f64,
    pub longitude: f64,
    pub provider: String,
}

impl WeatherCacheKey {
    pub fn new(latitude: f64, longitude: f64, provider: impl Into<String>) -> Self {
        Self {
            latitude,
            longitude,
            provider: provider.into(),
        }
    }

    pub fn place_key(&self) -> String {
        format!("{:.8},{:.8}", self.latitude, self.longitude)
    }
}

impl PartialEq for WeatherCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.latitude.to_bits() == other.latitude.to_bits()
            && self.longitude.to_bits() == other.longitude.to_bits()
            && self.provider == other.provider
    }
}

impl Eq for WeatherCacheKey {}

impl Hash for WeatherCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.latitude.to_bits().hash(state);
        self.longitude.to_bits().hash(state);
        self.provider.hash(state);
    }
}

/// Optional cache supplied by `cap` or another client.  The service applies a
/// hard fifteen-minute freshness limit even when an implementation returns an
/// older observation.
pub trait ContextCache: Send + Sync {
    fn get(&self, key: &WeatherCacheKey, now: DateTime<Utc>) -> Option<WeatherObservation>;
    fn put(&self, key: &WeatherCacheKey, observation: &WeatherObservation);
}

#[derive(Debug, Default)]
pub struct NoopContextCache;

impl ContextCache for NoopContextCache {
    fn get(&self, _key: &WeatherCacheKey, _now: DateTime<Utc>) -> Option<WeatherObservation> {
        None
    }

    fn put(&self, _key: &WeatherCacheKey, _observation: &WeatherObservation) {}
}

#[derive(Debug, Default)]
pub struct MemoryContextCache {
    values: Mutex<HashMap<WeatherCacheKey, WeatherObservation>>,
}

impl MemoryContextCache {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ContextCache for MemoryContextCache {
    fn get(&self, key: &WeatherCacheKey, now: DateTime<Utc>) -> Option<WeatherObservation> {
        let value = self.values.lock().ok()?.get(key).cloned()?;
        let fetched_at = value.fetched_at?;
        let age = now.signed_duration_since(fetched_at).num_seconds();
        (0..=WEATHER_CACHE_MAX_AGE_SECONDS)
            .contains(&age)
            .then_some(value)
    }

    fn put(&self, key: &WeatherCacheKey, observation: &WeatherObservation) {
        if let Ok(mut values) = self.values.lock() {
            values.insert(key.clone(), observation.clone());
        }
    }
}

/// Dependencies for one context invocation.  All fields are shared so the
/// same fake transport/clock/cache can be observed across provider calls.
#[derive(Clone)]
pub struct ContextDependencies {
    pub http: Arc<dyn HttpClient>,
    pub clock: Arc<dyn Clock>,
    pub cache: Arc<dyn ContextCache>,
    pub cancellation: Arc<dyn Cancellation>,
}

impl Default for ContextDependencies {
    fn default() -> Self {
        Self {
            http: Arc::new(ReqwestHttpClient::default()),
            clock: Arc::new(SystemClock::default()),
            cache: Arc::new(NoopContextCache),
            cancellation: Arc::new(NeverCancel),
        }
    }
}

/// The independent operation requested by a client.
#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub db_path: PathBuf,
    pub entry_uuid: String,
    pub policy: ContextPolicy,
    /// Optional identity captured before provider work.  When present, the
    /// service revalidates it immediately before persistence so a same-path
    /// database replacement cannot receive context for the wrong entry.
    pub database_identity: Option<crate::db::FileIdentity>,
    pub skip: bool,
    pub config_valid: bool,
    pub enrich: bool,
    /// Optional immutable backup destination/retention snapshot.  Explicit
    /// enrichment uses this policy rather than consulting process settings
    /// after provider work has started.
    pub backup_policy: Option<crate::contracts::BackupPolicy>,
}

impl ContextRequest {
    pub fn capture(db_path: &Path, entry_uuid: &str, mut policy: ContextPolicy) -> Self {
        policy.database_path = db_path.to_path_buf();
        Self {
            db_path: db_path.to_path_buf(),
            entry_uuid: entry_uuid.to_string(),
            policy,
            database_identity: db_path
                .exists()
                .then(|| crate::db::FileIdentity::for_path(db_path)),
            skip: false,
            config_valid: true,
            enrich: false,
            backup_policy: None,
        }
    }

    pub fn enrich(db_path: &Path, entry_uuid: &str, policy: ContextPolicy) -> Self {
        let mut request = Self::capture(db_path, entry_uuid, policy);
        request.enrich = true;
        request
    }

    pub fn without_context(mut self) -> Self {
        self.skip = true;
        self
    }

    pub fn with_backup_policy(mut self, policy: crate::contracts::BackupPolicy) -> Self {
        self.backup_policy = Some(policy);
        self
    }
}

/// Additional cache/provider metadata that does not fit the frozen
/// renderer-facing `ContextResult` DTO.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextDetails {
    pub weather_provider: Option<String>,
    pub weather_place_key: Option<String>,
    pub weather_age_seconds: Option<i64>,
    pub weather_from_cache: bool,
    pub location_from_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextReport {
    pub result: ContextResult,
    pub details: ContextDetails,
    #[serde(skip)]
    pub(crate) location_cache_write: Option<providers::GeocodeData>,
}

impl ContextReport {
    fn skipped(result: ContextResult) -> Self {
        Self {
            result,
            details: ContextDetails {
                weather_provider: None,
                weather_place_key: None,
                weather_age_seconds: None,
                weather_from_cache: false,
                location_from_cache: false,
            },
            location_cache_write: None,
        }
    }
}

/// Provider work prepared for a short, caller-controlled persistence phase.
///
/// `prepare` performs all database reads and network/cache resolution without
/// holding a backup or mutation lock.  A desktop or CLI adapter can then
/// acquire its own bounded guard and pass this value to `persist_prepared`.
/// The private snapshot prevents callers from accidentally changing the
/// merge inputs between phases.
pub struct ContextPreparation {
    report: ContextReport,
    existing: Option<PersistedLocation>,
    deadline: ContextDeadline,
    persistable: bool,
    db_path: PathBuf,
    entry_uuid: String,
    database_identity: Option<crate::db::FileIdentity>,
    enrich: bool,
    backup_policy: Option<crate::contracts::BackupPolicy>,
}

impl ContextPreparation {
    pub fn report(&self) -> &ContextReport {
        &self.report
    }

    pub fn remaining(&self) -> Option<Duration> {
        self.deadline.remaining()
    }

    pub fn is_persistable(&self) -> bool {
        self.persistable
    }

    pub fn into_report(self) -> ContextReport {
        self.report
    }

    fn finished(report: ContextReport, clock: Arc<dyn Clock>) -> Self {
        Self {
            report,
            existing: None,
            deadline: ContextDeadline::new(clock, 0),
            persistable: false,
            db_path: PathBuf::new(),
            entry_uuid: String::new(),
            database_identity: None,
            enrich: false,
            backup_policy: None,
        }
    }

    fn is_bound_to(&self, request: &ContextRequest) -> bool {
        self.db_path == request.db_path
            && self.entry_uuid == request.entry_uuid
            && self.enrich == request.enrich
            && self.database_identity == request.database_identity
            && self.backup_policy == request.backup_policy
    }
}

/// Shared context service.  The service never emits output and never sends
/// journal content to a provider.
#[derive(Clone)]
pub struct ContextService {
    dependencies: ContextDependencies,
}

impl Default for ContextService {
    fn default() -> Self {
        Self::new(ContextDependencies::default())
    }
}

impl ContextService {
    pub fn new(dependencies: ContextDependencies) -> Self {
        Self { dependencies }
    }

    pub fn dependencies(&self) -> &ContextDependencies {
        &self.dependencies
    }

    pub fn capture(&self, request: &ContextRequest) -> Result<ContextResult> {
        Ok(self.capture_report(request)?.result)
    }

    pub fn capture_report(&self, request: &ContextRequest) -> Result<ContextReport> {
        self.run(request)
    }

    pub fn enrich(&self, request: &ContextRequest) -> Result<ContextResult> {
        Ok(self.run(request)?.result)
    }

    fn run(&self, request: &ContextRequest) -> Result<ContextReport> {
        let preparation = self.prepare(request)?;
        self.persist_prepared(request, preparation)
    }

    /// Resolve location/weather without holding a backup or mutation lock.
    /// Call [`Self::persist_prepared`] after acquiring the caller's short-lived
    /// persistence guard.
    pub fn prepare(&self, request: &ContextRequest) -> Result<ContextPreparation> {
        let mut result = ContextResult::default();
        if request.skip {
            result.location_status = ContextStatus::Skipped;
            result.weather_status = ContextStatus::Skipped;
            return Ok(ContextPreparation::finished(
                ContextReport::skipped(result),
                self.dependencies.clock.clone(),
            ));
        }
        if !request.config_valid {
            result.location_status = ContextStatus::Unavailable;
            result.weather_status = ContextStatus::Unavailable;
            result
                .warnings
                .push("location configuration is invalid; context skipped".to_string());
            return Ok(ContextPreparation::finished(
                ContextReport::skipped(result),
                self.dependencies.clock.clone(),
            ));
        }
        if !request.policy.auto_capture {
            result.location_status = ContextStatus::Disabled;
            result.weather_status = ContextStatus::Disabled;
            return Ok(ContextPreparation::finished(
                ContextReport::skipped(result),
                self.dependencies.clock.clone(),
            ));
        }

        let deadline = ContextDeadline::new(
            self.dependencies.clock.clone(),
            request.policy.deadline_ms.min(MAX_CONTEXT_DEADLINE_MS),
        );
        if self.dependencies.cancellation.is_cancelled() {
            result.location_status = ContextStatus::Skipped;
            result.weather_status = ContextStatus::Skipped;
            result
                .warnings
                .push("context capture cancelled".to_string());
            return Ok(ContextPreparation::finished(
                ContextReport::skipped(result),
                self.dependencies.clock.clone(),
            ));
        }

        if !database_identity_matches(request) {
            result.location_status = ContextStatus::Unavailable;
            result.weather_status = ContextStatus::Unavailable;
            result
                .warnings
                .push("database changed before context capture".to_string());
            return Ok(ContextPreparation::finished(
                ContextReport::skipped(result),
                self.dependencies.clock.clone(),
            ));
        }

        let Some(timeout) = deadline.remaining() else {
            result.location_status = ContextStatus::Unavailable;
            result.weather_status = ContextStatus::Unavailable;
            result
                .warnings
                .push("context capture deadline exceeded".to_string());
            return Ok(ContextPreparation::finished(
                ContextReport::skipped(result),
                self.dependencies.clock.clone(),
            ));
        };
        let Some(entry_created_at) =
            read_entry_created_at(&request.db_path, &request.entry_uuid, timeout)?
        else {
            result.location_status = ContextStatus::Unavailable;
            result.weather_status = ContextStatus::Unavailable;
            result
                .warnings
                .push("entry was deleted before context capture".to_string());
            return Ok(ContextPreparation::finished(
                ContextReport::skipped(result),
                self.dependencies.clock.clone(),
            ));
        };

        // Read the current location row for both capture and enrichment.  A
        // concurrent desktop edit/manual row must be merged rather than
        // overwritten after provider work completes.
        let existing = load_existing_location(
            &request.db_path,
            &request.entry_uuid,
            deadline.remaining().unwrap_or_default(),
        )?;

        let mut report = self.collect(request, &entry_created_at, existing.as_ref(), &deadline);
        if self.dependencies.cancellation.is_cancelled() {
            report
                .result
                .warnings
                .push("context capture cancelled".to_string());
            report.result.location_status = ContextStatus::Skipped;
            report.result.weather_status = ContextStatus::Skipped;
            return Ok(ContextPreparation::finished(
                report,
                self.dependencies.clock.clone(),
            ));
        }

        if deadline.expired() {
            report
                .result
                .warnings
                .push("context capture deadline exceeded".to_string());
            report.result.location = None;
            report.result.weather = None;
            report.result.location_status = ContextStatus::Unavailable;
            report.result.weather_status = ContextStatus::Unavailable;
            return Ok(ContextPreparation::finished(
                report,
                self.dependencies.clock.clone(),
            ));
        }

        if !database_identity_matches(request) {
            report
                .result
                .warnings
                .push("database changed while resolving context".to_string());
            report.result.location = None;
            report.result.weather = None;
            report.result.location_status = ContextStatus::Unavailable;
            report.result.weather_status = ContextStatus::Unavailable;
            return Ok(ContextPreparation::finished(
                report,
                self.dependencies.clock.clone(),
            ));
        }

        Ok(ContextPreparation {
            persistable: report.result.location.is_some() || report.result.weather.is_some(),
            report,
            existing,
            deadline,
            db_path: request.db_path.clone(),
            entry_uuid: request.entry_uuid.clone(),
            database_identity: request.database_identity.clone(),
            enrich: request.enrich,
            backup_policy: request.backup_policy.clone(),
        })
    }

    /// Persist a previously prepared report while the caller owns the
    /// database's short-lived mutation/backup guard.
    pub fn persist_prepared(
        &self,
        request: &ContextRequest,
        mut preparation: ContextPreparation,
    ) -> Result<ContextReport> {
        if !preparation.persistable {
            return Ok(preparation.report);
        }
        if !preparation.is_bound_to(request) {
            clear_unpersisted_context(
                &mut preparation.report,
                "context preparation does not match the persistence request",
            );
            return Ok(preparation.report);
        }
        if self.dependencies.cancellation.is_cancelled() {
            preparation.report.result.location = None;
            preparation.report.result.weather = None;
            preparation.report.result.location_status = ContextStatus::Skipped;
            preparation.report.result.weather_status = ContextStatus::Skipped;
            preparation
                .report
                .result
                .warnings
                .push("context capture cancelled".to_string());
            return Ok(preparation.report);
        }
        if preparation.deadline.expired() {
            clear_unpersisted_context(&mut preparation.report, "context capture deadline exceeded");
            return Ok(preparation.report);
        }
        if !database_identity_matches(request) {
            clear_unpersisted_context(
                &mut preparation.report,
                "database changed before context persistence",
            );
            return Ok(preparation.report);
        }

        match persist_context(
            &request.db_path,
            &request.entry_uuid,
            request.enrich,
            &preparation.report,
            preparation.existing.as_ref(),
            &preparation.deadline,
            self.dependencies.cancellation.as_ref(),
        ) {
            Ok(Some(outcome)) => {
                preparation.report.result.persisted_at = Some(outcome.persisted_at);
                if preparation.report.result.location.is_some() {
                    preparation.report.result.location = Some(outcome.location);
                }
                if preparation.report.result.weather.is_some() {
                    preparation.report.result.weather = outcome.weather;
                    if preparation.report.result.weather.is_none() {
                        preparation.report.result.weather_status = ContextStatus::Unavailable;
                        if outcome.weather_discarded_due_to_coordinate_change {
                            preparation.report.result.warnings.push(
                                "weather was not attached because the saved coordinates changed"
                                    .to_string(),
                            );
                        } else {
                            preparation
                                .report
                                .result
                                .warnings
                                .push("persisted weather was incomplete".to_string());
                        }
                    }
                }
            }
            Ok(None) => clear_unpersisted_context(
                &mut preparation.report,
                "context fields were not persisted",
            ),
            Err(error) => clear_unpersisted_context(
                &mut preparation.report,
                &format!("context persistence unavailable: {error}"),
            ),
        }
        Ok(preparation.report)
    }

    fn collect(
        &self,
        request: &ContextRequest,
        entry_created_at: &str,
        existing: Option<&PersistedLocation>,
        deadline: &ContextDeadline,
    ) -> ContextReport {
        let mut result = ContextResult::default();
        let fixture = location::test_auto_capture_fixture();

        let mut location_from_cache = false;
        let location_candidate = if request.enrich {
            // Enrichment never changes coordinates/source and only asks a
            // provider for fields that are missing.
            if let Some(row) = existing {
                let needs_place = location::is_blank(row.place_name.as_deref());
                if let Some(place_name) =
                    fixture.as_ref().and_then(|value| value.place_name.clone())
                {
                    Some(LocationCandidate {
                        latitude: row.latitude,
                        longitude: row.longitude,
                        place_name: Some(place_name),
                        place_details: row.place_details.clone(),
                        source: row.source.clone(),
                        from_cache: false,
                        cache_write: None,
                    })
                } else if !needs_place {
                    // Keep coordinates available for weather enrichment while
                    // result.location remains unchanged below.
                    Some(LocationCandidate {
                        latitude: row.latitude,
                        longitude: row.longitude,
                        place_name: row.place_name.clone(),
                        place_details: row.place_details.clone(),
                        source: row.source.clone(),
                        from_cache: false,
                        cache_write: None,
                    })
                } else {
                    self.resolve_existing_place(
                        row,
                        &request.policy,
                        deadline,
                        &mut location_from_cache,
                        &mut result.warnings,
                    )
                }
            } else {
                self.resolve_location(
                    &request.policy,
                    deadline,
                    &mut location_from_cache,
                    &mut result.warnings,
                )
            }
        } else if let Some(fixture) = fixture.as_ref() {
            providers::Coordinates::new(fixture.latitude, fixture.longitude).map(|coordinates| {
                LocationCandidate {
                    latitude: coordinates.latitude,
                    longitude: coordinates.longitude,
                    place_name: fixture.place_name.clone(),
                    place_details: None,
                    source: fixture.source.clone(),
                    from_cache: false,
                    cache_write: None,
                }
            })
        } else {
            self.resolve_location(
                &request.policy,
                deadline,
                &mut location_from_cache,
                &mut result.warnings,
            )
        };

        if let Some(candidate) = location_candidate.as_ref() {
            let include_location = !request.enrich
                || existing.is_none()
                || (existing.is_some_and(|row| location::is_blank(row.place_name.as_deref()))
                    && candidate.place_name.is_some());
            if include_location {
                result.location_status = if candidate.from_cache {
                    ContextStatus::Cached
                } else {
                    ContextStatus::Captured
                };
                result.location = Some(ContextLocation {
                    latitude: candidate.latitude,
                    longitude: candidate.longitude,
                    place_name: candidate.place_name.clone(),
                    place_details: candidate.place_details.clone(),
                    source: Some(candidate.source.clone()),
                });
            } else {
                result.location_status = ContextStatus::Skipped;
            }
        } else if !request.enrich {
            result.location_status = ContextStatus::Unavailable;
        } else {
            result.location_status = ContextStatus::Skipped;
        }

        let weather_provider = providers::normalize_provider(
            request
                .policy
                .weather_provider
                .as_deref()
                .unwrap_or("open_meteo"),
        );
        let mut weather_from_cache = false;
        let mut weather_age_seconds = None;
        let weather = if let Some(candidate) = location_candidate.as_ref() {
            self.resolve_weather(
                &request.policy,
                &weather_provider,
                candidate,
                entry_created_at,
                existing,
                deadline,
                &mut weather_from_cache,
                &mut weather_age_seconds,
                &mut result.warnings,
            )
        } else {
            None
        };

        if let Some(observation) = weather {
            result.weather_status = if weather_from_cache {
                ContextStatus::Cached
            } else {
                ContextStatus::Captured
            };
            result.weather = Some(observation);
        } else if request.enrich && existing.is_some_and(|row| row.weather_complete()) {
            result.weather_status = ContextStatus::Skipped;
        } else {
            result.weather_status = ContextStatus::Unavailable;
        }

        let location_cache_write = location_candidate
            .as_ref()
            .and_then(|candidate| candidate.cache_write.clone());
        ContextReport {
            result,
            details: ContextDetails {
                weather_provider: Some(weather_provider),
                weather_place_key: location_candidate.as_ref().map(|candidate| {
                    WeatherCacheKey::new(candidate.latitude, candidate.longitude, "").place_key()
                }),
                weather_age_seconds,
                weather_from_cache,
                location_from_cache,
            },
            location_cache_write,
        }
    }

    fn resolve_location(
        &self,
        policy: &ContextPolicy,
        deadline: &ContextDeadline,
        location_from_cache: &mut bool,
        warnings: &mut Vec<String>,
    ) -> Option<LocationCandidate> {
        if policy.use_default_location {
            let place = policy.default_location_name.as_deref()?.trim();
            if place.is_empty() || !policy.allow_network {
                if !policy.allow_network {
                    warnings.push("configured location unavailable while offline".to_string());
                }
                return None;
            }
            return match providers::forward_geocode(
                self.dependencies.http.as_ref(),
                place,
                deadline,
                self.dependencies.cancellation.as_ref(),
            ) {
                Ok(Some(geocode)) => Some(LocationCandidate {
                    latitude: geocode.coordinates.latitude,
                    longitude: geocode.coordinates.longitude,
                    // Capsule displays the configured label for a fixed place.
                    place_name: Some(place.to_string()),
                    place_details: geocode.place_details,
                    source: "default".to_string(),
                    from_cache: false,
                    cache_write: None,
                }),
                Ok(None) => {
                    warnings.push("configured location could not be geocoded".to_string());
                    None
                }
                Err(error) => {
                    warnings.push(format!("configured location unavailable: {error}"));
                    None
                }
            };
        }

        let method = policy
            .auto_capture_method
            .as_deref()
            .unwrap_or("ip")
            .trim()
            .to_lowercase();
        if method != "ip" {
            warnings.push(format!("unsupported location capture method: {method}"));
            return None;
        }
        if !policy.allow_network {
            warnings.push("IP location unavailable while offline".to_string());
            return None;
        }
        let coordinates = match providers::location_from_ip(
            self.dependencies.http.as_ref(),
            deadline,
            self.dependencies.cancellation.as_ref(),
        ) {
            Ok(Some(coordinates)) => coordinates,
            Ok(None) => {
                warnings.push("IP location provider returned no coordinates".to_string());
                return None;
            }
            Err(error) => {
                warnings.push(format!("IP location unavailable: {error}"));
                return None;
            }
        };

        let mut place_name = None;
        let mut place_details = None;
        let mut cache_write = None;
        if policy.allow_cache {
            if let Ok(Some(cached)) = read_geocode_cache(
                &policy.database_path,
                &coordinates,
                policy.geocoding_cache_hours,
                self.dependencies.clock.now(),
                deadline.remaining().unwrap_or_default(),
            ) {
                place_name = Some(cached.place_name);
                place_details = cached.place_details;
                *location_from_cache = true;
            }
        }
        if place_name.is_none() && policy.allow_network && !deadline.expired() {
            match providers::reverse_geocode(
                self.dependencies.http.as_ref(),
                &coordinates,
                false,
                deadline,
                self.dependencies.cancellation.as_ref(),
            ) {
                Ok(Some(geocode)) => {
                    place_name = Some(geocode.place_name.clone());
                    place_details = geocode.place_details.clone();
                    if policy.allow_cache {
                        cache_write = Some(geocode);
                    }
                }
                Ok(None) => warnings.push("reverse geocoder returned no place".to_string()),
                Err(error) => warnings.push(format!("reverse geocoding unavailable: {error}")),
            }
        }
        Some(LocationCandidate {
            latitude: coordinates.latitude,
            longitude: coordinates.longitude,
            place_name,
            place_details,
            source: "ip".to_string(),
            from_cache: *location_from_cache,
            cache_write,
        })
    }

    fn resolve_existing_place(
        &self,
        existing: &PersistedLocation,
        policy: &ContextPolicy,
        deadline: &ContextDeadline,
        location_from_cache: &mut bool,
        warnings: &mut Vec<String>,
    ) -> Option<LocationCandidate> {
        let coordinates = providers::Coordinates::new(existing.latitude, existing.longitude)?;
        let mut place_name = None;
        let mut place_details = None;
        let mut cache_write = None;
        if policy.allow_cache {
            if let Ok(Some(cached)) = read_geocode_cache(
                &policy.database_path,
                &coordinates,
                policy.geocoding_cache_hours,
                self.dependencies.clock.now(),
                deadline.remaining().unwrap_or_default(),
            ) {
                place_name = Some(cached.place_name);
                place_details = cached.place_details;
                *location_from_cache = true;
            }
        }
        if place_name.is_none() && policy.allow_network && !deadline.expired() {
            match providers::reverse_geocode(
                self.dependencies.http.as_ref(),
                &coordinates,
                true,
                deadline,
                self.dependencies.cancellation.as_ref(),
            ) {
                Ok(Some(geocode)) => {
                    place_name = Some(geocode.place_name.clone());
                    place_details = geocode.place_details.clone();
                    if policy.allow_cache {
                        cache_write = Some(geocode);
                    }
                }
                Ok(None) => warnings.push("reverse geocoder returned no place".to_string()),
                Err(error) => warnings.push(format!("reverse geocoding unavailable: {error}")),
            }
        }
        // Keep coordinates as a usable candidate even when reverse geocoding
        // is unavailable.  Weather enrichment is independent of place-name
        // enrichment and must still be able to use the saved coordinates.
        Some(LocationCandidate {
            latitude: coordinates.latitude,
            longitude: coordinates.longitude,
            place_name,
            place_details,
            source: existing.source.clone(),
            from_cache: *location_from_cache,
            cache_write,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_weather(
        &self,
        policy: &ContextPolicy,
        provider: &str,
        candidate: &LocationCandidate,
        entry_created_at: &str,
        existing: Option<&PersistedLocation>,
        deadline: &ContextDeadline,
        weather_from_cache: &mut bool,
        weather_age_seconds: &mut Option<i64>,
        warnings: &mut Vec<String>,
    ) -> Option<WeatherObservation> {
        let fixture = location::test_auto_capture_fixture();
        let old_entry = providers::parse_entry_time(entry_created_at).is_some_and(|created| {
            deadline
                .clock
                .now()
                .naive_utc()
                .signed_duration_since(created)
                > ChronoDuration::hours(3)
        });
        if old_entry && provider == "met_norway" {
            warnings.push(
                "MET Norway does not provide historical observations; old entry weather unavailable"
                    .to_string(),
            );
            return None;
        }

        // Enrichment only asks for fields that are missing.  Returning an
        // existing complete observation here would make it look newly fetched.
        if let Some(existing) = existing {
            if existing.weather_complete() {
                return None;
            }
        }

        if let Some(fixture) = fixture {
            let temp_c = fixture.weather_temp_c?;
            return Some(weather_observation_from_data(
                providers::WeatherData {
                    temp_c: Some(temp_c),
                    temp_f: Some(round1(temp_c * 9.0 / 5.0 + 32.0)),
                    condition: fixture.weather_condition,
                    icon: None,
                    humidity: None,
                    wind_kph: None,
                },
                provider,
                deadline.clock.now(),
            ));
        }

        // A fifteen-minute cache is valid only for current weather.  Historical
        // Open-Meteo requests must never reuse a current observation.
        if !old_entry && policy.allow_cache {
            let key = WeatherCacheKey::new(candidate.latitude, candidate.longitude, provider);
            if let Some(mut observation) = self.dependencies.cache.get(&key, deadline.clock.now()) {
                if let Some(fetched_at) = observation.fetched_at {
                    let age = deadline
                        .clock
                        .now()
                        .signed_duration_since(fetched_at)
                        .num_seconds();
                    if (0..=WEATHER_CACHE_MAX_AGE_SECONDS).contains(&age) {
                        if observation.provider.is_none() {
                            observation.provider = Some(provider.to_string());
                        }
                        *weather_from_cache = true;
                        *weather_age_seconds = Some(age);
                        return Some(observation);
                    }
                }
            }
        }
        if !policy.allow_network || deadline.expired() {
            return None;
        }
        match providers::weather(
            self.dependencies.http.as_ref(),
            provider,
            &providers::Coordinates {
                latitude: candidate.latitude,
                longitude: candidate.longitude,
            },
            Some(entry_created_at),
            deadline.clock.now(),
            deadline,
            self.dependencies.cancellation.as_ref(),
        ) {
            Ok(Some(data)) => {
                let observation =
                    weather_observation_from_data(data, provider, deadline.clock.now());
                if policy.allow_cache {
                    let key =
                        WeatherCacheKey::new(candidate.latitude, candidate.longitude, provider);
                    self.dependencies.cache.put(&key, &observation);
                }
                Some(observation)
            }
            Ok(None) => {
                warnings.push(format!("{provider} returned no weather observation"));
                None
            }
            Err(error) => {
                warnings.push(format!("{provider} weather unavailable: {error}"));
                None
            }
        }
    }
}

fn clear_unpersisted_context(report: &mut ContextReport, warning: &str) {
    report.result.location = None;
    report.result.weather = None;
    report.result.location_status = ContextStatus::Unavailable;
    report.result.weather_status = ContextStatus::Unavailable;
    report.result.warnings.push(warning.to_string());
}

/// Capture context with production dependencies.
pub fn capture_context(
    db_path: &Path,
    entry_uuid: &str,
    policy: ContextPolicy,
) -> Result<ContextResult> {
    ContextService::default().capture(&ContextRequest::capture(db_path, entry_uuid, policy))
}

/// Capture context with injected dependencies, primarily for `cap` and tests.
pub fn capture_context_with_dependencies(
    request: &ContextRequest,
    dependencies: ContextDependencies,
) -> Result<ContextReport> {
    ContextService::new(dependencies).capture_report(request)
}

/// Explicit enrichment variant for clients/tests that provide their own
/// transport, clock or cache.  It retains the same fresh verified-backup
/// contract as [`enrich_context`].
pub fn enrich_context_with_dependencies(
    request: &ContextRequest,
    dependencies: ContextDependencies,
) -> Result<ContextReport> {
    let service = ContextService::new(dependencies);
    let mut request = request.clone();
    if request.backup_policy.is_none() {
        request.backup_policy = Some(default_backup_policy(&request.db_path));
    }
    let preparation = service.prepare(&request)?;
    if !preparation.is_persistable() {
        return Ok(preparation.into_report());
    }
    let timeout = preparation.remaining().unwrap_or_default();
    if timeout.is_zero() {
        return Ok(preparation.into_report());
    }
    let backup_policy = request
        .backup_policy
        .clone()
        .expect("context enrichment sets a backup policy before prepare");
    let db_path = request.db_path.clone();
    let guarded = crate::backup::with_database_backup_for_database_using_policy_with_timeout(
        &db_path,
        "context.enrich",
        timeout,
        &backup_policy,
        move |_path| service.persist_prepared(&request, preparation),
    )?;
    Ok(guarded.value)
}

/// Explicit enrichment creates one verified fresh backup, then fills only
/// missing fields.  The sync workflow calls the lower-level pending helper
/// after its sync backup has been released; that helper uses a lock-only
/// persistence phase.
pub fn enrich_context(
    db_path: &Path,
    entry_uuid: &str,
    policy: ContextPolicy,
) -> Result<ContextResult> {
    let service = ContextService::default();
    let backup_policy = default_backup_policy(db_path);
    let request = ContextRequest::enrich(db_path, entry_uuid, policy)
        .with_backup_policy(backup_policy.clone());
    let preparation = service.prepare(&request)?;
    if !preparation.is_persistable() {
        return Ok(preparation.into_report().result);
    }
    let timeout = preparation.remaining().unwrap_or_default();
    if timeout.is_zero() {
        return Ok(preparation.into_report().result);
    }
    let guarded = crate::backup::with_database_backup_for_database_using_policy_with_timeout(
        db_path,
        "context.enrich",
        timeout,
        &backup_policy,
        move |_path| service.persist_prepared(&request, preparation),
    )?;
    Ok(guarded.value.result)
}

fn default_backup_policy(db_path: &Path) -> crate::contracts::BackupPolicy {
    crate::contracts::BackupPolicy::new(
        crate::db::backup_directory_for_database(db_path),
        crate::db::backup_retention_count_for_database(db_path),
    )
}

/// Enrich all pending mobile rows for the existing desktop sync wrapper.
/// Provider work is prepared before acquiring the caller-owned mutation lock;
/// only the short merge/persist phase runs under that lock.  No backup is made
/// here because the sync wrapper already reserved one for its mutation.
pub fn enrich_pending_mobile_locations(db_path: &Path) -> Result<usize> {
    let settings = location::load_context_settings(db_path, None)?;
    if !settings.valid || !settings.policy.auto_capture {
        return Ok(0);
    }
    let read = crate::db::open_read_only_connection(db_path)?;
    if !location::table_exists(&read, "plugin_entry_locations")? {
        return Ok(0);
    }
    let mut statement = read.prepare(
        "SELECT pel.entry_uuid
         FROM plugin_entry_locations pel
         JOIN entries e ON e.uuid = pel.entry_uuid
         WHERE LOWER(TRIM(pel.source)) = 'mobile'
           AND (pel.place_name IS NULL OR TRIM(pel.place_name) = ''
             OR pel.weather_temp_c IS NULL
             OR pel.weather_condition IS NULL OR TRIM(pel.weather_condition) = ''
             OR pel.weather_fetched_at IS NULL OR TRIM(pel.weather_fetched_at) = '')",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let uuids = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    drop(read);

    let service = ContextService::default();
    let mut prepared = Vec::new();
    for uuid in uuids {
        let mut request = ContextRequest::enrich(db_path, &uuid, settings.policy.clone());
        request.config_valid = settings.valid;
        let preparation = service.prepare(&request)?;
        if preparation.is_persistable() {
            prepared.push((request, preparation));
        }
    }
    if prepared.is_empty() {
        return Ok(0);
    }
    let timeout = prepared
        .iter()
        .filter_map(|(_, preparation)| preparation.remaining())
        .min()
        .unwrap_or_default();
    if timeout.is_zero() {
        return Ok(0);
    }
    backup::with_mutation_lock_for_database_with_timeout(db_path, timeout, move |_| {
        let mut enriched = 0;
        for (request, preparation) in prepared {
            let report = service.persist_prepared(&request, preparation)?;
            if report.result.persisted_at.is_some() {
                enriched += 1;
            }
        }
        Ok(enriched)
    })
}

#[derive(Debug, Clone)]
struct LocationCandidate {
    latitude: f64,
    longitude: f64,
    place_name: Option<String>,
    place_details: Option<String>,
    source: String,
    from_cache: bool,
    cache_write: Option<providers::GeocodeData>,
}

#[derive(Debug, Clone)]
struct PersistedLocation {
    latitude: f64,
    longitude: f64,
    place_name: Option<String>,
    place_details: Option<String>,
    source: String,
    weather_temp_c: Option<f64>,
    weather_temp_f: Option<f64>,
    weather_condition: Option<String>,
    weather_icon: Option<String>,
    weather_humidity: Option<i64>,
    weather_wind_kph: Option<f64>,
    weather_fetched_at: Option<String>,
    created_at: Option<String>,
}

impl PersistedLocation {
    fn weather_complete(&self) -> bool {
        self.weather_temp_c.is_some()
            && self
                .weather_condition
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && self
                .weather_fetched_at
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }
}

#[derive(Clone)]
pub struct ContextDeadline {
    pub(crate) clock: Arc<dyn Clock>,
    start_ms: u64,
    limit_ms: u64,
}

impl ContextDeadline {
    pub fn new(clock: Arc<dyn Clock>, limit_ms: u64) -> Self {
        Self {
            start_ms: clock.monotonic_ms(),
            clock,
            limit_ms,
        }
    }

    pub fn remaining(&self) -> Option<Duration> {
        let elapsed = self.clock.monotonic_ms().saturating_sub(self.start_ms);
        if elapsed >= self.limit_ms {
            None
        } else {
            Some(Duration::from_millis(self.limit_ms - elapsed))
        }
    }

    pub fn expired(&self) -> bool {
        self.remaining().is_none()
    }
}

fn read_entry_created_at(path: &Path, uuid: &str, timeout: Duration) -> Result<Option<String>> {
    if timeout.is_zero() {
        return Ok(None);
    }
    let connection = open_read_only_with_timeout(path, timeout)?;
    connection
        .query_row(
            "SELECT created_at FROM entries WHERE uuid = ?1",
            [uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn load_existing_location(
    path: &Path,
    uuid: &str,
    timeout: Duration,
) -> Result<Option<PersistedLocation>> {
    if timeout.is_zero() {
        return Ok(None);
    }
    let connection = open_read_only_with_timeout(path, timeout)?;
    if !location::table_exists(&connection, "plugin_entry_locations")? {
        return Ok(None);
    }
    load_existing_location_from_connection(&connection, uuid)
}

fn load_existing_location_from_connection(
    connection: &Connection,
    uuid: &str,
) -> Result<Option<PersistedLocation>> {
    let columns = location::table_columns(connection, "plugin_entry_locations")?;
    if !columns.contains("latitude") || !columns.contains("longitude") {
        return Ok(None);
    }
    let expression = |name: &str| {
        if columns.contains(name) {
            name.to_string()
        } else {
            "NULL".to_string()
        }
    };
    let sql = format!(
        "SELECT latitude, longitude, {place_name}, {place_details}, {source},
                {weather_temp_c}, {weather_temp_f}, {weather_condition}, {weather_icon},
                {weather_humidity}, {weather_wind_kph}, {weather_fetched_at}, {created_at}
         FROM plugin_entry_locations WHERE entry_uuid = ?1",
        place_name = expression("place_name"),
        place_details = expression("place_details"),
        source = expression("source"),
        weather_temp_c = expression("weather_temp_c"),
        weather_temp_f = expression("weather_temp_f"),
        weather_condition = expression("weather_condition"),
        weather_icon = expression("weather_icon"),
        weather_humidity = expression("weather_humidity"),
        weather_wind_kph = expression("weather_wind_kph"),
        weather_fetched_at = expression("weather_fetched_at"),
        created_at = expression("created_at"),
    );
    connection
        .query_row(&sql, [uuid], |row| {
            Ok(PersistedLocation {
                latitude: row.get(0)?,
                longitude: row.get(1)?,
                place_name: row.get(2)?,
                place_details: row.get(3)?,
                source: row
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| "auto".to_string()),
                weather_temp_c: row.get(5)?,
                weather_temp_f: row.get(6)?,
                weather_condition: row.get(7)?,
                weather_icon: row.get(8)?,
                weather_humidity: row.get(9)?,
                weather_wind_kph: row.get(10)?,
                weather_fetched_at: row.get(11)?,
                created_at: row.get(12)?,
            })
        })
        .optional()
        .map_err(Into::into)
}

fn database_identity_matches(request: &ContextRequest) -> bool {
    request.database_identity.as_ref().is_none_or(|expected| {
        request.db_path.exists()
            && crate::db::FileIdentity::for_path(&request.db_path).same_file(expected)
    })
}

fn persist_context(
    path: &Path,
    uuid: &str,
    _enrich: bool,
    report: &ContextReport,
    existing: Option<&PersistedLocation>,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<Option<PersistOutcome>> {
    if cancellation.is_cancelled() || deadline.expired() {
        return Ok(None);
    }
    let timeout = deadline.remaining().unwrap_or_default();
    if timeout.is_zero() {
        return Ok(None);
    }
    let mut connection = open_read_write_with_timeout(path, timeout)?;
    // Opening/configuring SQLite can consume a meaningful part of the
    // invocation budget (notably while switching journal mode).  Refresh the
    // busy timeout from the live deadline immediately before BEGIN so that a
    // contended writer cannot wait on the stale pre-open timeout.
    if !refresh_write_timeout(&connection, deadline, cancellation)? {
        return Ok(None);
    }
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if cancellation.is_cancelled() || deadline.expired() {
        return Ok(None);
    }
    // Schema creation, the existence check, and the row re-read all happen
    // under one immediate transaction.  A concurrent manual edit cannot land
    // between the read and merge.
    location::ensure_schema(&tx)?;
    if cancellation.is_cancelled() || deadline.expired() {
        return Ok(None);
    }
    let entry_exists = tx
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM entries WHERE uuid = ?1)",
            [uuid],
            |row| row.get::<_, bool>(0),
        )
        .unwrap_or(false);
    if !entry_exists {
        return Ok(None);
    }

    let location = report.result.location.as_ref();
    let weather = report.result.weather.as_ref();
    if location.is_none() && weather.is_none() {
        return Ok(None);
    }

    // Re-read while the write connection is open.  Another client may have
    // inserted or edited a row while provider requests were in flight.
    let latest_existing = load_existing_location_from_connection(&tx, uuid)?;
    // If a row existed when provider work began but was deleted meanwhile,
    // treat that as a deletion rather than resurrecting stale context.  A
    // missing row on an initial capture remains insertable.
    if existing.is_some() && latest_existing.is_none() {
        return Ok(None);
    }
    let current_existing = latest_existing.as_ref();
    let weather_discarded_due_to_coordinate_change =
        if let (Some(_weather), Some(current)) = (weather, current_existing) {
            let current_place_key =
                WeatherCacheKey::new(current.latitude, current.longitude, "").place_key();
            report.details.weather_place_key.as_deref() != Some(current_place_key.as_str())
        } else {
            false
        };
    let weather_for_merge = if weather_discarded_due_to_coordinate_change {
        None
    } else {
        weather
    };
    let merged = if let Some(current) = current_existing {
        merge_existing(current, location, weather_for_merge)
    } else {
        let Some(location) = location else {
            return Ok(None);
        };
        PersistedLocation {
            latitude: location.latitude,
            longitude: location.longitude,
            place_name: location.place_name.clone(),
            place_details: location.place_details.clone(),
            source: location
                .source
                .clone()
                .unwrap_or_else(|| "auto".to_string()),
            weather_temp_c: weather_for_merge.and_then(|value| value.temp_c),
            weather_temp_f: weather_for_merge.and_then(|value| value.temp_f),
            weather_condition: weather_for_merge.and_then(|value| value.condition.clone()),
            weather_icon: weather_for_merge.and_then(|value| value.icon.clone()),
            weather_humidity: weather_for_merge.and_then(|value| value.humidity),
            weather_wind_kph: weather_for_merge.and_then(|value| value.wind_kph),
            weather_fetched_at: weather_for_merge
                .and_then(|value| value.fetched_at)
                .map(local_timestamp),
            created_at: Some(local_timestamp(deadline.clock.now())),
        }
    };

    if let Some(cache) = report.location_cache_write.as_ref() {
        tx.execute(
            "INSERT OR REPLACE INTO plugin_location_cache
             (latitude, longitude, place_name, place_details, reverse_geocoded_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                round4(cache.coordinates.latitude),
                round4(cache.coordinates.longitude),
                cache.place_name,
                cache.place_details,
                local_timestamp(deadline.clock.now()),
            ],
        )?;
    }
    if current_existing.is_some() {
        tx.execute(
            "UPDATE plugin_entry_locations
             SET latitude = ?2, longitude = ?3, place_name = ?4, place_details = ?5,
                 source = ?6, weather_temp_c = ?7, weather_temp_f = ?8,
                 weather_condition = ?9, weather_icon = ?10, weather_humidity = ?11,
                 weather_wind_kph = ?12, weather_fetched_at = ?13
             WHERE entry_uuid = ?1",
            params![
                uuid,
                merged.latitude,
                merged.longitude,
                merged.place_name,
                merged.place_details,
                merged.source,
                merged.weather_temp_c,
                merged.weather_temp_f,
                merged.weather_condition,
                merged.weather_icon,
                merged.weather_humidity,
                merged.weather_wind_kph,
                merged.weather_fetched_at,
            ],
        )?;
    } else {
        tx.execute(
            "DELETE FROM sync_location_tombstones WHERE entry_uuid = ?1",
            [uuid],
        )?;
        tx.execute(
            "INSERT INTO plugin_entry_locations (
                entry_uuid, latitude, longitude, place_name, place_details, source,
                weather_temp_c, weather_temp_f, weather_condition, weather_icon,
                weather_humidity, weather_wind_kph, weather_fetched_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                uuid,
                merged.latitude,
                merged.longitude,
                merged.place_name,
                merged.place_details,
                merged.source,
                merged.weather_temp_c,
                merged.weather_temp_f,
                merged.weather_condition,
                merged.weather_icon,
                merged.weather_humidity,
                merged.weather_wind_kph,
                merged.weather_fetched_at,
                merged
                    .created_at
                    .clone()
                    .unwrap_or_else(|| local_timestamp(deadline.clock.now())),
            ],
        )?;
    }
    if cancellation.is_cancelled() || deadline.expired() {
        return Ok(None);
    }
    tx.commit()?;
    let persisted_weather = persisted_weather(&merged, weather_for_merge);
    Ok(Some(PersistOutcome {
        persisted_at: deadline.clock.now(),
        location: ContextLocation {
            latitude: merged.latitude,
            longitude: merged.longitude,
            place_name: merged.place_name,
            place_details: merged.place_details,
            source: Some(merged.source.clone()),
        },
        weather: persisted_weather,
        weather_discarded_due_to_coordinate_change,
    }))
}

fn persisted_weather(
    location: &PersistedLocation,
    hint: Option<&WeatherObservation>,
) -> Option<WeatherObservation> {
    if !location.weather_complete() {
        return None;
    }
    let fetched_at = location
        .weather_fetched_at
        .as_deref()
        .and_then(|value| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
                .ok()
                .and_then(|naive| chrono::Local.from_local_datetime(&naive).single())
                .map(|value| value.with_timezone(&Utc))
        })
        .or_else(|| hint.and_then(|value| value.fetched_at));
    Some(WeatherObservation {
        provider: hint.and_then(|value| value.provider.clone()),
        condition: location.weather_condition.clone(),
        icon: location.weather_icon.clone(),
        temp_c: location.weather_temp_c,
        temp_f: location.weather_temp_f,
        humidity: location.weather_humidity,
        wind_kph: location.weather_wind_kph,
        fetched_at,
    })
}

struct PersistOutcome {
    persisted_at: DateTime<Utc>,
    location: ContextLocation,
    weather: Option<WeatherObservation>,
    weather_discarded_due_to_coordinate_change: bool,
}

fn merge_existing(
    existing: &PersistedLocation,
    location: Option<&ContextLocation>,
    weather: Option<&WeatherObservation>,
) -> PersistedLocation {
    let incoming_location_source = location.and_then(|value| value.source.clone());
    let incoming_weather_fetched_at = weather
        .and_then(|value| value.fetched_at)
        .map(local_timestamp);
    PersistedLocation {
        latitude: existing.latitude,
        longitude: existing.longitude,
        place_name: preferred_text(
            existing.place_name.as_ref(),
            location.and_then(|value| value.place_name.as_ref()),
        ),
        place_details: preferred_text(
            existing.place_details.as_ref(),
            location.and_then(|value| value.place_details.as_ref()),
        ),
        source: if existing.source.trim().is_empty() {
            incoming_location_source.unwrap_or_else(|| "auto".to_string())
        } else {
            existing.source.clone()
        },
        weather_temp_c: existing
            .weather_temp_c
            .or_else(|| weather.and_then(|value| value.temp_c)),
        weather_temp_f: existing
            .weather_temp_f
            .or_else(|| weather.and_then(|value| value.temp_f)),
        weather_condition: preferred_text(
            existing.weather_condition.as_ref(),
            weather.and_then(|value| value.condition.as_ref()),
        ),
        weather_icon: preferred_text(
            existing.weather_icon.as_ref(),
            weather.and_then(|value| value.icon.as_ref()),
        ),
        weather_humidity: existing
            .weather_humidity
            .or_else(|| weather.and_then(|value| value.humidity)),
        weather_wind_kph: existing
            .weather_wind_kph
            .or_else(|| weather.and_then(|value| value.wind_kph)),
        weather_fetched_at: preferred_text(
            existing.weather_fetched_at.as_ref(),
            incoming_weather_fetched_at.as_ref(),
        ),
        created_at: existing.created_at.clone(),
    }
}

fn preferred_text(existing: Option<&String>, incoming: Option<&String>) -> Option<String> {
    existing
        .filter(|value| !value.trim().is_empty())
        .or(incoming)
        .cloned()
}

fn weather_observation_from_data(
    data: providers::WeatherData,
    provider: &str,
    fetched_at: DateTime<Utc>,
) -> WeatherObservation {
    WeatherObservation {
        provider: Some(provider.to_string()),
        condition: data.condition,
        icon: data.icon,
        temp_c: data.temp_c,
        temp_f: data.temp_f,
        humidity: data.humidity,
        wind_kph: data.wind_kph,
        fetched_at: Some(fetched_at),
    }
}

fn read_geocode_cache(
    path: &Path,
    coordinates: &providers::Coordinates,
    cache_hours: Option<u64>,
    now: DateTime<Utc>,
    timeout: Duration,
) -> Result<Option<providers::GeocodeData>> {
    if timeout.is_zero() {
        return Ok(None);
    }
    let connection = open_read_only_with_timeout(path, timeout)?;
    if !location::table_exists(&connection, "plugin_location_cache")? {
        return Ok(None);
    }
    let cutoff = now - ChronoDuration::hours(cache_hours.unwrap_or(720) as i64);
    let cutoff_local = cutoff
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M")
        .to_string();
    connection
        .query_row(
            "SELECT place_name, place_details
             FROM plugin_location_cache
             WHERE latitude = ?1 AND longitude = ?2 AND reverse_geocoded_at >= ?3",
            params![
                round4(coordinates.latitude),
                round4(coordinates.longitude),
                cutoff_local
            ],
            |row| {
                let place_name = row.get::<_, String>(0)?;
                let place_details = row.get::<_, Option<String>>(1)?;
                Ok(providers::GeocodeData {
                    coordinates: coordinates.clone(),
                    place_name,
                    place_details,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn open_read_only_with_timeout(path: &Path, timeout: Duration) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", path.display()))?;
    connection.busy_timeout(timeout)?;
    connection.pragma_update(None, "query_only", "ON")?;
    Ok(connection)
}

fn open_read_write_with_timeout(path: &Path, timeout: Duration) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open {}", path.display()))?;
    connection.busy_timeout(timeout)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(connection)
}

fn refresh_write_timeout(
    connection: &Connection,
    deadline: &ContextDeadline,
    cancellation: &dyn Cancellation,
) -> Result<bool> {
    if cancellation.is_cancelled() {
        return Ok(false);
    }
    let Some(timeout) = deadline.remaining() else {
        return Ok(false);
    };
    connection.busy_timeout(timeout)?;
    // A cancellation/deadline transition can happen while configuring the
    // connection.  Do not enter BEGIN after that transition, and reset the
    // timeout once more from the final remaining budget.
    if cancellation.is_cancelled() {
        return Ok(false);
    }
    let Some(timeout) = deadline.remaining() else {
        return Ok(false);
    };
    connection.busy_timeout(timeout)?;
    Ok(true)
}

fn local_timestamp(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Mutex;

    struct FakeHttp {
        responses: Mutex<Vec<(String, String)>>,
        requests: Mutex<Vec<String>>,
    }

    impl FakeHttp {
        fn new(responses: Vec<(&str, &str)>) -> Self {
            Self {
                responses: Mutex::new(
                    responses
                        .into_iter()
                        .map(|(needle, body)| (needle.to_string(), body.to_string()))
                        .collect(),
                ),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn request_count(&self) -> usize {
            self.requests
                .lock()
                .map(|items| items.len())
                .unwrap_or_default()
        }
    }

    impl HttpClient for FakeHttp {
        fn get(
            &self,
            request: HttpRequest,
            _timeout: Duration,
        ) -> std::result::Result<HttpResponse, String> {
            self.requests
                .lock()
                .map_err(|_| "request lock poisoned".to_string())?
                .push(request.url.clone());
            let responses = self
                .responses
                .lock()
                .map_err(|_| "response lock poisoned".to_string())?;
            let body = responses
                .iter()
                .find(|(needle, _)| request.url.contains(needle))
                .map(|(_, body)| body.clone())
                .ok_or_else(|| format!("no fixture for {}", request.url))?;
            Ok(HttpResponse {
                status: 200,
                body: body.into_bytes(),
            })
        }
    }

    struct AdvancingHttp {
        inner: FakeHttp,
        clock: Arc<ManualClock>,
        advance: Duration,
    }

    impl HttpClient for AdvancingHttp {
        fn get(
            &self,
            request: HttpRequest,
            timeout: Duration,
        ) -> std::result::Result<HttpResponse, String> {
            let response = self.inner.get(request, timeout);
            self.clock.advance(self.advance);
            response
        }
    }

    fn fixture_database(path: &Path, created_at: &str) -> PathBuf {
        let db_path = path.join("capsule.db");
        let connection = Connection::open(&db_path).expect("db");
        connection
            .execute_batch(
                "CREATE TABLE entries (
                    id INTEGER PRIMARY KEY,
                    uuid TEXT UNIQUE NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT,
                    text TEXT NOT NULL,
                    text_plain TEXT NOT NULL,
                    content_format TEXT NOT NULL,
                    title TEXT,
                    summary TEXT,
                    mood TEXT,
                    starred INTEGER DEFAULT 0,
                    pinned INTEGER DEFAULT 0,
                    hidden INTEGER DEFAULT 0
                );",
            )
            .expect("schema");
        connection
            .execute(
                "INSERT INTO entries (id, uuid, created_at, text, text_plain, content_format)
                 VALUES (1, 'entry_test', ?1, 'body', 'body', 'plain')",
                [created_at],
            )
            .expect("entry");
        db_path
    }

    fn dependencies(http: Arc<dyn HttpClient>, clock: Arc<dyn Clock>) -> ContextDependencies {
        ContextDependencies {
            http,
            clock,
            cache: Arc::new(NoopContextCache),
            cancellation: Arc::new(NeverCancel),
        }
    }

    fn ip_weather_fixtures() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "ip-api.com",
                r#"{"status":"success","lat":69.65,"lon":18.96}"#,
            ),
            (
                "nominatim.openstreetmap.org/reverse",
                r#"{"display_name":"Tromso, Norway","address":{"city":"Tromso","country":"Norway","country_code":"no"}}"#,
            ),
            (
                "api.open-meteo.com/v1/forecast",
                r#"{"current":{"temperature_2m":12.3,"relative_humidity_2m":72,"weather_code":61,"wind_speed_10m":4.0,"is_day":1}}"#,
            ),
        ]
    }

    #[test]
    fn memory_cache_enforces_fifteen_minute_age_and_exact_coordinates() {
        let clock = Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap();
        let cache = MemoryContextCache::new();
        let key = WeatherCacheKey::new(69.65, 18.96, "open_meteo");
        let observation = WeatherObservation {
            provider: Some("open_meteo".to_string()),
            condition: Some("Clear".to_string()),
            icon: Some("clear-day".to_string()),
            temp_c: Some(12.0),
            temp_f: Some(53.6),
            humidity: Some(50),
            wind_kph: Some(5.0),
            fetched_at: Some(clock),
        };
        cache.put(&key, &observation);
        assert!(cache
            .get(&key, clock + ChronoDuration::minutes(15))
            .is_some());
        assert!(cache
            .get(&key, clock + ChronoDuration::minutes(16))
            .is_none());
        assert!(cache
            .get(&WeatherCacheKey::new(69.650001, 18.96, "open_meteo"), clock,)
            .is_none());
    }

    #[test]
    fn capture_uses_fresh_weather_cache_without_calling_weather_provider() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let now = Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap();
        let clock = Arc::new(ManualClock::new(now));
        let cache = Arc::new(MemoryContextCache::new());
        let cached = WeatherObservation {
            provider: Some("open_meteo".to_string()),
            condition: Some("Clear".to_string()),
            icon: Some("clear-day".to_string()),
            temp_c: Some(8.5),
            temp_f: Some(47.3),
            humidity: Some(40),
            wind_kph: Some(3.6),
            fetched_at: Some(now - ChronoDuration::minutes(5)),
        };
        cache.put(&WeatherCacheKey::new(69.65, 18.96, "open_meteo"), &cached);
        let http = Arc::new(FakeHttp::new(vec![
            (
                "ip-api.com",
                r#"{"status":"success","lat":69.65,"lon":18.96}"#,
            ),
            (
                "nominatim.openstreetmap.org/reverse",
                r#"{"display_name":"Tromso, Norway","address":{"city":"Tromso","country":"Norway","country_code":"no"}}"#,
            ),
            (
                "api.open-meteo.com/v1/forecast",
                r#"{"current":{"temperature_2m":99.0,"weather_code":65}}"#,
            ),
        ]));
        let dependencies = ContextDependencies {
            http: http.clone(),
            clock,
            cache,
            cancellation: Arc::new(NeverCancel),
        };
        let request = ContextRequest::capture(
            &db_path,
            "entry_test",
            ContextPolicy {
                database_path: db_path.clone(),
                ..ContextPolicy::default()
            },
        );
        let report = ContextService::new(dependencies)
            .capture_report(&request)
            .expect("capture");
        assert_eq!(http.request_count(), 2, "IP and reverse geocode only");
        assert_eq!(report.result.weather_status, ContextStatus::Cached);
        assert_eq!(report.details.weather_age_seconds, Some(300));
        assert_eq!(
            report
                .result
                .weather
                .as_ref()
                .and_then(|value| value.temp_c),
            Some(8.5)
        );
    }

    #[test]
    fn explicit_enrichment_uses_frozen_backup_policy_after_provider_work() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let backup_dir = temp_dir.path().join("backups");
        let connection = Connection::open(&db_path).expect("db");
        location::ensure_schema(&connection).expect("location schema");
        connection
            .execute(
                "INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, source, created_at)
                 VALUES ('entry_test', 69.65, 18.96, 'mobile', '2026-09-14 11:59')",
                [],
            )
            .expect("location");
        drop(connection);
        location::set_test_auto_capture_fixture(Some(location::TestAutoCaptureFixture {
            latitude: 69.65,
            longitude: 18.96,
            place_name: Some("Tromso, Norway".to_string()),
            source: "mobile".to_string(),
            weather_temp_c: Some(12.8),
            weather_condition: Some("Partly cloudy".to_string()),
        }));
        let request = ContextRequest::enrich(
            &db_path,
            "entry_test",
            ContextPolicy {
                database_path: db_path.clone(),
                ..ContextPolicy::default()
            },
        )
        .with_backup_policy(crate::contracts::BackupPolicy::new(&backup_dir, 2));
        let dependencies = ContextDependencies {
            http: Arc::new(FakeHttp::new(Vec::new())),
            clock: Arc::new(ManualClock::new(
                Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
            )),
            cache: Arc::new(NoopContextCache),
            cancellation: Arc::new(NeverCancel),
        };
        let report =
            enrich_context_with_dependencies(&request, dependencies).expect("explicit enrich");
        location::set_test_auto_capture_fixture(None);
        assert!(report.result.persisted_at.is_some());
        assert!(report.result.location.is_some());
        assert!(backup_dir.exists());
        assert_eq!(
            std::fs::read_dir(&backup_dir)
                .expect("backup directory")
                .filter_map(Result::ok)
                .count(),
            2,
            "database and manifest are published together",
        );
    }

    #[test]
    fn prepared_context_cannot_be_persisted_for_a_different_request_binding() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        location::set_test_auto_capture_fixture(Some(location::TestAutoCaptureFixture {
            latitude: 69.65,
            longitude: 18.96,
            place_name: Some("Tromso, Norway".to_string()),
            source: "ip".to_string(),
            weather_temp_c: Some(12.8),
            weather_condition: Some("Partly cloudy".to_string()),
        }));
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let dependencies = ContextDependencies {
            http: Arc::new(FakeHttp::new(Vec::new())),
            clock: clock.clone(),
            cache: Arc::new(NoopContextCache),
            cancellation: Arc::new(NeverCancel),
        };
        let service = ContextService::new(dependencies);
        let original = ContextRequest::capture(
            &db_path,
            "entry_test",
            ContextPolicy {
                database_path: db_path.clone(),
                ..ContextPolicy::default()
            },
        );
        let preparation = service.prepare(&original).expect("prepare");
        let different = ContextRequest::capture(
            &db_path,
            "different_entry",
            ContextPolicy {
                database_path: db_path.clone(),
                ..ContextPolicy::default()
            },
        );
        let report = service
            .persist_prepared(&different, preparation)
            .expect("binding check");
        location::set_test_auto_capture_fixture(None);
        assert!(report.result.persisted_at.is_none());
        assert!(report.result.location.is_none());
        assert!(report
            .result
            .warnings
            .iter()
            .any(|warning| warning.contains("does not match")));
        let connection = Connection::open(&db_path).expect("db");
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'plugin_entry_locations'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 0);
    }

    #[test]
    fn deadline_is_single_budget_not_multiplied() {
        let clock = Arc::new(ManualClock::new(Utc::now()));
        let deadline = ContextDeadline::new(clock.clone(), 8_000);
        assert_eq!(deadline.remaining().unwrap().as_millis(), 8_000);
        clock.advance(Duration::from_secs(7));
        assert!(deadline.remaining().unwrap() <= Duration::from_secs(1));
        clock.advance(Duration::from_secs(1));
        assert!(deadline.expired());
    }

    #[test]
    fn persistence_refreshes_busy_timeout_after_connection_setup_consumes_budget() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let clock = Arc::new(ManualClock::new(Utc::now()));
        let deadline = ContextDeadline::new(clock.clone(), 8_000);
        let initial_timeout = deadline.remaining().expect("initial budget");
        let connection =
            open_read_write_with_timeout(&db_path, initial_timeout).expect("write connection");

        // Simulate journal-mode setup consuming most of the invocation budget
        // before BEGIN IMMEDIATE is attempted.
        clock.advance(Duration::from_millis(7_500));
        assert!(refresh_write_timeout(&connection, &deadline, &NeverCancel).expect("refresh"));
        let busy_timeout_ms: i64 = connection
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .expect("busy timeout");
        assert!(
            busy_timeout_ms <= 500,
            "BEGIN must use the remaining budget, not the pre-open timeout"
        );
    }

    #[test]
    fn request_without_context_never_opens_database() {
        let request = ContextRequest::capture(
            Path::new("does-not-exist.db"),
            "entry",
            ContextPolicy::default(),
        )
        .without_context();
        let result = ContextService::default().capture(&request).expect("skip");
        assert_eq!(result.location_status, ContextStatus::Skipped);
        assert_eq!(result.weather_status, ContextStatus::Skipped);
    }

    #[test]
    fn capture_uses_one_weather_request_and_persists_actual_fields() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let http = Arc::new(FakeHttp::new(ip_weather_fixtures()));
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let policy = ContextPolicy {
            database_path: db_path.clone(),
            deadline_ms: 8_000,
            ..ContextPolicy::default()
        };
        let request = ContextRequest::capture(&db_path, "entry_test", policy);
        let report = ContextService::new(dependencies(http.clone(), clock))
            .capture_report(&request)
            .expect("capture");
        assert_eq!(http.request_count(), 3, "IP, reverse geocode, weather only");
        assert_eq!(report.result.location_status, ContextStatus::Captured);
        assert_eq!(report.result.weather_status, ContextStatus::Captured);
        assert!(report.result.persisted_at.is_some());

        let connection = Connection::open(&db_path).expect("db");
        let row: (f64, f64, String, String, f64, String) = connection
            .query_row(
                "SELECT latitude, longitude, source, place_name, weather_temp_c, weather_condition
                 FROM plugin_entry_locations WHERE entry_uuid = 'entry_test'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("location row");
        assert_eq!(row.2, "ip");
        assert_eq!(row.3, "Tromso, Norway");
        assert_eq!(row.4, 12.3);
        assert_eq!(row.5, "Slight rain");
    }

    #[test]
    fn fixed_place_failure_does_not_fall_back_to_ip() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let http = Arc::new(FakeHttp::new(vec![(
            "ip-api.com",
            r#"{"status":"success","lat":1,"lon":2}"#,
        )]));
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let policy = ContextPolicy {
            database_path: db_path.clone(),
            use_default_location: true,
            default_location_name: Some("Nowhere".to_string()),
            ..ContextPolicy::default()
        };
        let request = ContextRequest::capture(&db_path, "entry_test", policy);
        let report = ContextService::new(dependencies(http.clone(), clock))
            .capture_report(&request)
            .expect("capture");
        assert_eq!(http.request_count(), 1);
        assert_eq!(report.result.location_status, ContextStatus::Unavailable);
        assert!(report.result.location.is_none());
        assert!(report
            .result
            .warnings
            .iter()
            .any(|warning| warning.contains("configured location")));
    }

    #[test]
    fn offline_capture_does_not_call_http_or_create_context_schema() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let http = Arc::new(FakeHttp::new(ip_weather_fixtures()));
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let policy = ContextPolicy {
            database_path: db_path.clone(),
            allow_network: false,
            ..ContextPolicy::default()
        };
        let request = ContextRequest::capture(&db_path, "entry_test", policy);
        let report = ContextService::new(dependencies(http.clone(), clock))
            .capture_report(&request)
            .expect("offline capture");
        assert_eq!(http.request_count(), 0);
        assert_eq!(report.result.location_status, ContextStatus::Unavailable);
        let connection = Connection::open(&db_path).expect("db");
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'plugin_entry_locations'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 0);
    }

    #[test]
    fn cancellation_before_capture_skips_all_database_and_network_work() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let http = Arc::new(FakeHttp::new(ip_weather_fixtures()));
        let clock = Arc::new(ManualClock::new(Utc::now()));
        let token = Arc::new(CancellationToken::default());
        token.cancel();
        let dependencies = ContextDependencies {
            http: http.clone(),
            clock,
            cache: Arc::new(NoopContextCache),
            cancellation: token,
        };
        let request = ContextRequest::capture(&db_path, "entry_test", ContextPolicy::default());
        let report = ContextService::new(dependencies)
            .capture_report(&request)
            .expect("cancelled capture");
        assert_eq!(http.request_count(), 0);
        assert_eq!(report.result.location_status, ContextStatus::Skipped);
        assert_eq!(report.result.weather_status, ContextStatus::Skipped);
    }

    #[test]
    fn deadline_expiry_clears_provider_values_before_persistence() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let http = Arc::new(AdvancingHttp {
            inner: FakeHttp::new(ip_weather_fixtures()),
            clock: clock.clone(),
            advance: Duration::from_secs(9),
        });
        let policy = ContextPolicy {
            database_path: db_path.clone(),
            deadline_ms: 8_000,
            ..ContextPolicy::default()
        };
        let request = ContextRequest::capture(&db_path, "entry_test", policy);
        let report = ContextService::new(dependencies(http, clock))
            .capture_report(&request)
            .expect("capture");
        assert_eq!(report.result.location_status, ContextStatus::Unavailable);
        assert_eq!(report.result.weather_status, ContextStatus::Unavailable);
        assert!(report.result.location.is_none());
        assert!(report.result.weather.is_none());
        assert!(report.result.persisted_at.is_none());

        let connection = Connection::open(&db_path).expect("db");
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'plugin_entry_locations'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 0);
    }

    #[test]
    fn changed_database_identity_skips_all_provider_work() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let replacement_path = temp_dir.path().join("replacement.db");
        std::fs::copy(&db_path, &replacement_path).expect("copy replacement");
        let http = Arc::new(FakeHttp::new(ip_weather_fixtures()));
        let clock = Arc::new(ManualClock::new(Utc::now()));
        let request = ContextRequest::capture(
            &db_path,
            "entry_test",
            ContextPolicy {
                database_path: db_path.clone(),
                ..ContextPolicy::default()
            },
        );
        std::fs::remove_file(&db_path).expect("remove original");
        std::fs::rename(&replacement_path, &db_path).expect("replace database");

        let report = ContextService::new(dependencies(http.clone(), clock))
            .capture_report(&request)
            .expect("capture");
        assert_eq!(http.request_count(), 0);
        assert_eq!(report.result.location_status, ContextStatus::Unavailable);
        assert_eq!(report.result.weather_status, ContextStatus::Unavailable);
        assert!(report.result.persisted_at.is_none());
    }

    #[test]
    fn old_met_norway_entry_never_fetches_current_weather() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-13 08:00");
        let http = Arc::new(FakeHttp::new(vec![(
            "nominatim.openstreetmap.org/search",
            r#"[{"lat":"69.65","lon":"18.96","display_name":"Tromso, Norway","address":{"city":"Tromso","country":"Norway","country_code":"no"}}]"#,
        )]));
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let policy = ContextPolicy {
            database_path: db_path.clone(),
            use_default_location: true,
            default_location_name: Some("Tromso".to_string()),
            weather_provider: Some("met_norway".to_string()),
            ..ContextPolicy::default()
        };
        let request = ContextRequest::capture(&db_path, "entry_test", policy);
        let report = ContextService::new(dependencies(http.clone(), clock))
            .capture_report(&request)
            .expect("capture");
        assert_eq!(http.request_count(), 1, "forward geocode only");
        assert_eq!(report.result.weather_status, ContextStatus::Unavailable);
        assert!(report
            .result
            .warnings
            .iter()
            .any(|warning| warning.contains("historical")));
    }

    #[test]
    fn enrichment_preserves_existing_fields_and_fills_only_missing_weather() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let connection = Connection::open(&db_path).expect("db");
        location::ensure_schema(&connection).expect("location schema");
        connection
            .execute(
                "INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, place_name, source,
                     weather_temp_c, weather_condition, weather_fetched_at, created_at)
                 VALUES ('entry_test', 69.65, 18.96, 'Manual place', 'manual',
                         1.0, NULL, NULL, '2026-09-14 11:59')",
                [],
            )
            .expect("location");
        drop(connection);

        let http = Arc::new(FakeHttp::new(vec![(
            "api.open-meteo.com/v1/forecast",
            r#"{"current":{"temperature_2m":12.3,"relative_humidity_2m":72,"weather_code":61,"wind_speed_10m":4.0,"is_day":1}}"#,
        )]));
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let policy = ContextPolicy {
            database_path: db_path.clone(),
            ..ContextPolicy::default()
        };
        let request = ContextRequest::enrich(&db_path, "entry_test", policy);
        let report = ContextService::new(dependencies(http, clock))
            .capture_report(&request)
            .expect("enrich");
        assert_eq!(report.result.weather_status, ContextStatus::Captured);
        assert!(report.result.persisted_at.is_some());

        let connection = Connection::open(&db_path).expect("db");
        let row: (String, String, f64, String) = connection
            .query_row(
                "SELECT place_name, source, weather_temp_c, weather_condition
                 FROM plugin_entry_locations WHERE entry_uuid = 'entry_test'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("location row");
        assert_eq!(row.0, "Manual place");
        assert_eq!(row.1, "manual");
        assert_eq!(row.2, 1.0, "existing temperature is manual and preserved");
        assert_eq!(row.3, "Slight rain");
    }

    #[test]
    fn weather_is_not_attached_after_coordinates_change_during_provider_work() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let connection = Connection::open(&db_path).expect("db");
        location::ensure_schema(&connection).expect("location schema");
        connection
            .execute(
                "INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, place_name, source, created_at)
                 VALUES ('entry_test', 69.65, 18.96, 'Manual place', 'manual', '2026-09-14 11:59')",
                [],
            )
            .expect("location");
        let existing = load_existing_location(&db_path, "entry_test", Duration::from_secs(1))
            .expect("read location")
            .expect("existing location");
        connection
            .execute(
                "UPDATE plugin_entry_locations SET latitude = 40.71, longitude = -74.0
                 WHERE entry_uuid = 'entry_test'",
                [],
            )
            .expect("concurrent edit");
        drop(connection);

        let now = Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap();
        let weather = WeatherObservation {
            provider: Some("open_meteo".to_string()),
            condition: Some("Slight rain".to_string()),
            icon: Some("rain".to_string()),
            temp_c: Some(12.3),
            temp_f: Some(54.1),
            humidity: Some(72),
            wind_kph: Some(14.4),
            fetched_at: Some(now),
        };
        let report = ContextReport {
            result: ContextResult {
                location_status: ContextStatus::Captured,
                weather_status: ContextStatus::Captured,
                location: Some(ContextLocation {
                    latitude: existing.latitude,
                    longitude: existing.longitude,
                    place_name: Some("Provider place".to_string()),
                    place_details: None,
                    source: Some("ip".to_string()),
                }),
                weather: Some(weather),
                persisted_at: None,
                warnings: Vec::new(),
            },
            details: ContextDetails {
                weather_provider: Some("open_meteo".to_string()),
                weather_place_key: Some(
                    WeatherCacheKey::new(existing.latitude, existing.longitude, "").place_key(),
                ),
                weather_age_seconds: None,
                weather_from_cache: false,
                location_from_cache: false,
            },
            location_cache_write: None,
        };
        let clock = Arc::new(ManualClock::new(now));
        let deadline = ContextDeadline::new(clock, 8_000);
        let outcome = persist_context(
            &db_path,
            "entry_test",
            true,
            &report,
            Some(&existing),
            &deadline,
            &NeverCancel,
        )
        .expect("persist")
        .expect("persisted location");
        assert!(outcome.weather.is_none());
        assert!(outcome.weather_discarded_due_to_coordinate_change);

        let connection = Connection::open(&db_path).expect("db");
        let row: (f64, f64, Option<f64>, Option<String>) = connection
            .query_row(
                "SELECT latitude, longitude, weather_temp_c, weather_condition
                 FROM plugin_entry_locations WHERE entry_uuid = 'entry_test'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("location row");
        assert_eq!((row.0, row.1), (40.71, -74.0));
        assert!(row.2.is_none());
        assert!(row.3.is_none());
    }

    #[test]
    fn enrichment_keeps_weather_when_reverse_geocode_is_unavailable() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let http = Arc::new(FakeHttp::new(vec![
            (
                "ip-api.com",
                r#"{"status":"success","lat":69.65,"lon":18.96}"#,
            ),
            (
                "api.open-meteo.com/v1/forecast",
                r#"{"current":{"temperature_2m":12.3,"relative_humidity_2m":72,"weather_code":61,"wind_speed_10m":4.0,"is_day":1}}"#,
            ),
        ]));
        let clock = Arc::new(ManualClock::new(
            Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
        ));
        let policy = ContextPolicy {
            database_path: db_path.clone(),
            ..ContextPolicy::default()
        };
        let request = ContextRequest::enrich(&db_path, "entry_test", policy);
        let report = ContextService::new(dependencies(http.clone(), clock))
            .capture_report(&request)
            .expect("enrich");
        assert_eq!(http.request_count(), 3, "IP, reverse geocode, weather");
        assert_eq!(report.result.weather_status, ContextStatus::Captured);
        assert!(report.result.persisted_at.is_some());

        let connection = Connection::open(&db_path).expect("db");
        let row: (f64, f64, Option<String>, f64, String) = connection
            .query_row(
                "SELECT latitude, longitude, place_name, weather_temp_c, weather_condition
                 FROM plugin_entry_locations WHERE entry_uuid = 'entry_test'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("location row");
        assert_eq!((row.0, row.1), (69.65, 18.96));
        assert!(row.2.is_none());
        assert_eq!(row.3, 12.3);
        assert_eq!(row.4, "Slight rain");
    }

    #[test]
    fn enrichment_does_not_resurrect_deleted_location_row() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = fixture_database(temp_dir.path(), "2026-09-14 11:59");
        let connection = Connection::open(&db_path).expect("db");
        location::ensure_schema(&connection).expect("location schema");
        connection
            .execute(
                "INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, place_name, source, created_at)
                 VALUES ('entry_test', 69.65, 18.96, 'Manual place', 'mobile', '2026-09-14 11:59')",
                [],
            )
            .expect("location");
        let existing = load_existing_location(&db_path, "entry_test", Duration::from_secs(1))
            .expect("read location")
            .expect("existing location");
        connection
            .execute(
                "DELETE FROM plugin_entry_locations WHERE entry_uuid = 'entry_test'",
                [],
            )
            .expect("delete");
        drop(connection);

        let now = Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap();
        let report = ContextReport {
            result: ContextResult {
                location_status: ContextStatus::Captured,
                weather_status: ContextStatus::Skipped,
                location: Some(ContextLocation {
                    latitude: existing.latitude,
                    longitude: existing.longitude,
                    place_name: Some("Provider place".to_string()),
                    place_details: None,
                    source: Some("ip".to_string()),
                }),
                weather: None,
                persisted_at: None,
                warnings: Vec::new(),
            },
            details: ContextDetails {
                weather_provider: Some("open_meteo".to_string()),
                weather_place_key: None,
                weather_age_seconds: None,
                weather_from_cache: false,
                location_from_cache: false,
            },
            location_cache_write: None,
        };
        let deadline = ContextDeadline::new(Arc::new(ManualClock::new(now)), 8_000);
        assert!(persist_context(
            &db_path,
            "entry_test",
            true,
            &report,
            Some(&existing),
            &deadline,
            &NeverCancel,
        )
        .expect("persist")
        .is_none());

        let connection = Connection::open(&db_path).expect("db");
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM plugin_entry_locations WHERE entry_uuid = 'entry_test'",
                [],
                |row| row.get(0),
            )
            .expect("row count");
        assert_eq!(count, 0);
    }
}
