use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{anyhow, Context, Result};
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
pub const DEFAULT_BACKUP_RETENTION_COUNT: usize = 5;
pub const MAX_BACKUP_RETENTION_COUNT: usize = 1000;
pub const DEFAULT_WORD_TARGET: usize = 500;
pub const MAX_WORD_TARGET: usize = 100_000;

/// Where a resolved path came from.  Keeping provenance alongside the path
/// prevents a caller from presenting a fallback as an explicitly selected
/// setting.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PathSource {
    Explicit,
    Environment,
    SavedSettings,
    PlatformDefault,
    CapsuleHome,
    #[default]
    Fallback,
}

/// Safe, non-secret metadata for one resolved path candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PathDiagnostic {
    pub path: Option<String>,
    pub source: PathSource,
    pub exists: bool,
    pub readable: bool,
    pub error: Option<String>,
}

impl PathDiagnostic {
    fn for_path(path: Option<&Path>, source: PathSource, error: Option<String>) -> Self {
        let (exists, readable) = match path {
            Some(path) => {
                let exists = path.exists();
                let readable = exists && fs::metadata(path).is_ok();
                (exists, readable)
            }
            None => (false, false),
        };
        Self {
            path: path.map(path_to_string),
            source,
            exists,
            readable,
            error,
        }
    }
}

/// A stable-enough file identity used to bind a request to the database that
/// was inspected.  It includes a platform file ID where the host exposes one,
/// while keeping a portable canonical-path fallback for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIdentity {
    pub canonical_path: String,
    pub stable_id: Option<String>,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<String>,
}

impl FileIdentity {
    pub fn for_path(path: &Path) -> Self {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let metadata = fs::metadata(path).ok();
        Self {
            canonical_path: comparable_path(&canonical),
            stable_id: stable_file_id(path),
            size_bytes: metadata.as_ref().map(|value| value.len()),
            modified_at: metadata
                .and_then(|value| value.modified().ok())
                .map(system_time_to_iso),
        }
    }

    /// Compare identity while ignoring mutable observation metadata such as
    /// file length and modified time.  A stable OS file ID is preferred when
    /// available, with canonical path as the portable fallback.
    pub fn same_file(&self, other: &Self) -> bool {
        match (&self.stable_id, &other.stable_id) {
            (Some(left), Some(right)) => left == right,
            (None, None) => self.canonical_path == other.canonical_path,
            _ => false,
        }
    }
}

impl PartialEq for FileIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.same_file(other)
    }
}

impl Eq for FileIdentity {}

/// The allowlisted path settings that are safe to expose in diagnostics.
/// Tokens, sync credentials, and unrelated editor settings never cross this
/// boundary.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SafePathSettings {
    pub database_path: Option<PathBuf>,
    pub image_media_root: Option<PathBuf>,
    pub cover_wall_root: Option<PathBuf>,
    pub backup_directory: Option<PathBuf>,
    pub backup_retention_count: Option<usize>,
}

impl From<&LocalPathSettings> for SafePathSettings {
    fn from(settings: &LocalPathSettings) -> Self {
        Self {
            database_path: settings.database_path.as_deref().map(PathBuf::from),
            image_media_root: settings.image_media_root.as_deref().map(PathBuf::from),
            cover_wall_root: settings.cover_wall_root.as_deref().map(PathBuf::from),
            backup_directory: settings.backup_directory.as_deref().map(PathBuf::from),
            backup_retention_count: settings.backup_retention_count,
        }
    }
}

/// A caller-provided snapshot of process environment values.  Taking this
/// snapshot at the application boundary keeps resolver tests deterministic and
/// avoids mutating process-global environment variables in command execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolverEnvironment {
    pub capsule_db_path: Option<PathBuf>,
    pub capsule_home: Option<PathBuf>,
    pub capsule_config_path: Option<PathBuf>,
    pub capsule_backup_dir: Option<PathBuf>,
    pub path_settings_path: Option<PathBuf>,
    pub app_data: Option<PathBuf>,
    pub user_home: Option<PathBuf>,
}

impl ResolverEnvironment {
    pub fn from_process() -> Self {
        Self {
            capsule_db_path: env_path("CAPSULE_DB_PATH"),
            capsule_home: env_path("CAPSULE_HOME"),
            capsule_config_path: env_path("CAPSULE_CONFIG_PATH"),
            capsule_backup_dir: env_path(BACKUP_DIRECTORY_ENV),
            path_settings_path: env_path(PATH_SETTINGS_ENV),
            app_data: env_path("APPDATA"),
            user_home: env_path("USERPROFILE").or_else(|| env_path("HOME")),
        }
    }
}

