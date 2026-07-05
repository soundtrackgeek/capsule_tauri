use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Seek, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use rusqlite::Connection;
use serde_json::Value as JsonValue;

use crate::{
    ai_chat, ai_config, db, images,
    models::{
        AiChatContextFilters, AiChatContextPreviewRequest, DebugAiReport, DebugBundleResponse,
        DebugCheck, DebugDatabaseReport, DebugDiagnosticsResponse, DebugImageReport, DebugLogEntry,
        DebugLogRequest, DebugLogResponse, ImageAttachment,
    },
    settings,
};

const DEBUG_LOG_TAIL: usize = 20;
const SAMPLE_IMAGE_LIMIT: i64 = 6;
const ZIP_VERSION_NEEDED: u16 = 20;
const ZIP_UTF8_FLAG: u16 = 1 << 11;
const ZIP_STORED_METHOD: u16 = 0;
const DOS_DATE_1980_01_01: u16 = 33;

#[derive(Debug, Clone)]
struct ZipItem {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct CentralDirectoryEntry {
    name: String,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    local_header_offset: u32,
}

#[derive(Debug, Clone)]
struct DebugImageRow {
    hash: String,
    storage_key: String,
}

pub fn get_debug_diagnostics() -> Result<DebugDiagnosticsResponse> {
    let db_path = db::resolve_database_path();
    let settings_path = db::local_path_settings_path();
    let debug_log_path = debug_log_path();
    let bundle_directory = debug_bundle_directory();
    let mut warnings = Vec::new();

    let database = database_report(&db_path)?;
    let images = image_report(&db_path)?;
    let ai = ai_report(&db_path);
    let ai = match ai {
        Ok(report) => report,
        Err(error) => {
            warnings.push(format!("Unable to collect AI diagnostics: {error}"));
            DebugAiReport {
                cloud_provider: "unknown".to_string(),
                selected_model: "unknown".to_string(),
                provider_configured: false,
                provider_statuses: Vec::new(),
                context_preview_ok: false,
                context_preview_entries: 0,
                warnings: vec![error.to_string()],
            }
        }
    };
    let recent_logs = match read_recent_logs(DEBUG_LOG_TAIL) {
        Ok(logs) => logs,
        Err(error) => {
            warnings.push(format!("Unable to read debug log: {error}"));
            Vec::new()
        }
    };

    Ok(DebugDiagnosticsResponse {
        generated_at: Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        settings_path: db::path_to_string(&settings_path),
        debug_log_path: db::path_to_string(&debug_log_path),
        bundle_directory: db::path_to_string(&bundle_directory),
        database,
        images,
        ai,
        recent_logs,
        warnings,
    })
}

pub fn append_debug_log(input: DebugLogRequest) -> Result<DebugLogResponse> {
    let entry = DebugLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        level: normalize_log_level(input.level.as_deref()),
        message: normalize_log_message(&input.message)?,
    };
    let path = debug_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(&entry)?)
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(DebugLogResponse {
        entry,
        recent_logs: read_recent_logs(DEBUG_LOG_TAIL)?,
        log_path: db::path_to_string(&path),
    })
}

pub fn create_debug_bundle() -> Result<DebugBundleResponse> {
    let created_at = Utc::now().to_rfc3339();
    let diagnostics = get_debug_diagnostics()?;
    let mut warnings = diagnostics.warnings.clone();
    let bundle_directory = debug_bundle_directory();
    fs::create_dir_all(&bundle_directory)
        .with_context(|| format!("failed to create {}", bundle_directory.display()))?;

    let filename = format!(
        "capsule_diagnostics_{}.zip",
        Local::now().format("%Y%m%d_%H%M%S")
    );
    let bundle_path = bundle_directory.join(filename);
    let mut items = vec![
        ZipItem {
            name: "diagnostics.json".to_string(),
            bytes: serde_json::to_vec_pretty(&diagnostics)?,
        },
        ZipItem {
            name: "README.txt".to_string(),
            bytes: diagnostic_readme().into_bytes(),
        },
        ZipItem {
            name: "environment.txt".to_string(),
            bytes: environment_summary().into_bytes(),
        },
    ];

    push_debug_log_file(&mut items, &mut warnings, &debug_log_path());
    push_optional_file(
        &mut items,
        &mut warnings,
        "path_settings.redacted.json",
        &db::local_path_settings_path(),
        Redaction::Json,
    );
    let config_path = settings::config_path_for_database(&db::resolve_database_path());
    push_optional_file(
        &mut items,
        &mut warnings,
        "capsule_config.redacted.json",
        &config_path,
        Redaction::Json,
    );

    write_zip(&bundle_path, &items)?;
    let size_bytes = fs::metadata(&bundle_path)
        .with_context(|| format!("failed to inspect {}", bundle_path.display()))?
        .len();

    Ok(DebugBundleResponse {
        path: db::path_to_string(&bundle_path),
        size_bytes,
        created_at,
        included_files: items.into_iter().map(|item| item.name).collect(),
        warnings,
    })
}

