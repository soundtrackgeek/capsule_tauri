//! Renderer-independent seams shared by capture and context workers.
//!
//! These DTOs intentionally describe facts and policy only.  They do not open
//! a database, perform network work, write receipts, or emit terminal output.

use std::path::PathBuf;

use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

/// Canonical authored fields for client retry comparison. Identity, destination,
/// backup policy and the invocation timestamp are deliberately separate: the
/// client must validate its frozen database binding and replay the original
/// CaptureRequest, never replace its reserved UUID or saved time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedCaptureContent {
    pub schema_version: u32,
    pub text: String,
    pub text_plain: String,
    pub content_format: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub mood: Option<String>,
    /// Trimmed, Unicode-lowercased, deduplicated and sorted comparison keys.
    pub tags: Vec<String>,
    pub starred: bool,
    pub pinned: bool,
    pub continue_from_uuid: Option<String>,
}

/// Explicit backup policy captured alongside a headless request.  A client
/// that needs deterministic retries must not allow the core to consult mutable
/// process settings after the request is created.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicy {
    pub directory: PathBuf,
    pub retention_count: usize,
}

impl BackupPolicy {
    pub fn new(directory: impl Into<PathBuf>, retention_count: usize) -> Self {
        Self {
            directory: directory.into(),
            retention_count,
        }
    }
}

/// A normalized request handed to the shared capture implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub text: String,
    pub content_format: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub mood: Option<String>,
    pub tags: Vec<String>,
    pub starred: bool,
    pub pinned: bool,
    pub continue_from_uuid: Option<String>,
    /// Immutable local wall-clock timestamp captured with its original
    /// offset.  Retrying later must not reconvert this through a new timezone.
    pub created_at: DateTime<FixedOffset>,
    pub reserved_uuid: String,
    pub capture_id: String,
    /// The database binding is frozen at request creation and is never
    /// re-resolved during commit or retry.
    pub database_path: PathBuf,
    pub database_identity: Option<crate::db::FileIdentity>,
    /// Explicit backup destination and retention snapshot.  Older serialized
    /// requests may omit this field, but capture rejects them rather than
    /// silently consulting current process settings.
    #[serde(default)]
    pub backup_policy: Option<BackupPolicy>,
}

impl CaptureRequest {
    pub fn new(
        text: impl Into<String>,
        capture_id: impl Into<String>,
        reserved_uuid: impl Into<String>,
        database_path: PathBuf,
        created_at: DateTime<FixedOffset>,
    ) -> Self {
        Self {
            text: text.into(),
            content_format: "markdown".to_string(),
            title: None,
            summary: None,
            mood: None,
            tags: Vec::new(),
            starred: false,
            pinned: false,
            continue_from_uuid: None,
            created_at,
            reserved_uuid: reserved_uuid.into(),
            capture_id: capture_id.into(),
            database_path,
            database_identity: None,
            backup_policy: None,
        }
    }

    pub fn with_backup_policy(mut self, policy: BackupPolicy) -> Self {
        self.backup_policy = Some(policy);
        self
    }
}

/// The durable result available immediately after an entry transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitReceipt {
    pub uuid: String,
    pub capture_id: String,
    pub display_number: Option<i64>,
    /// Actual commit observation when this process made the commit.  A
    /// read-only reconciliation of an older row cannot invent that instant.
    pub saved_at: Option<DateTime<Utc>>,
    pub backup_path: Option<PathBuf>,
    pub backup_operation: Option<String>,
    pub committed: bool,
}

impl CommitReceipt {
    pub fn committed(
        uuid: impl Into<String>,
        capture_id: impl Into<String>,
        saved_at: DateTime<Utc>,
    ) -> Self {
        Self {
            uuid: uuid.into(),
            capture_id: capture_id.into(),
            display_number: None,
            saved_at: Some(saved_at),
            backup_path: None,
            backup_operation: None,
            committed: true,
        }
    }
}

/// State of the entry mutation.  `Unknown` must not be treated as a safe
/// ordinary retry because the process may have committed before losing I/O.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureOutcome {
    NotCommitted,
    Committed,
    Unknown,
}

/// Snapshot of context settings and network/cache policy for one request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextPolicy {
    pub auto_capture: bool,
    pub use_default_location: bool,
    pub default_location_name: Option<String>,
    pub auto_capture_method: Option<String>,
    pub weather_provider: Option<String>,
    pub geocoding_cache_hours: Option<u64>,
    pub allow_network: bool,
    pub allow_cache: bool,
    pub deadline_ms: u64,
    pub database_path: PathBuf,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            auto_capture: true,
            use_default_location: false,
            default_location_name: None,
            auto_capture_method: Some("ip".to_string()),
            weather_provider: Some("open_meteo".to_string()),
            geocoding_cache_hours: None,
            allow_network: true,
            allow_cache: true,
            deadline_ms: 8_000,
            database_path: PathBuf::new(),
        }
    }
}

/// Independent location and weather outcomes returned after commit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextResult {
    pub location_status: ContextStatus,
    pub weather_status: ContextStatus,
    pub location: Option<ContextLocation>,
    pub weather: Option<WeatherObservation>,
    pub persisted_at: Option<DateTime<Utc>>,
    pub warnings: Vec<String>,
}

impl Default for ContextResult {
    fn default() -> Self {
        Self {
            location_status: ContextStatus::Skipped,
            weather_status: ContextStatus::Skipped,
            location: None,
            weather: None,
            persisted_at: None,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextStatus {
    Captured,
    Cached,
    Disabled,
    Unavailable,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeatherObservation {
    pub provider: Option<String>,
    pub condition: Option<String>,
    pub icon: Option<String>,
    pub temp_c: Option<f64>,
    pub temp_f: Option<f64>,
    pub humidity: Option<i64>,
    pub wind_kph: Option<f64>,
    pub fetched_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub place_name: Option<String>,
    pub place_details: Option<String>,
    pub source: Option<String>,
}

/// A compact pure rendering input for consumers that need to retain context
/// without depending on database models.  This helper is intentionally small;
/// WP03 may extend context implementation while retaining this boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextAttachment {
    pub result: ContextResult,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn capture_request_preserves_local_offset_and_capsule_defaults() {
        let local = FixedOffset::east_opt(2 * 60 * 60)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 14, 18, 42, 0)
            .unwrap();
        let request = CaptureRequest::new(
            "hello",
            "cap_01",
            "entry_01",
            PathBuf::from("journal.db"),
            local,
        );

        assert_eq!(request.content_format, "markdown");
        assert_eq!(request.created_at.offset().local_minus_utc(), 2 * 60 * 60);
        assert!(!request.starred);
        assert!(!request.pinned);
    }

    #[test]
    fn context_policy_defaults_match_capsule_location_contract() {
        let policy = ContextPolicy::default();
        assert!(!policy.use_default_location);
        assert_eq!(policy.auto_capture_method.as_deref(), Some("ip"));
        assert_eq!(policy.weather_provider.as_deref(), Some("open_meteo"));
        assert_eq!(policy.deadline_ms, 8_000);
    }
}