/// Explicit resolver inputs.  A non-existent `database_path` is an error; it
/// never falls through to an environment or platform candidate.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRequest {
    pub database_path: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub backup_directory: Option<PathBuf>,
    pub path_settings_path: Option<PathBuf>,
    pub settings: Option<LocalPathSettings>,
}

impl ResolveRequest {
    pub fn explicit_database(path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: Some(path.into()),
            ..Self::default()
        }
    }

    pub fn with_database_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.database_path = Some(path.into());
        self
    }

    pub fn with_config_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_path = Some(path.into());
        self
    }

    pub fn with_backup_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.backup_directory = Some(path.into());
        self
    }

    pub fn with_path_settings_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path_settings_path = Some(path.into());
        self
    }
}

/// A complete, frozen path/configuration resolution plus read-only schema
/// inspection.  The contained settings are allowlisted and do not include
/// credentials.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCapsule {
    pub database_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub backup_directory: PathBuf,
    pub database_source: PathSource,
    pub config_source: PathSource,
    pub config_valid: bool,
    pub config_error: Option<String>,
    pub backup_source: PathSource,
    pub settings_path: Option<PathBuf>,
    pub settings_source: PathSource,
    pub settings_valid: bool,
    pub settings_error: Option<String>,
    pub database_identity: Option<FileIdentity>,
    pub settings: SafePathSettings,
    pub diagnostics: Vec<PathDiagnostic>,
    pub capabilities: CapabilityReport,
}

impl ResolvedCapsule {
    pub fn database_identity(&self) -> Option<&FileIdentity> {
        self.database_identity.as_ref()
    }
}

/// Read-only schema capability information.  No migration, ID repair, backup,
/// or other write is performed while building this report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaCapabilities {
    pub table_names: Vec<String>,
    pub required_columns: BTreeMap<String, bool>,
    pub optional_tables: BTreeMap<String, bool>,
    pub has_entries_table: bool,
    pub has_tags_table: bool,
    pub has_fts_table: bool,
    pub supports_read: bool,
    pub supports_write: bool,
    pub missing_required_columns: Vec<String>,
}

/// Safe diagnostics for a database before a client attempts a mutation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityReport {
    pub database_path: Option<String>,
    pub database_exists: bool,
    pub readable: bool,
    pub schema: SchemaCapabilities,
    pub write_support_reasons: Vec<String>,
    pub warnings: Vec<String>,
}

impl CapabilityReport {
    pub fn supports_read(&self) -> bool {
        self.schema.supports_read
    }

    pub fn supports_write(&self) -> bool {
        self.schema.supports_write
    }
}

/// Resolver facade useful to clients that need to retain the exact input and
/// environment snapshot for retries.
#[derive(Debug, Clone, Default)]
pub struct CapsuleResolver {
    request: ResolveRequest,
    environment: ResolverEnvironment,
}

impl CapsuleResolver {
    pub fn new(request: ResolveRequest) -> Self {
        Self {
            request,
            environment: ResolverEnvironment::default(),
        }
    }

    pub fn with_environment(request: ResolveRequest, environment: ResolverEnvironment) -> Self {
        Self {
            request,
            environment,
        }
    }

    pub fn resolve(&self) -> Result<ResolvedCapsule> {
        resolve_capsule_with_environment(self.request.clone(), self.environment.clone())
    }
}

const REQUIRED_ENTRY_COLUMNS: &[&str] = &[
    "id",
    "uuid",
    "created_at",
    "updated_at",
    "text",
    "text_plain",
    "content_format",
    "title",
    "summary",
    "mood",
    "starred",
    "pinned",
    "hidden",
];

const OPTIONAL_RELATION_TABLES: &[&str] = &[
    "tags",
    "entry_tags",
    "entries_fts",
    "plugin_entry_locations",
    "entry_continuations",
    "entry_thread_titles",
    "entry_thread_summaries",
    "plugin_entry_media",
    "plugin_media_assets",
];

/// Resolve a Capsule installation using only the values in `request` and the
/// platform defaults.  Callers that intentionally want process environment
/// values should snapshot them with [`ResolverEnvironment::from_process`] and
/// pass them to [`resolve_capsule_with_environment`].
pub fn resolve_capsule(request: ResolveRequest) -> Result<ResolvedCapsule> {
    resolve_capsule_with_environment(request, ResolverEnvironment::default())
}