fn database_report(db_path: &Path) -> Result<DebugDatabaseReport> {
    let status = db::database_status_for_path(db_path.to_path_buf())?;
    let mut warnings = Vec::new();
    let mut integrity_check = None;
    let mut foreign_key_issue_count = None;
    let wal_size_bytes = sidecar_size(db_path, "-wal");
    let mut required_tables = Vec::new();
    let mut feature_tables = Vec::new();

    if status.readable {
        let connection = db::open_read_only_connection(db_path)?;
        let tables = db::inspect_schema(&connection)?
            .detected_tables
            .into_iter()
            .collect::<HashSet<_>>();
        integrity_check = Some(
            connection
                .query_row("PRAGMA integrity_check(1)", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap_or_else(|error| format!("failed: {error}")),
        );
        foreign_key_issue_count = count_foreign_key_issues(&connection).ok();
        required_tables = table_checks(
            &connection,
            &tables,
            &[
                ("entries", "Entries"),
                ("tags", "Tags"),
                ("entry_tags", "Entry tags"),
            ],
            true,
        );
        feature_tables = table_checks(
            &connection,
            &tables,
            &[
                ("entries_fts", "Full-text search"),
                ("entry_history", "Entry history"),
                ("entry_continuations", "Threads"),
                ("entry_thread_titles", "Thread titles"),
                ("entry_thread_summaries", "Thread summaries"),
                ("plugin_media_assets", "Image assets"),
                ("plugin_entry_media", "Image attachments"),
                ("plugin_entry_locations", "Location and weather"),
                ("ai_conversations", "AI conversations"),
                ("ai_conversation_messages", "AI messages"),
                ("library_templates", "Template library"),
                ("library_prompts", "Prompt library"),
                ("sync_history", "Sync history"),
                ("gamification_xp_events", "Gamification XP"),
                ("gamification_quest_state", "Gamification quests"),
            ],
            false,
        );
    } else {
        warnings.push(
            "Database could not be opened; table and integrity checks were skipped.".to_string(),
        );
    }

    Ok(DebugDatabaseReport {
        status,
        integrity_check,
        foreign_key_issue_count,
        wal_size_bytes,
        required_tables,
        feature_tables,
        warnings,
    })
}

fn image_report(db_path: &Path) -> Result<DebugImageReport> {
    let media_root = images::get_image_media_root()?;
    let media_root_path = PathBuf::from(&media_root);
    let root_exists = media_root_path.exists();
    let root_writable = root_exists && test_directory_writable(&media_root_path);
    let mut warnings = Vec::new();
    let mut total_assets = 0;
    let mut total_attachments = 0;
    let mut attachments_with_originals = 0;
    let mut attachments_with_thumbnails = 0;
    let mut sample_images = Vec::new();

    if !root_exists {
        warnings.push("Resolved image media root does not exist.".to_string());
    } else if !root_writable {
        warnings.push("Resolved image media root is not writable.".to_string());
    }

    if let Ok(connection) = db::open_read_only_connection(db_path) {
        let tables = db::inspect_schema(&connection)?
            .detected_tables
            .into_iter()
            .collect::<HashSet<_>>();
        if tables.contains("plugin_media_assets") {
            total_assets = count_table_rows(&connection, "plugin_media_assets")?;
        }
        if tables.contains("plugin_entry_media") && tables.contains("plugin_media_assets") {
            let rows = image_attachment_rows(&connection)?;
            total_attachments = rows.len() as i64;
            let roots = images::media_roots_for_database(db_path, None);
            for row in &rows {
                if images::media_exists(&roots, &row.storage_key) {
                    attachments_with_originals += 1;
                }
                if images::media_exists(&roots, &images::thumbnail_key(&row.hash)) {
                    attachments_with_thumbnails += 1;
                }
            }
            sample_images = sample_image_attachments(db_path, &connection)?;
        } else {
            warnings.push("Image metadata tables are not available.".to_string());
        }
    } else {
        warnings.push("Database could not be opened for image diagnostics.".to_string());
    }

    Ok(DebugImageReport {
        media_root,
        root_exists,
        root_writable,
        total_assets,
        total_attachments,
        attachments_with_originals,
        attachments_with_thumbnails,
        missing_originals: total_attachments.saturating_sub(attachments_with_originals),
        missing_thumbnails: total_attachments.saturating_sub(attachments_with_thumbnails),
        sample_images,
        warnings,
    })
}

fn ai_report(db_path: &Path) -> Result<DebugAiReport> {
    let settings = ai_config::get_ai_settings_for_database(db_path)?;
    let provider_statuses = ai_config::get_ai_provider_status()?;
    let selected_model =
        ai_config::selected_model_for_provider(&settings, &settings.cloud_provider);
    let provider_configured = provider_statuses
        .iter()
        .find(|status| status.provider == settings.cloud_provider)
        .map(|status| status.configured)
        .unwrap_or(false);
    let mut warnings = settings.warnings.clone();
    let mut context_preview_ok = false;
    let mut context_preview_entries = 0;

    match ai_chat::preview_ai_chat_context_for_database(
        db_path,
        AiChatContextPreviewRequest {
            message: Some("Capsule debug local context preview".to_string()),
            scope: "search".to_string(),
            scope_identifiers: Vec::new(),
            context_filters: Some(AiChatContextFilters {
                text: None,
                since: None,
                until: None,
                tags: None,
                exclude_tags: None,
                moods: None,
                exclude_moods: None,
                starred: None,
                pinned: None,
                include_hidden: Some(false),
                has_images: None,
                sort: Some(crate::models::EntrySort::Desc),
                limit: Some(1),
            }),
            context_limit: Some(1),
            since: None,
            until: None,
            context_entry_uuids: None,
        },
    ) {
        Ok(preview) => {
            context_preview_ok = true;
            context_preview_entries = preview.entries.len() as i64;
            warnings.extend(preview.warnings);
        }
        Err(error) => warnings.push(format!("Local AI context preview failed: {error}")),
    }

    Ok(DebugAiReport {
        cloud_provider: settings.cloud_provider,
        selected_model,
        provider_configured,
        provider_statuses,
        context_preview_ok,
        context_preview_entries,
        warnings,
    })
}

fn table_checks(
    connection: &Connection,
    tables: &HashSet<String>,
    checks: &[(&str, &str)],
    required: bool,
) -> Vec<DebugCheck> {
    checks
        .iter()
        .map(|(table, label)| {
            if tables.contains(*table) {
                let detail = count_table_rows(connection, table)
                    .map(|count| format!("{count} rows"))
                    .unwrap_or_else(|error| format!("present; count failed: {error}"));
                DebugCheck {
                    label: (*label).to_string(),
                    status: "ok".to_string(),
                    detail,
                    warnings: Vec::new(),
                }
            } else {
                DebugCheck {
                    label: (*label).to_string(),
                    status: if required { "error" } else { "warn" }.to_string(),
                    detail: "missing".to_string(),
                    warnings: vec![format!("Table '{table}' was not detected.")],
                }
            }
        })
        .collect()
}

fn count_table_rows(connection: &Connection, table_name: &str) -> Result<i64> {
    if !table_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(anyhow!("unsafe table name"));
    }
    let sql = format!("SELECT COUNT(*) FROM {table_name}");
    connection
        .query_row(&sql, [], |row| row.get::<_, i64>(0))
        .with_context(|| format!("failed to count {table_name}"))
}