/// Resolve using an explicit environment snapshot.  This is the boundary used
/// by desktop adapters and keeps tests from mutating process-global state.
pub fn resolve_capsule_with_environment(
    request: ResolveRequest,
    environment: ResolverEnvironment,
) -> Result<ResolvedCapsule> {
    let (settings, settings_path, settings_source, settings_error) =
        resolve_settings(&request, &environment)?;
    let settings_valid = settings_error.is_none();

    let user_home_string = environment
        .user_home
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    let mvp_database_path = platform_default_database_path(user_home_string.as_deref());

    let (database_path, database_source) = if let Some(path) = request.database_path.clone() {
        if !path.is_file() {
            return Err(anyhow!(
                "explicit database path does not exist: {}",
                path.display()
            ));
        }
        (path, PathSource::Explicit)
    } else if let Some(path) = environment.capsule_db_path.clone() {
        if !path.is_file() {
            return Err(anyhow!(
                "explicit CAPSULE_DB_PATH does not exist: {}",
                path.display()
            ));
        }
        (path, PathSource::Environment)
    } else if let Some(path) = settings.database_path.clone().map(PathBuf::from) {
        (path, PathSource::SavedSettings)
    } else if mvp_database_path.exists() {
        (mvp_database_path, PathSource::PlatformDefault)
    } else if let Some(home) = environment.user_home.clone() {
        let profile_path = home.join(".capsule").join("capsule.db");
        if profile_path.exists() {
            (profile_path, PathSource::PlatformDefault)
        } else if let Some(capsule_home) = environment.capsule_home.clone() {
            (capsule_home.join("capsule.db"), PathSource::CapsuleHome)
        } else {
            (mvp_database_path, PathSource::Fallback)
        }
    } else if let Some(capsule_home) = environment.capsule_home.clone() {
        (capsule_home.join("capsule.db"), PathSource::CapsuleHome)
    } else {
        (mvp_database_path, PathSource::Fallback)
    };

    let (backup_directory, backup_source) = if let Some(path) = request.backup_directory.clone() {
        (path, PathSource::Explicit)
    } else if let Some(path) = environment.capsule_backup_dir.clone() {
        (path, PathSource::Environment)
    } else if let Some(path) = settings.backup_directory.clone().map(PathBuf::from) {
        (path, PathSource::SavedSettings)
    } else {
        (
            database_directory_for_database(&database_path),
            PathSource::Fallback,
        )
    };

    let (config_path, config_source) = if let Some(path) = request.config_path.clone() {
        (Some(path), PathSource::Explicit)
    } else if let Some(path) = environment.capsule_config_path.clone() {
        (Some(path), PathSource::Environment)
    } else {
        (
            Some(database_directory_for_database(&database_path).join("config.json")),
            PathSource::Fallback,
        )
    };

    let (config_valid, config_error) = match config_path.as_deref() {
        Some(path) if path.exists() => match validate_config_object(path) {
            Ok(()) => (true, None),
            Err(error)
                if matches!(
                    config_source,
                    PathSource::Explicit | PathSource::Environment
                ) =>
            {
                return Err(anyhow!(
                    "explicit config override is invalid: {} ({error})",
                    path.display()
                ));
            }
            Err(error) => (false, Some(error.to_string())),
        },
        Some(path)
            if matches!(
                config_source,
                PathSource::Explicit | PathSource::Environment
            ) =>
        {
            return Err(anyhow!(
                "explicit config path does not exist: {}",
                path.display()
            ));
        }
        Some(path) => (
            false,
            Some(format!(
                "configuration file does not exist: {}",
                path.display()
            )),
        ),
        None => (true, None),
    };

    let database_identity = database_path
        .exists()
        .then(|| FileIdentity::for_path(&database_path));
    let capabilities = capability_report_for_database(&database_path)?;
    let mut diagnostics = vec![PathDiagnostic::for_path(
        Some(&database_path),
        database_source,
        None,
    )];
    diagnostics.push(PathDiagnostic::for_path(
        config_path.as_deref(),
        config_source,
        config_error.clone(),
    ));
    diagnostics.push(PathDiagnostic::for_path(
        Some(&backup_directory),
        backup_source,
        None,
    ));
    diagnostics.push(PathDiagnostic::for_path(
        settings_path.as_deref(),
        settings_source,
        settings_error.clone(),
    ));

    Ok(ResolvedCapsule {
        database_path,
        config_path,
        backup_directory,
        database_source,
        config_source,
        config_valid,
        config_error,
        backup_source,
        settings_path,
        settings_source,
        settings_valid,
        settings_error,
        database_identity,
        settings: SafePathSettings::from(&settings),
        diagnostics,
        capabilities,
    })
}