fn count_foreign_key_issues(connection: &Connection) -> Result<i64> {
    let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
    let mut rows = statement.query([])?;
    let mut count = 0;
    while rows.next()?.is_some() {
        count += 1;
    }
    Ok(count)
}

fn image_attachment_rows(connection: &Connection) -> Result<Vec<DebugImageRow>> {
    let mut statement = connection.prepare(
        "SELECT ma.hash, ma.storage_key
         FROM plugin_entry_media em
         JOIN plugin_media_assets ma ON ma.id = em.media_id
         WHERE ma.deleted_at IS NULL
         ORDER BY em.created_at DESC, em.id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(DebugImageRow {
            hash: row.get(0)?,
            storage_key: row.get(1)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to inspect image attachment files")
}

fn sample_image_attachments(
    db_path: &Path,
    connection: &Connection,
) -> Result<Vec<ImageAttachment>> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT em.entry_uuid
         FROM plugin_entry_media em
         JOIN plugin_media_assets ma ON ma.id = em.media_id
         WHERE ma.deleted_at IS NULL
         ORDER BY em.created_at DESC, em.id DESC
         LIMIT ?1",
    )?;
    let uuids = statement
        .query_map([SAMPLE_IMAGE_LIMIT], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(
        images::list_images_for_entries_for_database(db_path, uuids)?
            .entries
            .into_iter()
            .flat_map(|entry| entry.images)
            .take(SAMPLE_IMAGE_LIMIT as usize)
            .collect(),
    )
}

fn sidecar_size(path: &Path, suffix: &str) -> Option<u64> {
    let sidecar = PathBuf::from(format!("{}{}", path.to_string_lossy(), suffix));
    fs::metadata(sidecar).ok().map(|metadata| metadata.len())
}

fn test_directory_writable(path: &Path) -> bool {
    let test_path = path.join(".capsule-debug-write-test");
    match fs::write(&test_path, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(test_path);
            true
        }
        Err(_) => false,
    }
}

fn normalize_log_level(value: Option<&str>) -> String {
    match value.map(str::trim).map(str::to_lowercase).as_deref() {
        Some("warn" | "warning") => "warn".to_string(),
        Some("error") => "error".to_string(),
        _ => "info".to_string(),
    }
}

fn normalize_log_message(value: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(anyhow!("Debug log message cannot be empty."));
    }
    Ok(normalized.chars().take(2_000).collect())
}

fn read_recent_logs(limit: usize) -> Result<Vec<DebugLogEntry>> {
    let path = debug_log_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut entries = raw
        .lines()
        .filter_map(|line| serde_json::from_str::<DebugLogEntry>(line).ok())
        .collect::<Vec<_>>();
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }
    Ok(entries)
}

fn debug_log_path() -> PathBuf {
    app_settings_directory().join("debug.log")
}

fn debug_bundle_directory() -> PathBuf {
    app_settings_directory().join("diagnostics")
}

fn app_settings_directory() -> PathBuf {
    db::local_path_settings_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Debug, Clone, Copy)]
enum Redaction {
    None,
    Json,
}

fn push_optional_file(
    items: &mut Vec<ZipItem>,
    warnings: &mut Vec<String>,
    name: &str,
    path: &Path,
    redaction: Redaction,
) {
    if !path.exists() {
        warnings.push(format!(
            "Optional diagnostic file was not found: {}",
            path.display()
        ));
        return;
    }
    match fs::read(path) {
        Ok(bytes) => {
            let bytes = match redaction {
                Redaction::None => bytes,
                Redaction::Json => redact_json_bytes(bytes).unwrap_or_else(|error| {
                    warnings.push(format!("Could not redact {}: {error}", path.display()));
                    Vec::new()
                }),
            };
            if !bytes.is_empty() {
                items.push(ZipItem {
                    name: name.to_string(),
                    bytes,
                });
            }
        }
        Err(error) => warnings.push(format!("Could not read {}: {error}", path.display())),
    }
}

fn push_debug_log_file(items: &mut Vec<ZipItem>, warnings: &mut Vec<String>, path: &Path) {
    if path.exists() {
        push_optional_file(items, warnings, "debug.log", path, Redaction::None);
        return;
    }

    items.push(ZipItem {
        name: "debug.log".to_string(),
        bytes: b"No debug log entries were recorded before this diagnostics bundle was created.\n"
            .to_vec(),
    });
}

fn redact_json_bytes(bytes: Vec<u8>) -> Result<Vec<u8>> {
    let mut value: JsonValue = serde_json::from_slice(&bytes)?;
    redact_json_value(&mut value);
    Ok(serde_json::to_vec_pretty(&value)?)
}