/// Desktop convenience resolution that snapshots process environment once.
pub fn resolve_capsule_from_environment(request: ResolveRequest) -> Result<ResolvedCapsule> {
    resolve_capsule_with_environment(request, ResolverEnvironment::from_process())
}

fn resolve_settings(
    request: &ResolveRequest,
    environment: &ResolverEnvironment,
) -> Result<(
    LocalPathSettings,
    Option<PathBuf>,
    PathSource,
    Option<String>,
)> {
    if let Some(settings) = request.settings.clone() {
        let mut settings = settings;
        settings.normalize();
        return Ok((settings, None, PathSource::Explicit, None));
    }

    let (path, inherited_source) = if let Some(path) = request.path_settings_path.clone() {
        (Some(path), PathSource::Explicit)
    } else if let Some(path) = environment.path_settings_path.clone() {
        (Some(path), PathSource::Environment)
    } else if let Some(app_data) = environment.app_data.clone() {
        (
            Some(app_data.join("Capsule").join("path_settings.json")),
            PathSource::PlatformDefault,
        )
    } else if let Some(home) = environment.user_home.clone() {
        (
            Some(home.join(".capsule").join("path_settings.json")),
            PathSource::PlatformDefault,
        )
    } else {
        (
            Some(PathBuf::from("path_settings.json")),
            PathSource::Fallback,
        )
    };
    match path {
        Some(path) => {
            if path.exists() {
                match read_local_path_settings_from_path(&path) {
                    Ok(settings) => Ok((settings, Some(path), inherited_source, None)),
                    Err(error)
                        if matches!(
                            inherited_source,
                            PathSource::Explicit | PathSource::Environment
                        ) =>
                    {
                        Err(error).with_context(|| {
                            format!("invalid path settings override: {}", path.display())
                        })
                    }
                    Err(error) => Ok((
                        LocalPathSettings::default(),
                        Some(path),
                        inherited_source,
                        Some(error.to_string()),
                    )),
                }
            } else if matches!(
                inherited_source,
                PathSource::Explicit | PathSource::Environment
            ) {
                Err(anyhow!(
                    "explicit path settings file does not exist: {}",
                    path.display()
                ))
            } else {
                Ok((
                    LocalPathSettings::default(),
                    Some(path),
                    inherited_source,
                    None,
                ))
            }
        }
        None => Ok((
            LocalPathSettings::default(),
            None,
            PathSource::Fallback,
            None,
        )),
    }
}

/// Inspect the schema through an already-open read-only connection.  This
/// function only reads `sqlite_master` and `PRAGMA table_info`; it never calls
/// an ID repair or schema migration helper.
pub fn inspect_capabilities(connection: &Connection) -> Result<CapabilityReport> {
    let schema = inspect_schema(connection)?;
    let mut required_columns = BTreeMap::new();
    let mut missing_required_columns = Vec::new();
    let entry_columns = if schema.has_entries_table {
        table_columns(connection, "entries")?
    } else {
        std::collections::HashSet::new()
    };
    for column in REQUIRED_ENTRY_COLUMNS {
        let present = entry_columns.contains(*column);
        required_columns.insert((*column).to_string(), present);
        if !present {
            missing_required_columns.push((*column).to_string());
        }
    }

    let mut optional_tables = BTreeMap::new();
    for table in OPTIONAL_RELATION_TABLES {
        optional_tables.insert(
            (*table).to_string(),
            schema
                .detected_tables
                .iter()
                .any(|detected| detected == table),
        );
    }

    let supports_read = schema.has_entries_table && missing_required_columns.is_empty();
    let supports_write = supports_read;
    let mut write_support_reasons = Vec::new();
    if !schema.has_entries_table {
        write_support_reasons.push("entries table is missing".to_string());
    }
    for column in &missing_required_columns {
        write_support_reasons.push(format!("entries.{column} is missing"));
    }
    if supports_write {
        write_support_reasons.push(
            "schema has the current entry projection; filesystem writability is checked at commit"
                .to_string(),
        );
    }

    Ok(CapabilityReport {
        database_path: None,
        database_exists: true,
        readable: true,
        schema: SchemaCapabilities {
            table_names: schema.detected_tables,
            required_columns,
            optional_tables,
            has_entries_table: schema.has_entries_table,
            has_tags_table: schema.has_tags_table,
            has_fts_table: schema.has_fts_table,
            supports_read,
            supports_write,
            missing_required_columns,
        },
        write_support_reasons,
        warnings: Vec::new(),
    })
}

/// Open a database read-only and inspect its capabilities.  Missing or
/// unreadable paths become a negative report rather than causing a fallback or
/// a write attempt.
pub fn capability_report_for_database(path: &Path) -> Result<CapabilityReport> {
    if !path.exists() {
        let mut report = CapabilityReport {
            database_path: Some(path_to_string(path)),
            database_exists: false,
            readable: false,
            schema: empty_schema_capabilities(),
            write_support_reasons: vec!["database file does not exist".to_string()],
            warnings: vec!["Database file does not exist.".to_string()],
        };
        report.schema.missing_required_columns = REQUIRED_ENTRY_COLUMNS
            .iter()
            .map(|column| (*column).to_string())
            .collect();
        return Ok(report);
    }

    let connection = match open_read_only_connection(path) {
        Ok(connection) => connection,
        Err(error) => {
            return Ok(CapabilityReport {
                database_path: Some(path_to_string(path)),
                database_exists: true,
                readable: false,
                schema: empty_schema_capabilities(),
                write_support_reasons: vec![format!(
                    "database could not be opened read-only: {error}"
                )],
                warnings: vec![format!("Unable to inspect database read-only: {error}")],
            });
        }
    };
    let mut report = inspect_capabilities(&connection)?;
    report.database_path = Some(path_to_string(path));
    Ok(report)
}