fn redact_json_value(value: &mut JsonValue) {
    match value {
        JsonValue::Object(object) => {
            for (key, value) in object {
                if key_is_secret(key) {
                    *value = JsonValue::String("[redacted]".to_string());
                } else {
                    redact_json_value(value);
                }
            }
        }
        JsonValue::Array(items) => {
            for item in items {
                redact_json_value(item);
            }
        }
        _ => {}
    }
}

fn key_is_secret(key: &str) -> bool {
    let key = key.to_lowercase();
    key.contains("token")
        || key.contains("secret")
        || key.contains("password")
        || key.contains("api_key")
        || key.ends_with("_key")
}

fn diagnostic_readme() -> String {
    [
        "Capsule diagnostics bundle",
        "",
        "This bundle contains structured health checks, local settings with secret-like fields redacted, Capsule config with secret-like fields redacted, and the app debug log.",
        "It does not include the journal database, entry text exports, image files, or API keys.",
        "",
    ]
    .join("\n")
}

fn environment_summary() -> String {
    let env_names = [
        "CAPSULE_DB_PATH",
        "CAPSULE_IMAGES_MEDIA_ROOT",
        "CAPSULE_COVERS_ROOT",
        "CAPSULE_BACKUP_DIR",
        "CAPSULE_SYNC_PATH",
        "CAPSULE_GITHUB_GIST_ID",
        "CAPSULE_GITHUB_GIST_TOKEN",
        "CAPSULE_PATH_SETTINGS_PATH",
        "CAPSULE_CONFIG_PATH",
        "CAPSULE_ENV_PATH",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "OPENROUTER_API_KEY",
    ];
    let db_path = db::resolve_database_path();
    let local_settings = db::read_local_path_settings();
    let config_path = settings::config_path_for_database(&db_path);
    let image_media_root =
        images::get_image_media_root().unwrap_or_else(|error| format!("unavailable ({error})"));
    let sync_path = env_setting("CAPSULE_SYNC_PATH").or_else(|| local_settings.sync_path.clone());
    let github_gist_id =
        env_setting("CAPSULE_GITHUB_GIST_ID").or_else(|| local_settings.github_gist_id.clone());
    let auto_sync_enabled = local_settings.auto_sync_enabled.unwrap_or(false);
    let auto_sync_interval_minutes = local_settings
        .auto_sync_interval_minutes
        .unwrap_or(15)
        .clamp(1, 24 * 60);

    let mut lines = Vec::new();
    lines.push("Environment overrides (process only)".to_string());
    lines.push(
        "If these are not set, Capsule may still be using saved Settings values, defaults, OS credential store keys, or local .env keys."
            .to_string(),
    );
    lines.extend(
        env_names
            .into_iter()
            .map(|name| format!("{name}: {}", env_state(name))),
    );
    lines.push(String::new());
    lines.push("Effective Capsule settings".to_string());
    lines.push(format!("Database path: {}", db::path_to_string(&db_path)));
    lines.push(format!("Image media root: {image_media_root}"));
    lines.push(format!(
        "Cover Wall image root: {}",
        images::get_cover_wall_root()
    ));
    lines.push(format!(
        "Backup directory: {}",
        db::path_to_string(&db::backup_directory_for_database(&db_path))
    ));
    lines.push(format!(
        "Sync folder: {}",
        display_optional_setting(sync_path.as_deref())
    ));
    lines.push(format!(
        "GitHub Gist ID: {}",
        display_redacted_identifier(github_gist_id.as_deref())
    ));
    lines.push(format!(
        "GitHub Gist token: {}",
        configured_source(
            "CAPSULE_GITHUB_GIST_TOKEN",
            local_settings.github_gist_token.as_deref(),
            "saved settings"
        )
    ));
    lines.push(format!(
        "Auto sync: {}",
        if auto_sync_enabled {
            format!("enabled every {auto_sync_interval_minutes} minutes")
        } else {
            "disabled".to_string()
        }
    ));
    lines.push(format!(
        "Local path settings file: {}",
        db::path_to_string(&db::local_path_settings_path())
    ));
    lines.push(format!(
        "Capsule config file: {}",
        db::path_to_string(&config_path)
    ));
    lines.push(String::new());
    lines.extend(ai_environment_summary_lines(&db_path));
    lines.join("\n") + "\n"
}

fn ai_environment_summary_lines(db_path: &Path) -> Vec<String> {
    let mut lines = vec!["AI settings".to_string()];
    let settings = match ai_config::get_ai_settings_for_database(db_path) {
        Ok(settings) => settings,
        Err(error) => {
            lines.push(format!("AI settings: unavailable ({error})"));
            return lines;
        }
    };
    let provider_statuses = match ai_config::get_ai_provider_status() {
        Ok(statuses) => statuses,
        Err(error) => {
            lines.push(format!("Provider status: unavailable ({error})"));
            Vec::new()
        }
    };
    let selected_provider_label = provider_statuses
        .iter()
        .find(|status| status.provider == settings.cloud_provider)
        .map(|status| status.label.as_str())
        .unwrap_or(settings.cloud_provider.as_str());
    let selected_model =
        ai_config::selected_model_for_provider(&settings, &settings.cloud_provider);

    lines.push(format!(
        "Selected provider: {selected_provider_label} ({})",
        settings.cloud_provider
    ));
    lines.push(format!("Selected model: {selected_model}"));
    lines.push(format!(
        "Default context limit: {}",
        settings
            .default_context_limit
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "all".to_string())
    ));
    lines.push(format!(
        "Default since: {}",
        settings.default_since.as_deref().unwrap_or("not set")
    ));
    lines.push(format!(
        "Default until: {}",
        settings.default_until.as_deref().unwrap_or("not set")
    ));
    for warning in &settings.warnings {
        lines.push(format!("AI settings warning: {warning}"));
    }
    lines.push(String::new());
    lines.push("AI credential status".to_string());
    if provider_statuses.is_empty() {
        lines.push("Provider status: unavailable".to_string());
        return lines;
    }
    for status in provider_statuses {
        let state = if status.configured {
            format!(
                "configured via {}",
                status.key_source.as_deref().unwrap_or("unknown source")
            )
        } else {
            "not configured".to_string()
        };
        lines.push(format!(
            "{}: {state} (model: {})",
            status.label, status.selected_model
        ));
        if !status.configured {
            if let Some(reason) = status.missing_reason {
                lines.push(format!("{} detail: {reason}", status.label));
            }
        }
    }
    lines
}

fn env_state(name: &str) -> &'static str {
    if env_setting(name).is_some() {
        "set"
    } else {
        "not set"
    }
}

fn env_setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .and_then(|value| normalize_summary_string(Some(&value)))
}

fn configured_source(env_name: &str, saved_value: Option<&str>, saved_label: &str) -> String {
    if env_setting(env_name).is_some() {
        return "configured via environment".to_string();
    }
    if normalize_summary_string(saved_value).is_some() {
        return format!("configured via {saved_label}");
    }
    "not configured".to_string()
}

fn display_optional_setting(value: Option<&str>) -> String {
    normalize_summary_string(value).unwrap_or_else(|| "not configured".to_string())
}

fn display_redacted_identifier(value: Option<&str>) -> String {
    match normalize_summary_string(value) {
        Some(value) => format!("configured ({})", redact_identifier(&value)),
        None => "not configured".to_string(),
    }
}