fn empty_schema_capabilities() -> SchemaCapabilities {
    SchemaCapabilities {
        table_names: Vec::new(),
        required_columns: REQUIRED_ENTRY_COLUMNS
            .iter()
            .map(|column| ((*column).to_string(), false))
            .collect(),
        optional_tables: OPTIONAL_RELATION_TABLES
            .iter()
            .map(|table| ((*table).to_string(), false))
            .collect(),
        has_entries_table: false,
        has_tags_table: false,
        has_fts_table: false,
        supports_read: false,
        supports_write: false,
        missing_required_columns: REQUIRED_ENTRY_COLUMNS
            .iter()
            .map(|column| (*column).to_string())
            .collect(),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalPathSettings {
    pub database_path: Option<String>,
    pub image_media_root: Option<String>,
    pub cover_wall_root: Option<String>,
    pub backup_directory: Option<String>,
    pub backup_retention_count: Option<usize>,
    pub sync_path: Option<String>,
    pub github_gist_id: Option<String>,
    pub github_gist_token: Option<String>,
    pub auto_sync_enabled: Option<bool>,
    pub auto_sync_interval_minutes: Option<i64>,
    pub minimize_to_tray_on_close: Option<bool>,
    pub debug_menu_enabled: Option<bool>,
    pub word_target_enabled: Option<bool>,
    pub word_target: Option<usize>,
    pub gauntlet_mode_enabled: Option<bool>,
    pub show_window_after_update_restart: Option<bool>,
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
    let user_home = user_home_directory();
    let default_database_path = platform_default_database_path(user_home.as_deref());
    resolve_database_path_from_parts(
        env_value("CAPSULE_DB_PATH"),
        read_local_path_settings().database_path,
        user_home,
        env_value("CAPSULE_HOME"),
        &default_database_path,
        Path::exists,
    )
}

#[cfg(windows)]
fn platform_default_database_path(_user_home: Option<&str>) -> PathBuf {
    PathBuf::from(MVP_DATABASE_PATH)
}

#[cfg(not(windows))]
fn platform_default_database_path(user_home: Option<&str>) -> PathBuf {
    user_home
        .map(PathBuf::from)
        .map(|home| home.join(".capsule").join("capsule.db"))
        .unwrap_or_else(|| PathBuf::from(MVP_DATABASE_PATH))
}

fn resolve_database_path_from_parts(
    capsule_db_path: Option<String>,
    local_database_path: Option<String>,
    user_home: Option<String>,
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

    if let Some(home) = user_home {
        let profile_path = PathBuf::from(home).join(".capsule").join("capsule.db");
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

pub fn backup_retention_count_for_database(path: &Path) -> usize {
    if is_active_database_path(path) {
        return read_local_path_settings()
            .backup_retention_count
            .unwrap_or(DEFAULT_BACKUP_RETENTION_COUNT);
    }

    DEFAULT_BACKUP_RETENTION_COUNT
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

pub fn local_cover_wall_root_for_database(path: &Path) -> Option<PathBuf> {
    if is_active_database_path(path) {
        return read_local_path_settings()
            .cover_wall_root
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

    if let Some(home) = user_home_directory() {
        return PathBuf::from(home)
            .join(".capsule")
            .join("path_settings.json");
    }

    PathBuf::from("path_settings.json")
}

fn user_home_directory() -> Option<String> {
    env_value("USERPROFILE").or_else(|| env_value("HOME"))
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
    read_local_path_settings_from_path(&local_path_settings_path())
}

fn read_local_path_settings_from_path(path: &Path) -> Result<LocalPathSettings> {
    if !path.exists() {
        return Ok(LocalPathSettings::default());
    }

    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut settings: LocalPathSettings = serde_json::from_slice(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    settings.normalize();
    Ok(settings)
}

pub fn write_local_path_settings(settings: &LocalPathSettings) -> Result<()> {
    write_local_path_settings_to_path(&local_path_settings_path(), settings)
}

fn write_local_path_settings_to_path(path: &Path, settings: &LocalPathSettings) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut settings = settings.clone();
    settings.normalize();
    fs::write(path, serde_json::to_vec_pretty(&settings)?)
        .with_context(|| format!("failed to write {}", path.display()))
}

pub fn set_show_window_after_update_restart(requested: bool) -> Result<()> {
    set_show_window_after_update_restart_at_path(&local_path_settings_path(), requested)
}

fn set_show_window_after_update_restart_at_path(path: &Path, requested: bool) -> Result<()> {
    let mut settings = read_local_path_settings_from_path(path)?;
    settings.show_window_after_update_restart = requested.then_some(true);
    write_local_path_settings_to_path(path, &settings)
}

pub fn consume_show_window_after_update_restart() -> Result<bool> {
    consume_show_window_after_update_restart_at_path(&local_path_settings_path())
}

fn consume_show_window_after_update_restart_at_path(path: &Path) -> Result<bool> {
    let mut settings = read_local_path_settings_from_path(path)?;
    let requested = settings.show_window_after_update_restart.unwrap_or(false);
    if requested {
        settings.show_window_after_update_restart = None;
        write_local_path_settings_to_path(path, &settings)?;
    }

    Ok(requested)
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

fn table_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<std::collections::HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    rows.collect::<rusqlite::Result<std::collections::HashSet<_>>>()
        .map_err(Into::into)
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn validate_config_object(path: &Path) -> Result<()> {
    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&raw)
        .with_context(|| format!("failed to parse {} as JSON", path.display()))?;
    if !value.is_object() {
        return Err(anyhow!("configuration root must be a JSON object"));
    }
    Ok(())
}

fn stable_file_id(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };

        let file = fs::File::open(path).ok()?;
        let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
        let success =
            unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) };
        if success == 0 {
            None
        } else {
            let info = unsafe { info.assume_init() };
            let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
            Some(format!("windows:{}:{index}", info.dwVolumeSerialNumber))
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(path).ok()?;
        Some(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        None
    }
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
        self.cover_wall_root = normalize_path_setting(self.cover_wall_root.take());
        self.backup_directory = normalize_path_setting(self.backup_directory.take());
        self.backup_retention_count = self
            .backup_retention_count
            .map(|count| count.clamp(1, MAX_BACKUP_RETENTION_COUNT));
        self.sync_path = normalize_path_setting(self.sync_path.take());
        self.github_gist_id = normalize_path_setting(self.github_gist_id.take());
        self.github_gist_token = normalize_path_setting(self.github_gist_token.take());
        self.auto_sync_interval_minutes = self
            .auto_sync_interval_minutes
            .map(|minutes| minutes.clamp(1, 24 * 60));
        self.word_target = self
            .word_target
            .map(|target| target.clamp(1, MAX_WORD_TARGET));
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

    #[test]
    #[cfg(not(windows))]
    fn platform_default_database_uses_home_capsule_directory() {
        let home = PathBuf::from("/Users/capsule");
        let expected = home.join(".capsule").join("capsule.db");
        let resolved = platform_default_database_path(Some(&home.to_string_lossy()));

        assert_eq!(resolved, expected);
    }

    #[test]
    fn update_restart_window_request_round_trips_and_clears() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let settings_path = temp_dir.path().join("path_settings.json");
        let settings = LocalPathSettings {
            minimize_to_tray_on_close: Some(true),
            ..LocalPathSettings::default()
        };
        write_local_path_settings_to_path(&settings_path, &settings).expect("seed settings");

        set_show_window_after_update_restart_at_path(&settings_path, true).expect("mark request");
        let marked = read_local_path_settings_from_path(&settings_path).expect("read marked");
        assert_eq!(marked.minimize_to_tray_on_close, Some(true));
        assert_eq!(marked.show_window_after_update_restart, Some(true));

        assert!(
            consume_show_window_after_update_restart_at_path(&settings_path)
                .expect("consume request")
        );
        let consumed = read_local_path_settings_from_path(&settings_path).expect("read consumed");
        assert_eq!(consumed.minimize_to_tray_on_close, Some(true));
        assert_eq!(consumed.show_window_after_update_restart, None);

        assert!(
            !consume_show_window_after_update_restart_at_path(&settings_path)
                .expect("consume empty request")
        );
    }

    #[test]
    fn word_target_settings_round_trip_and_normalize() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let settings_path = temp_dir.path().join("path_settings.json");
        let settings = LocalPathSettings {
            word_target_enabled: Some(true),
            word_target: Some(MAX_WORD_TARGET + 1),
            gauntlet_mode_enabled: Some(true),
            ..LocalPathSettings::default()
        };

        write_local_path_settings_to_path(&settings_path, &settings).expect("write settings");
        let stored = read_local_path_settings_from_path(&settings_path).expect("read settings");

        assert_eq!(stored.word_target_enabled, Some(true));
        assert_eq!(stored.word_target, Some(MAX_WORD_TARGET));
        assert_eq!(stored.gauntlet_mode_enabled, Some(true));
    }

    #[test]
    fn explicit_resolver_binds_database_config_and_backup_without_fallback() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_capability_fixture(temp_dir.path().join("journal.db"));
        let config_path = temp_dir.path().join("explicit-config.json");
        let backup_dir = temp_dir.path().join("backups");
        std::fs::write(&config_path, br#"{"location.auto_capture":"false"}"#).expect("config");

        let resolved = resolve_capsule(
            ResolveRequest::explicit_database(&db_path)
                .with_config_path(&config_path)
                .with_backup_directory(&backup_dir),
        )
        .expect("resolve");

        assert_eq!(resolved.database_path, db_path);
        assert_eq!(resolved.database_source, PathSource::Explicit);
        assert_eq!(resolved.config_path, Some(config_path));
        assert_eq!(resolved.config_source, PathSource::Explicit);
        assert!(resolved.config_valid);
        assert_eq!(resolved.backup_directory, backup_dir);
        assert_eq!(resolved.backup_source, PathSource::Explicit);
        assert!(resolved.capabilities.schema.supports_read);
        assert!(resolved.capabilities.schema.supports_write);
    }

    #[test]
    fn resolver_reads_saved_database_path_from_app_data_candidate() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let app_data = temp_dir.path().join("appdata");
        let settings_path = app_data.join("Capsule").join("path_settings.json");
        let db_path = create_capability_fixture(temp_dir.path().join("saved.db"));
        let settings = LocalPathSettings {
            database_path: Some(db_path.to_string_lossy().to_string()),
            ..LocalPathSettings::default()
        };
        write_local_path_settings_to_path(&settings_path, &settings).expect("settings");

        let resolved = resolve_capsule_with_environment(
            ResolveRequest::default(),
            ResolverEnvironment {
                app_data: Some(app_data),
                user_home: Some(temp_dir.path().join("home")),
                ..ResolverEnvironment::default()
            },
        )
        .expect("resolve");

        assert_eq!(resolved.database_path, db_path);
        assert_eq!(resolved.database_source, PathSource::SavedSettings);
        assert_eq!(resolved.settings_source, PathSource::PlatformDefault);
        assert_eq!(resolved.settings_path, Some(settings_path));
    }

    #[test]
    fn explicit_missing_paths_and_malformed_settings_fail_without_fallback() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let missing_db = temp_dir.path().join("missing.db");
        let error = resolve_capsule(ResolveRequest::explicit_database(&missing_db))
            .expect_err("missing explicit database must fail");
        assert!(error.to_string().contains("explicit database path"));

        let bad_settings = temp_dir.path().join("bad-settings.json");
        std::fs::write(&bad_settings, b"not json").expect("settings");
        let error =
            resolve_capsule(ResolveRequest::default().with_path_settings_path(&bad_settings))
                .expect_err("malformed explicit settings must fail");
        assert!(error.to_string().contains("path settings"));
    }

    #[test]
    fn inherited_malformed_settings_are_reported_without_global_fallback() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let app_data = temp_dir.path().join("appdata");
        let settings_path = app_data.join("Capsule").join("path_settings.json");
        std::fs::create_dir_all(settings_path.parent().unwrap()).expect("settings directory");
        std::fs::write(&settings_path, b"not json").expect("settings");

        let resolved = resolve_capsule_with_environment(
            ResolveRequest::default(),
            ResolverEnvironment {
                app_data: Some(app_data),
                capsule_home: Some(temp_dir.path().join("capsule-home")),
                ..ResolverEnvironment::default()
            },
        )
        .expect("inherited malformed settings are diagnostic");

        assert!(!resolved.settings_valid);
        assert!(resolved.settings_error.is_some());
        assert!(resolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.error.is_some()));
    }

    #[test]
    fn config_validation_is_strict_for_override_and_diagnostic_for_inherited_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_capability_fixture(temp_dir.path().join("journal.db"));
        let bad_config = temp_dir.path().join("bad-config.json");
        std::fs::write(&bad_config, b"[]").expect("config");

        let error = resolve_capsule(
            ResolveRequest::explicit_database(&db_path).with_config_path(&bad_config),
        )
        .expect_err("malformed explicit config must fail");
        assert!(error.to_string().contains("explicit config override"));

        let sibling = db_path.parent().unwrap().join("config.json");
        std::fs::write(&sibling, b"[]").expect("sibling config");
        let resolved = resolve_capsule(ResolveRequest::explicit_database(&db_path))
            .expect("inherited malformed config is diagnostic");
        assert!(!resolved.config_valid);
        assert!(resolved.config_error.is_some());
        assert!(resolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.error.is_some()));
    }

    #[test]
    fn file_identity_ignores_mutable_observation_metadata() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("journal.db");
        std::fs::write(&db_path, b"before").expect("seed");
        let before = FileIdentity::for_path(&db_path);
        std::fs::write(&db_path, b"a longer database observation").expect("rewrite");
        let after = FileIdentity::for_path(&db_path);

        assert_eq!(before, after);
        assert!(before.same_file(&after));
        assert_eq!(before.canonical_path, after.canonical_path);
    }

    #[test]
    fn file_identity_detects_replacement_at_same_path() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("journal.db");
        let replacement_path = temp_dir.path().join("replacement.db");
        std::fs::write(&db_path, b"before").expect("seed");
        std::fs::write(&replacement_path, b"replacement").expect("replacement");
        let before = FileIdentity::for_path(&db_path);
        std::fs::remove_file(&db_path).expect("remove original");
        std::fs::rename(&replacement_path, &db_path).expect("replace");
        let after = FileIdentity::for_path(&db_path);

        assert!(!before.same_file(&after));
        assert_ne!(before.stable_id, after.stable_id);
    }

    #[test]
    fn capability_inspection_reports_missing_columns_without_writing() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("legacy.db");
        let connection = Connection::open(&db_path).expect("open");
        connection
            .execute_batch(
                "CREATE TABLE entries (id INTEGER, uuid TEXT, created_at TEXT, text TEXT);",
            )
            .expect("schema");
        drop(connection);
        let before = FileIdentity::for_path(&db_path);
        let report = capability_report_for_database(&db_path).expect("report");
        let after = FileIdentity::for_path(&db_path);

        assert!(!report.schema.supports_read);
        assert!(report
            .schema
            .missing_required_columns
            .contains(&"text_plain".to_string()));
        assert_eq!(before, after);
    }

    fn create_capability_fixture(path: PathBuf) -> PathBuf {
        let connection = Connection::open(&path).expect("open fixture");
        connection
            .execute_batch(
                "CREATE TABLE entries (
                    id INTEGER PRIMARY KEY,
                    uuid TEXT,
                    created_at TEXT,
                    updated_at TEXT,
                    text TEXT,
                    text_plain TEXT,
                    content_format TEXT,
                    title TEXT,
                    summary TEXT,
                    mood TEXT,
                    starred INTEGER,
                    pinned INTEGER,
                    hidden INTEGER
                );",
            )
            .expect("schema");
        drop(connection);
        path
    }
}