fn redact_identifier(value: &str) -> String {
    let value = value.trim();
    let char_count = value.chars().count();
    if char_count <= 8 {
        return "[configured]".to_string();
    }
    let prefix = value.chars().take(4).collect::<String>();
    let suffix = value
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

fn normalize_summary_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn write_zip(path: &Path, items: &[ZipItem]) -> Result<()> {
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut central_directory = Vec::new();

    for item in items {
        let offset = file.stream_position()?;
        let name_bytes = item.name.as_bytes();
        let name_length = u16::try_from(name_bytes.len()).context("ZIP filename is too long")?;
        let size = u32::try_from(item.bytes.len()).context("ZIP item is too large")?;
        let crc = crc32(&item.bytes);

        write_u32(&mut file, 0x0403_4b50)?;
        write_u16(&mut file, ZIP_VERSION_NEEDED)?;
        write_u16(&mut file, ZIP_UTF8_FLAG)?;
        write_u16(&mut file, ZIP_STORED_METHOD)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, DOS_DATE_1980_01_01)?;
        write_u32(&mut file, crc)?;
        write_u32(&mut file, size)?;
        write_u32(&mut file, size)?;
        write_u16(&mut file, name_length)?;
        write_u16(&mut file, 0)?;
        file.write_all(name_bytes)?;
        file.write_all(&item.bytes)?;

        central_directory.push(CentralDirectoryEntry {
            name: item.name.clone(),
            crc32: crc,
            compressed_size: size,
            uncompressed_size: size,
            local_header_offset: u32::try_from(offset).context("ZIP archive is too large")?,
        });
    }

    let central_start = file.stream_position()?;
    for entry in &central_directory {
        let name_bytes = entry.name.as_bytes();
        let name_length = u16::try_from(name_bytes.len()).context("ZIP filename is too long")?;
        write_u32(&mut file, 0x0201_4b50)?;
        write_u16(&mut file, ZIP_VERSION_NEEDED)?;
        write_u16(&mut file, ZIP_VERSION_NEEDED)?;
        write_u16(&mut file, ZIP_UTF8_FLAG)?;
        write_u16(&mut file, ZIP_STORED_METHOD)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, DOS_DATE_1980_01_01)?;
        write_u32(&mut file, entry.crc32)?;
        write_u32(&mut file, entry.compressed_size)?;
        write_u32(&mut file, entry.uncompressed_size)?;
        write_u16(&mut file, name_length)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u16(&mut file, 0)?;
        write_u32(&mut file, 0)?;
        write_u32(&mut file, entry.local_header_offset)?;
        file.write_all(name_bytes)?;
    }
    let central_end = file.stream_position()?;
    let central_size = central_end - central_start;
    let entry_count = u16::try_from(central_directory.len()).context("ZIP has too many entries")?;

    write_u32(&mut file, 0x0605_4b50)?;
    write_u16(&mut file, 0)?;
    write_u16(&mut file, 0)?;
    write_u16(&mut file, entry_count)?;
    write_u16(&mut file, entry_count)?;
    write_u32(
        &mut file,
        u32::try_from(central_size).context("ZIP archive is too large")?,
    )?;
    write_u32(
        &mut file,
        u32::try_from(central_start).context("ZIP archive is too large")?,
    )?;
    write_u16(&mut file, 0)?;
    file.flush()?;
    Ok(())
}

fn write_u16(file: &mut File, value: u16) -> Result<()> {
    Ok(file.write_all(&value.to_le_bytes())?)
}

fn write_u32(file: &mut File, value: u32) -> Result<()> {
    Ok(file.write_all(&value.to_le_bytes())?)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff;
    for byte in bytes {
        let mut value = (crc ^ u32::from(*byte)) & 0xff;
        for _ in 0..8 {
            value = if value & 1 == 1 {
                0xedb8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
        }
        crc = (crc >> 8) ^ value;
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn crc32_matches_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn zip_writer_emits_local_and_central_headers() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("bundle.zip");
        write_zip(
            &path,
            &[ZipItem {
                name: "diagnostics.json".to_string(),
                bytes: json!({ "ok": true }).to_string().into_bytes(),
            }],
        )
        .expect("zip");
        let bytes = fs::read(path).expect("read zip");
        assert!(bytes.starts_with(&0x0403_4b50u32.to_le_bytes()));
        assert!(bytes
            .windows(4)
            .any(|window| window == 0x0201_4b50u32.to_le_bytes()));
        assert!(bytes.ends_with(&[0, 0]));
    }

    #[test]
    fn missing_debug_log_adds_placeholder_without_warning() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing_log = temp.path().join("debug.log");
        let mut items = Vec::new();
        let mut warnings = Vec::new();

        push_debug_log_file(&mut items, &mut warnings, &missing_log);

        assert!(warnings.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "debug.log");
        assert!(String::from_utf8_lossy(&items[0].bytes).contains("No debug log entries"));
    }

    #[test]
    fn redacted_identifier_keeps_diagnostic_hint_without_full_value() {
        assert_eq!(
            redact_identifier("5f3d7920150f11c4578bdcf97cb8e4b1"),
            "5f3d...e4b1"
        );
        assert_eq!(redact_identifier("short"), "[configured]");
    }
}
