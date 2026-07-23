use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt, fs,
    hash::{Hash, Hasher},
    io::Write,
    marker::PhantomData,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use reqwest::blocking::{Client, RequestBuilder};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{
    de::{DeserializeOwned, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::{json, Map, Value as JsonValue};

use crate::{
    backup, db, entries, location,
    models::{SyncRunRequest, SyncRunResponse},
};

const MAIN_SYNC_FILE: &str = "capsule_sync.json";
const THREADS_SYNC_FILE: &str = "capsule_threads_sync.json";
const AI_CHATS_SYNC_FILE: &str = "capsule_ai_chats_sync.json";
const MOBILE_NOTES_FILE: &str = "mobile_notes.json";
const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2026-03-10";
const GITHUB_USER_AGENT: &str = "Capsule-Tauri";
const GITHUB_SYNC_FILES: [&str; 3] = [MAIN_SYNC_FILE, THREADS_SYNC_FILE, AI_CHATS_SYNC_FILE];
const MAIN_SYNC_VERSION: i64 = 6;
const SYNC_RETRY_LIMIT: usize = 3;
const SYNC_HISTORY_RETENTION_LIMIT: i64 = 200;

#[derive(Debug, Clone)]
struct GithubGistConfig {
    gist_id: String,
    token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSignature {
    exists: bool,
    content_hash: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct SyncCounts {
    imported: i64,
    updated: i64,
    deleted: i64,
    exported: i64,
    conflicts: Vec<String>,
    parts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubGistResponse {
    #[serde(default)]
    files: HashMap<String, GithubGistFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubGistFile {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    raw_url: Option<String>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Debug, Clone, Default)]
struct GithubGistPullResult {
    pulled: bool,
    mobile_notes: Vec<MobileNote>,
}

#[derive(Debug, Clone, Deserialize)]
struct MobileNote {
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    mood: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    when: String,
    #[serde(default)]
    location: Option<MobileNoteLocation>,
}

#[derive(Debug, Clone, Deserialize)]
struct MobileNoteLocation {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct MainSyncPayload {
    #[serde(default)]
    version: Option<i64>,
    #[serde(default)]
    entries: Vec<SyncEntry>,
    #[serde(default)]
    deleted_uuids: Vec<String>,
    #[serde(flatten)]
    extra: Map<String, JsonValue>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum SyncValue<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<T> SyncValue<T> {
    fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }

    fn from_option(value: Option<T>) -> Self {
        value.map_or(Self::Null, Self::Value)
    }
}

impl<T> Serialize for SyncValue<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Missing | Self::Null => serializer.serialize_none(),
            Self::Value(value) => serializer.serialize_some(value),
        }
    }
}

impl<'de, T> Deserialize<'de> for SyncValue<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SyncValueVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for SyncValueVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = SyncValue<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sync field value or null")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(SyncValue::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(SyncValue::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                T::deserialize(deserializer).map(SyncValue::Value)
            }
        }

        deserializer.deserialize_option(SyncValueVisitor(PhantomData))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct SyncEntry {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    title: SyncValue<String>,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    summary: SyncValue<String>,
    #[serde(default)]
    text: String,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    content_format: SyncValue<String>,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    mood: SyncValue<String>,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    tags: SyncValue<Vec<String>>,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    starred: SyncValue<bool>,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    pinned: SyncValue<bool>,
    #[serde(default, skip_serializing_if = "SyncValue::is_missing")]
    hidden: SyncValue<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ImageSyncPayload {
    #[serde(default)]
    assets: Vec<ImageAssetPayload>,
    #[serde(default)]
    attachments: Vec<ImageAttachmentPayload>,
    #[serde(default)]
    deleted_attachments: Vec<DeletedImageAttachment>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ImageAssetPayload {
    #[serde(default)]
    hash: String,
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    bytes: i64,
    #[serde(default)]
    width: i64,
    #[serde(default)]
    height: i64,
    #[serde(default)]
    storage_backend: String,
    #[serde(default)]
    storage_key: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    deleted_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ImageAttachmentPayload {
    #[serde(default)]
    entry_uuid: String,
    #[serde(default)]
    asset_hash: String,
    #[serde(default)]
    position: i64,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    alt_text: Option<String>,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DeletedImageAttachment {
    #[serde(default)]
    entry_uuid: String,
    #[serde(default)]
    asset_hash: String,
    #[serde(default)]
    position: i64,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    alt_text: Option<String>,
    #[serde(default)]
    deleted_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct LocationSyncPayload {
    #[serde(default)]
    supported: bool,
    #[serde(default)]
    locations: Vec<LocationPayload>,
    #[serde(default)]
    deleted_locations: Vec<DeletedLocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct LocationPayload {
    #[serde(default)]
    entry_uuid: String,
    #[serde(default)]
    latitude: f64,
    #[serde(default)]
    longitude: f64,
    #[serde(default)]
    place_name: Option<String>,
    #[serde(default)]
    place_details: Option<String>,
    #[serde(default)]
    source: String,
    #[serde(default)]
    weather_temp_c: Option<f64>,
    #[serde(default)]
    weather_temp_f: Option<f64>,
    #[serde(default)]
    weather_condition: Option<String>,
    #[serde(default)]
    weather_icon: Option<String>,
    #[serde(default)]
    weather_humidity: Option<i64>,
    #[serde(default)]
    weather_wind_kph: Option<f64>,
    #[serde(default)]
    weather_fetched_at: Option<String>,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DeletedLocation {
    #[serde(default)]
    entry_uuid: String,
    #[serde(default)]
    deleted_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct LibrarySyncPayload {
    #[serde(default)]
    templates: Vec<LibraryTemplatePayload>,
    #[serde(default)]
    prompts: Vec<LibraryPromptPayload>,
    #[serde(default)]
    deleted_templates: Vec<DeletedLibraryItem>,
    #[serde(default)]
    deleted_prompts: Vec<DeletedLibraryItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct LibraryTemplatePayload {
    #[serde(default)]
    slug: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    intro_text: String,
    #[serde(default)]
    sections: Vec<String>,
    #[serde(default = "default_true")]
    is_active: bool,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct LibraryPromptPayload {
    #[serde(default)]
    slug: String,
    #[serde(default)]
    prompt_text: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_true")]
    is_active: bool,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DeletedLibraryItem {
    #[serde(default)]
    slug: String,
    #[serde(default)]
    deleted_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct MoodSyncPayload {
    #[serde(default)]
    moods: Vec<MoodPayload>,
    #[serde(default)]
    deleted_moods: Vec<DeletedMood>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct MoodPayload {
    #[serde(default)]
    name: String,
    #[serde(default)]
    sentiment_score: f64,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DeletedMood {
    #[serde(default)]
    name: String,
    #[serde(default)]
    deleted_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ThreadSyncPayload {
    #[serde(default = "thread_sync_version")]
    version: i64,
    #[serde(default)]
    continuations: Vec<ThreadContinuation>,
    #[serde(default)]
    deleted_continuations: Vec<DeletedContinuation>,
    #[serde(default)]
    titles: Vec<ThreadTitle>,
    #[serde(default)]
    deleted_titles: Vec<DeletedThreadText>,
    #[serde(default)]
    summaries: Vec<ThreadSummary>,
    #[serde(default)]
    deleted_summaries: Vec<DeletedThreadText>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ThreadContinuation {
    #[serde(default)]
    child_entry_uuid: String,
    #[serde(default)]
    parent_entry_uuid: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DeletedContinuation {
    #[serde(default)]
    child_entry_uuid: String,
    #[serde(default)]
    deleted_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ThreadTitle {
    #[serde(default)]
    thread_root_uuid: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ThreadSummary {
    #[serde(default)]
    thread_root_uuid: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DeletedThreadText {
    #[serde(default)]
    thread_root_uuid: String,
    #[serde(default)]
    deleted_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct AiChatSyncPayload {
    #[serde(default = "ai_chat_sync_version")]
    version: i64,
    #[serde(default)]
    conversations: Vec<AiConversationPayload>,
    #[serde(default)]
    messages: Vec<AiMessagePayload>,
    #[serde(default)]
    deleted_conversations: Vec<DeletedAiConversation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct AiConversationPayload {
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    preview: Option<String>,
    #[serde(default)]
    cloud_provider: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    scope_identifiers: JsonValue,
    #[serde(default)]
    context_limit: Option<i64>,
    #[serde(default)]
    since: Option<String>,
    #[serde(default)]
    until: Option<String>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct AiMessagePayload {
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    conversation_uuid: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    sort_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DeletedAiConversation {
    #[serde(default)]
    conversation_uuid: String,
    #[serde(default)]
    deleted_at: String,
}

#[derive(Debug)]
enum SyncAttempt {
    Complete(SyncRunResponse),
    Retry,
}

pub fn run_sync(input: Option<SyncRunRequest>) -> Result<SyncRunResponse> {
    let local_settings = db::read_local_path_settings();
    let gist_config = resolve_github_gist_config(&local_settings);
    let sync_dir = resolve_sync_dir(
        input.and_then(|input| normalize_string(input.sync_path)),
        gist_config.is_some(),
    )?;
    fs::create_dir_all(&sync_dir)
        .with_context(|| format!("failed to create {}", sync_dir.display()))?;
    let sync_file = sync_dir.join(MAIN_SYNC_FILE);
    let db_path = db::resolve_database_path();
    let (github_gist_pulled, mobile_note_ids) = if let Some(config) = gist_config.as_ref() {
        match pull_github_gist_files(config, &sync_dir).and_then(|pull| {
            let mobile_note_ids = stage_mobile_notes(&sync_dir, &pull.mobile_notes)?;
            Ok((pull.pulled, mobile_note_ids))
        }) {
            Ok(result) => result,
            Err(error) => {
                if let Err(status_error) =
                    record_sync_error(&db_path, &sync_file, &error.to_string())
                {
                    eprintln!("[Sync] Failed to record GitHub Gist pull error: {status_error}");
                }
                return Err(error);
            }
        }
    } else {
        (false, HashSet::new())
    };

    let mut response = match run_sync_with_backup_if_needed(&db_path, &sync_dir) {
        Ok(response) => response,
        Err(error) => {
            if let Err(status_error) = record_sync_error(&db_path, &sync_file, &error.to_string()) {
                eprintln!("[Sync] Failed to record sync error: {status_error}");
            }
            return Err(error);
        }
    };
    response.github_gist_pulled = github_gist_pulled;
    response.github_gist_pushed = if let Some(config) = gist_config.as_ref() {
        if config.token.is_some() {
            match push_github_gist_files(
                config,
                &PathBuf::from(&response.sync_path),
                &mobile_note_ids,
            ) {
                Ok(pushed) => pushed,
                Err(error) => {
                    if let Err(status_error) = record_sync_error(
                        &db_path,
                        &PathBuf::from(&response.sync_file_path),
                        &error.to_string(),
                    ) {
                        eprintln!("[Sync] Failed to record GitHub Gist push error: {status_error}");
                    }
                    return Err(error);
                }
            }
        } else {
            false
        }
    } else {
        false
    };
    if let Some(summary) = github_summary_note(
        response.github_gist_pulled,
        response.github_gist_pushed,
        gist_config
            .as_ref()
            .and_then(|config| config.token.as_ref())
            .is_none()
            && gist_config.is_some(),
    ) {
        response.summary = append_summary_note(&response.summary, &summary);
        if let Err(error) =
            record_sync_summary_override(&db_path, &response.completed_at, &response.summary)
        {
            eprintln!("[Sync] Failed to update GitHub Gist sync summary: {error}");
        }
    }

    Ok(response)
}

fn resolve_sync_dir(override_path: Option<String>, gist_configured: bool) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(PathBuf::from(path));
    }
    if let Ok(path) = std::env::var("CAPSULE_SYNC_PATH") {
        if let Some(path) = normalize_string(Some(path)) {
            return Ok(PathBuf::from(path));
        }
    }
    if let Some(path) = db::read_local_path_settings().sync_path {
        return Ok(PathBuf::from(path));
    }
    if gist_configured {
        return Ok(db::local_github_gist_sync_cache_path());
    }
    Err(anyhow!(
        "No sync path or GitHub Gist configured. Set a sync folder or GitHub Gist in Settings first."
    ))
}

fn resolve_github_gist_config(settings: &db::LocalPathSettings) -> Option<GithubGistConfig> {
    let gist_id = std::env::var("CAPSULE_GITHUB_GIST_ID")
        .ok()
        .and_then(|value| normalize_string(Some(value)))
        .or_else(|| settings.github_gist_id.clone())?;
    let token = std::env::var("CAPSULE_GITHUB_GIST_TOKEN")
        .ok()
        .and_then(|value| normalize_string(Some(value)))
        .or_else(|| settings.github_gist_token.clone());

    Some(GithubGistConfig { gist_id, token })
}

fn github_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build GitHub client")
}

fn github_request(builder: RequestBuilder, token: Option<&str>) -> RequestBuilder {
    let builder = builder
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", GITHUB_USER_AGENT)
        .header("X-GitHub-Api-Version", GITHUB_API_VERSION);
    if let Some(token) = token {
        builder.bearer_auth(token)
    } else {
        builder
    }
}

fn fetch_github_gist(client: &Client, config: &GithubGistConfig) -> Result<GithubGistResponse> {
    let url = format!("{GITHUB_API_BASE}/gists/{}", config.gist_id);
    let response = github_request(client.get(url), config.token.as_deref())
        .send()
        .context("failed to fetch GitHub Gist")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(anyhow!(
            "GitHub Gist fetch failed with status {status}: {}",
            truncate_for_display(&body, 240)
        ));
    }

    response.json().context("failed to parse GitHub Gist")
}

fn pull_github_gist_files(
    config: &GithubGistConfig,
    sync_dir: &Path,
) -> Result<GithubGistPullResult> {
    let client = github_client()?;
    let gist = fetch_github_gist(&client, config)?;
    let mut result = GithubGistPullResult::default();
    for filename in GITHUB_SYNC_FILES {
        if let Some(file) = gist.files.get(filename) {
            let content = github_gist_file_content(&client, config, file)
                .with_context(|| format!("failed to fetch {filename} from GitHub Gist"))?;
            validate_sync_file_json(filename, &content)?;
            write_text_replace(&sync_dir.join(filename), &content)
                .with_context(|| format!("failed to write fetched GitHub Gist file {filename}"))?;
            result.pulled = true;
        }
    }

    if let Some(file) = gist.files.get(MOBILE_NOTES_FILE) {
        let content = github_gist_file_content(&client, config, file)
            .with_context(|| format!("failed to fetch {MOBILE_NOTES_FILE} from GitHub Gist"))?;
        result.mobile_notes = serde_json::from_str(&content).with_context(|| {
            format!("{MOBILE_NOTES_FILE} from GitHub Gist is not a valid note array")
        })?;
        result.pulled = true;
    }

    Ok(result)
}

fn stage_mobile_notes(sync_dir: &Path, mobile_notes: &[MobileNote]) -> Result<HashSet<String>> {
    if mobile_notes.is_empty() {
        return Ok(HashSet::new());
    }

    let sync_file = sync_dir.join(MAIN_SYNC_FILE);
    let mut main_payload = read_main_payload(&sync_file)?;
    let mut locations =
        parse_extra_payload::<LocationSyncPayload>(&main_payload.extra, "locations")?
            .unwrap_or_default();
    let mut entry_uuids = main_payload
        .entries
        .iter()
        .filter_map(|entry| normalize_string(entry.uuid.clone()))
        .collect::<HashSet<_>>();
    let mut location_uuids = locations
        .locations
        .iter()
        .filter_map(|location| normalize_string(Some(location.entry_uuid.clone())))
        .collect::<HashSet<_>>();
    let mut acknowledged_ids = HashSet::new();
    let mut entries_changed = false;
    let mut locations_changed = false;

    for mobile_note in mobile_notes {
        let client_id = normalize_string(Some(mobile_note.client_id.clone()))
            .ok_or_else(|| anyhow!("{MOBILE_NOTES_FILE} contains a note without a client_id"))?;
        let text = mobile_note.text.replace("\r\n", "\n").replace('\r', "\n");
        if text.trim().is_empty() {
            return Err(anyhow!(
                "{MOBILE_NOTES_FILE} note {client_id} does not contain text"
            ));
        }
        let when = normalize_string(Some(mobile_note.when.clone())).ok_or_else(|| {
            anyhow!("{MOBILE_NOTES_FILE} note {client_id} does not contain a timestamp")
        })?;
        let entry_uuid = format!("mobile_{client_id}");

        if let Some(location) = mobile_note.location.as_ref() {
            if !location.latitude.is_finite()
                || !location.longitude.is_finite()
                || !(-90.0..=90.0).contains(&location.latitude)
                || !(-180.0..=180.0).contains(&location.longitude)
            {
                return Err(anyhow!(
                    "{MOBILE_NOTES_FILE} note {client_id} contains invalid coordinates"
                ));
            }
            if location_uuids.insert(entry_uuid.clone()) {
                locations.locations.push(LocationPayload {
                    entry_uuid: entry_uuid.clone(),
                    latitude: location.latitude,
                    longitude: location.longitude,
                    source: "mobile".to_string(),
                    created_at: when.clone(),
                    ..LocationPayload::default()
                });
                locations.supported = true;
                locations_changed = true;
            }
        }

        if entry_uuids.insert(entry_uuid.clone()) {
            main_payload.entries.push(SyncEntry {
                uuid: Some(entry_uuid),
                created_at: when.clone(),
                updated_at: Some(when),
                text,
                content_format: SyncValue::Value("plain".to_string()),
                mood: SyncValue::from_option(normalize_string(mobile_note.mood.clone())),
                tags: SyncValue::Value(normalize_tags(mobile_note.tags.clone())),
                starred: SyncValue::Value(false),
                pinned: SyncValue::Value(false),
                hidden: SyncValue::Value(false),
                ..SyncEntry::default()
            });
            entries_changed = true;
        }
        acknowledged_ids.insert(client_id);
    }

    if locations_changed {
        main_payload
            .extra
            .insert("locations".to_string(), serde_json::to_value(locations)?);
    }
    if entries_changed || locations_changed {
        write_json_replace(&sync_file, &main_payload)?;
    }

    Ok(acknowledged_ids)
}

fn github_gist_file_content(
    client: &Client,
    config: &GithubGistConfig,
    file: &GithubGistFile,
) -> Result<String> {
    if !file.truncated {
        if let Some(content) = file.content.as_ref() {
            return Ok(content.clone());
        }
    }

    let raw_url = file
        .raw_url
        .as_ref()
        .ok_or_else(|| anyhow!("GitHub Gist file content was truncated without a raw URL"))?;
    let response = github_request(client.get(raw_url), config.token.as_deref())
        .send()
        .context("failed to fetch raw GitHub Gist file")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(anyhow!(
            "GitHub Gist raw file fetch failed with status {status}: {}",
            truncate_for_display(&body, 240)
        ));
    }

    response
        .text()
        .context("failed to read raw GitHub Gist file")
}

fn push_github_gist_files(
    config: &GithubGistConfig,
    sync_dir: &Path,
    acknowledged_mobile_note_ids: &HashSet<String>,
) -> Result<bool> {
    let token = config
        .token
        .as_deref()
        .ok_or_else(|| anyhow!("GitHub Gist token is required to push sync files"))?;
    let mut files = Map::new();
    for filename in GITHUB_SYNC_FILES {
        let path = sync_dir.join(filename);
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            validate_sync_file_json(filename, &content)?;
            files.insert(filename.to_string(), json!({ "content": content }));
        }
    }
    let client = github_client()?;
    if let Some(content) =
        mobile_notes_after_acknowledgement(&client, config, acknowledged_mobile_note_ids)?
    {
        files.insert(MOBILE_NOTES_FILE.to_string(), json!({ "content": content }));
    }
    if files.is_empty() {
        return Ok(false);
    }

    let url = format!("{GITHUB_API_BASE}/gists/{}", config.gist_id);
    let response = github_request(client.patch(url), Some(token))
        .json(&json!({ "files": files }))
        .send()
        .context("failed to update GitHub Gist")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(anyhow!(
            "GitHub Gist update failed with status {status}: {}",
            truncate_for_display(&body, 240)
        ));
    }

    Ok(true)
}

fn mobile_notes_after_acknowledgement(
    client: &Client,
    config: &GithubGistConfig,
    acknowledged_ids: &HashSet<String>,
) -> Result<Option<String>> {
    if acknowledged_ids.is_empty() {
        return Ok(None);
    }

    let gist = fetch_github_gist(client, config)?;
    let Some(file) = gist.files.get(MOBILE_NOTES_FILE) else {
        return Ok(None);
    };
    let content = github_gist_file_content(client, config, file)
        .with_context(|| format!("failed to refetch {MOBILE_NOTES_FILE} from GitHub Gist"))?;
    filter_acknowledged_mobile_notes(&content, acknowledged_ids)
}

fn filter_acknowledged_mobile_notes(
    content: &str,
    acknowledged_ids: &HashSet<String>,
) -> Result<Option<String>> {
    let value: JsonValue = serde_json::from_str(content)
        .with_context(|| format!("{MOBILE_NOTES_FILE} from GitHub Gist is not valid JSON"))?;
    let JsonValue::Array(mut notes) = value else {
        return Err(anyhow!(
            "{MOBILE_NOTES_FILE} from GitHub Gist is not a note array"
        ));
    };
    let original_len = notes.len();
    notes.retain(|note| {
        note.get("client_id")
            .and_then(JsonValue::as_str)
            .map(str::trim)
            .is_none_or(|client_id| !acknowledged_ids.contains(client_id))
    });
    if notes.len() == original_len {
        return Ok(None);
    }

    Ok(Some(format!(
        "{}\n",
        serde_json::to_string_pretty(&JsonValue::Array(notes))?
    )))
}

fn validate_sync_file_json(filename: &str, content: &str) -> Result<()> {
    serde_json::from_str::<JsonValue>(content)
        .with_context(|| format!("{filename} from GitHub Gist is not valid JSON"))?;
    Ok(())
}

fn write_text_replace(path: &Path, content: &str) -> Result<()> {
    let tmp_path = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000)
    ));
    write_bytes_replace(path, &tmp_path, content.as_bytes())
}

fn write_bytes_replace(path: &Path, tmp_path: &Path, content: &[u8]) -> Result<()> {
    let write_result = (|| -> Result<()> {
        let mut file = fs::File::create(tmp_path)
            .with_context(|| format!("failed to create {}", tmp_path.display()))?;
        file.write_all(content)
            .with_context(|| format!("failed to write {}", tmp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to flush {}", tmp_path.display()))?;
        drop(file);
        replace_file(tmp_path, path)
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(tmp_path);
    }
    write_result
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| {
            format!(
                "failed to atomically replace {} with {}",
                destination.display(),
                source.display()
            )
        });
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    fs::rename(source, destination).with_context(|| {
        format!(
            "failed to atomically replace {} with {}",
            destination.display(),
            source.display()
        )
    })
}

fn github_summary_note(pulled: bool, pushed: bool, read_only: bool) -> Option<String> {
    match (pulled, pushed, read_only) {
        (true, true, _) => Some("GitHub Gist pulled and pushed".to_string()),
        (true, false, true) => Some("GitHub Gist pulled read-only".to_string()),
        (true, false, false) => Some("GitHub Gist pulled".to_string()),
        (false, true, _) => Some("GitHub Gist pushed".to_string()),
        (false, false, true) => Some("GitHub Gist checked read-only".to_string()),
        (false, false, false) => None,
    }
}

fn append_summary_note(summary: &str, note: &str) -> String {
    if summary.trim().is_empty() {
        note.to_string()
    } else {
        format!("{summary}, {note}")
    }
}

fn run_sync_with_backup_if_needed(db_path: &Path, sync_dir: &Path) -> Result<SyncRunResponse> {
    if let Some(response) = run_sync_without_backup_if_unchanged(db_path, sync_dir)? {
        return Ok(response);
    }

    let sync_dir = sync_dir.to_path_buf();
    let guarded = backup::with_database_backup_for_database(db_path, "sync.run", move |path| {
        let mut response = run_sync_with_retries(path, &sync_dir)?;
        let enriched = location::enrich_pending_mobile_locations(path)?;
        if enriched > 0 {
            response.exported_count = refresh_sync_files_from_database(path, &sync_dir)?;
            let label = if enriched == 1 {
                "1 mobile location enriched".to_string()
            } else {
                format!("{enriched} mobile locations enriched")
            };
            response.summary = append_summary_note(&response.summary, &label);
            record_sync_summary_override(path, &response.completed_at, &response.summary)?;
        }
        Ok(response)
    })?;
    Ok(guarded.value)
}

fn run_sync_without_backup_if_unchanged(
    db_path: &Path,
    sync_dir: &Path,
) -> Result<Option<SyncRunResponse>> {
    entries::ensure_entry_ids_for_database(db_path)?;
    if location::has_pending_mobile_location_enrichment(db_path)? {
        return Ok(None);
    }
    let sync_file = sync_dir.join(MAIN_SYNC_FILE);
    let threads_sync_file = sync_dir.join(THREADS_SYNC_FILE);
    let ai_chat_sync_file = sync_dir.join(AI_CHATS_SYNC_FILE);
    let starting_signatures = sync_file_signatures(sync_dir)?;

    let remote_main = read_main_payload(&sync_file)?;
    let remote_threads = read_json_file::<ThreadSyncPayload>(&threads_sync_file)?;
    let remote_ai_chats = read_json_file::<AiChatSyncPayload>(&ai_chat_sync_file)?;
    let (main_payload, thread_payload, ai_payload) = build_local_sync_payloads(
        db_path,
        remote_main.extra.clone(),
        remote_main.deleted_uuids.clone(),
    )?;

    if !sync_payloads_match(
        &remote_main,
        &remote_threads,
        &remote_ai_chats,
        &main_payload,
        &thread_payload,
        &ai_payload,
    )? {
        return Ok(None);
    }

    if sync_file_signatures(sync_dir)? != starting_signatures {
        return Ok(None);
    }

    write_json_replace(&sync_file, &main_payload)?;
    write_json_replace(&threads_sync_file, &thread_payload)?;
    write_json_replace(&ai_chat_sync_file, &ai_payload)?;

    let counts = SyncCounts {
        exported: main_payload.entries.len() as i64,
        ..SyncCounts::default()
    };
    let completed_at = current_timestamp_seconds();
    let summary = sync_summary(&counts);
    record_sync_success(db_path, &sync_file, &counts, &summary, &completed_at, None)?;

    Ok(Some(SyncRunResponse {
        sync_path: db::path_to_string(sync_dir),
        sync_file_path: db::path_to_string(&sync_file),
        github_gist_pulled: false,
        github_gist_pushed: false,
        imported_count: counts.imported,
        updated_count: counts.updated,
        deleted_count: counts.deleted,
        exported_count: counts.exported,
        conflict_count: counts.conflicts.len() as i64,
        summary,
        completed_at,
    }))
}

fn refresh_sync_files_from_database(db_path: &Path, sync_dir: &Path) -> Result<i64> {
    let sync_file = sync_dir.join(MAIN_SYNC_FILE);
    let threads_sync_file = sync_dir.join(THREADS_SYNC_FILE);
    let ai_chat_sync_file = sync_dir.join(AI_CHATS_SYNC_FILE);
    let current_main = read_main_payload(&sync_file)?;
    let (main_payload, thread_payload, ai_payload) =
        build_local_sync_payloads(db_path, current_main.extra, current_main.deleted_uuids)?;
    let exported_count = main_payload.entries.len() as i64;
    write_json_replace(&sync_file, &main_payload)?;
    write_json_replace(&threads_sync_file, &thread_payload)?;
    write_json_replace(&ai_chat_sync_file, &ai_payload)?;
    Ok(exported_count)
}

fn build_local_sync_payloads(
    db_path: &Path,
    mut extra: Map<String, JsonValue>,
    remote_deleted_uuids: Vec<String>,
) -> Result<(MainSyncPayload, ThreadSyncPayload, AiChatSyncPayload)> {
    let connection = db::open_read_only_connection(db_path)?;
    extra.insert(
        "images".to_string(),
        serde_json::to_value(build_images_payload(&connection)?)?,
    );
    extra.insert(
        "locations".to_string(),
        serde_json::to_value(build_locations_payload(&connection)?)?,
    );
    extra.insert(
        "library".to_string(),
        serde_json::to_value(build_library_payload(&connection)?)?,
    );
    extra.insert(
        "moods".to_string(),
        serde_json::to_value(build_moods_payload(&connection)?)?,
    );
    Ok((
        build_main_payload(&connection, extra, remote_deleted_uuids)?,
        build_threads_payload(&connection)?,
        build_ai_chats_payload(&connection)?,
    ))
}

fn sync_payloads_match(
    remote_main: &MainSyncPayload,
    remote_threads: &ThreadSyncPayload,
    remote_ai_chats: &AiChatSyncPayload,
    local_main: &MainSyncPayload,
    local_threads: &ThreadSyncPayload,
    local_ai_chats: &AiChatSyncPayload,
) -> Result<bool> {
    Ok(
        serde_json::to_value(remote_main)? == serde_json::to_value(local_main)?
            && serde_json::to_value(remote_threads)? == serde_json::to_value(local_threads)?
            && serde_json::to_value(remote_ai_chats)? == serde_json::to_value(local_ai_chats)?,
    )
}

fn run_sync_with_retries(db_path: &Path, sync_dir: &Path) -> Result<SyncRunResponse> {
    let mut last_error = None;
    for _ in 0..SYNC_RETRY_LIMIT {
        match run_sync_once(db_path, sync_dir)? {
            SyncAttempt::Complete(response) => return Ok(response),
            SyncAttempt::Retry => {
                last_error = Some("Sync file changed during merge; retrying with the latest file.");
                continue;
            }
        }
    }
    Err(anyhow!(
        "{}",
        last_error.unwrap_or("Sync file kept changing during merge.")
    ))
}

fn run_sync_once(db_path: &Path, sync_dir: &Path) -> Result<SyncAttempt> {
    entries::ensure_entry_ids_for_database(db_path)?;
    let sync_file = sync_dir.join(MAIN_SYNC_FILE);
    let threads_sync_file = sync_dir.join(THREADS_SYNC_FILE);
    let ai_chat_sync_file = sync_dir.join(AI_CHATS_SYNC_FILE);
    let starting_signatures = sync_file_signatures(sync_dir)?;

    let remote_main = read_main_payload(&sync_file)?;
    let remote_threads = read_json_file::<ThreadSyncPayload>(&threads_sync_file)?;
    let remote_ai_chats = read_json_file::<AiChatSyncPayload>(&ai_chat_sync_file)?;
    let remote_images = parse_extra_payload::<ImageSyncPayload>(&remote_main.extra, "images")?;
    let remote_locations =
        parse_extra_payload::<LocationSyncPayload>(&remote_main.extra, "locations")?;
    let remote_library = parse_extra_payload::<LibrarySyncPayload>(&remote_main.extra, "library")?;
    let remote_moods = parse_extra_payload::<MoodSyncPayload>(&remote_main.extra, "moods")?;

    let mut counts = SyncCounts::default();
    let (main_payload, thread_payload, ai_payload) = {
        let mut connection = db::open_read_write_connection(db_path)?;
        ensure_sync_schema(&connection)?;
        let tx = connection.transaction()?;

        apply_remote_entry_deletes(&tx, &remote_main.deleted_uuids, &mut counts)?;
        apply_remote_entries(&tx, &remote_main.entries, &mut counts)?;
        if let Some(payload) = remote_images.as_ref() {
            apply_images_payload(&tx, payload, &mut counts)?;
        }
        if let Some(payload) = remote_locations.as_ref() {
            apply_locations_payload(&tx, payload, &mut counts)?;
        }
        if let Some(payload) = remote_library.as_ref() {
            apply_library_payload(&tx, payload, &mut counts)?;
        }
        if let Some(payload) = remote_moods.as_ref() {
            apply_moods_payload(&tx, payload, &mut counts)?;
        }
        apply_threads_payload(&tx, &remote_threads, &mut counts)?;
        apply_ai_chats_payload(&tx, &remote_ai_chats, &mut counts)?;

        let mut extra = remote_main.extra;
        extra.insert(
            "images".to_string(),
            serde_json::to_value(build_images_payload(&tx)?)?,
        );
        extra.insert(
            "locations".to_string(),
            serde_json::to_value(build_locations_payload(&tx)?)?,
        );
        extra.insert(
            "library".to_string(),
            serde_json::to_value(build_library_payload(&tx)?)?,
        );
        extra.insert(
            "moods".to_string(),
            serde_json::to_value(build_moods_payload(&tx)?)?,
        );
        let main_payload = build_main_payload(&tx, extra, remote_main.deleted_uuids)?;
        let thread_payload = build_threads_payload(&tx)?;
        let ai_payload = build_ai_chats_payload(&tx)?;
        counts.exported = main_payload.entries.len() as i64;
        if sync_file_signatures(sync_dir)? != starting_signatures {
            return Ok(SyncAttempt::Retry);
        }
        tx.commit()?;
        (main_payload, thread_payload, ai_payload)
    };

    if sync_file_signatures(sync_dir)? != starting_signatures {
        return Ok(SyncAttempt::Retry);
    }

    write_json_replace(&sync_file, &main_payload)?;
    write_json_replace(&threads_sync_file, &thread_payload)?;
    write_json_replace(&ai_chat_sync_file, &ai_payload)?;

    let completed_at = current_timestamp_seconds();
    let summary = sync_summary(&counts);
    record_sync_success(
        db_path,
        &sync_file,
        &counts,
        &summary,
        &completed_at,
        counts.conflicts.last().cloned(),
    )?;

    Ok(SyncAttempt::Complete(SyncRunResponse {
        sync_path: db::path_to_string(sync_dir),
        sync_file_path: db::path_to_string(&sync_file),
        github_gist_pulled: false,
        github_gist_pushed: false,
        imported_count: counts.imported,
        updated_count: counts.updated,
        deleted_count: counts.deleted,
        exported_count: counts.exported,
        conflict_count: counts.conflicts.len() as i64,
        summary,
        completed_at,
    }))
}

fn read_main_payload(path: &Path) -> Result<MainSyncPayload> {
    if !path.exists() {
        return Ok(MainSyncPayload {
            version: Some(MAIN_SYNC_VERSION),
            ..MainSyncPayload::default()
        });
    }
    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: JsonValue = serde_json::from_slice(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    if let JsonValue::Array(entries) = value {
        return Ok(MainSyncPayload {
            version: Some(1),
            entries: entries
                .into_iter()
                .filter_map(|value| serde_json::from_value::<SyncEntry>(value).ok())
                .collect(),
            ..MainSyncPayload::default()
        });
    }
    serde_json::from_value::<MainSyncPayload>(value)
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn parse_extra_payload<T>(extra: &Map<String, JsonValue>, key: &str) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    match extra.get(key) {
        Some(value) => Ok(Some(
            serde_json::from_value::<T>(value.clone())
                .with_context(|| format!("invalid {key} sync payload"))?,
        )),
        None => Ok(None),
    }
}

fn read_json_file<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de> + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn write_json_replace<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let tmp_path = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000)
    ));
    let payload = serde_json::to_vec_pretty(value)?;
    write_bytes_replace(path, &tmp_path, &payload)
}

fn file_signature(path: &Path) -> Result<FileSignature> {
    match fs::read(path) {
        Ok(bytes) => {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            Ok(FileSignature {
                exists: true,
                content_hash: Some(hasher.finish()),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileSignature {
            exists: false,
            content_hash: None,
        }),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn sync_file_signatures(sync_dir: &Path) -> Result<(FileSignature, FileSignature, FileSignature)> {
    Ok((
        file_signature(&sync_dir.join(MAIN_SYNC_FILE))?,
        file_signature(&sync_dir.join(THREADS_SYNC_FILE))?,
        file_signature(&sync_dir.join(AI_CHATS_SYNC_FILE))?,
    ))
}

fn ensure_sync_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sync_tombstones (
            uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_status (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            last_successful_sync_at TEXT,
            last_sync_file_path TEXT,
            last_sync_file_size_bytes INTEGER,
            last_sync_imported INTEGER NOT NULL DEFAULT 0,
            last_sync_updated INTEGER NOT NULL DEFAULT 0,
            last_sync_deleted INTEGER NOT NULL DEFAULT 0,
            last_sync_total INTEGER NOT NULL DEFAULT 0,
            last_sync_summary TEXT,
            last_conflict_count INTEGER NOT NULL DEFAULT 0,
            last_conflict_summary TEXT,
            last_sync_error TEXT
        );
        INSERT OR IGNORE INTO sync_status (
            id, last_sync_imported, last_sync_updated, last_sync_deleted,
            last_sync_total, last_conflict_count
        ) VALUES (1, 0, 0, 0, 0, 0);
        CREATE TABLE IF NOT EXISTS sync_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('success', 'failed')),
            sync_file_path TEXT,
            imported_count INTEGER NOT NULL DEFAULT 0,
            updated_count INTEGER NOT NULL DEFAULT 0,
            deleted_count INTEGER NOT NULL DEFAULT 0,
            exported_count INTEGER NOT NULL DEFAULT 0,
            conflict_count INTEGER NOT NULL DEFAULT 0,
            summary TEXT,
            details TEXT,
            error TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_sync_history_timestamp
            ON sync_history(timestamp DESC, id DESC);
        CREATE TABLE IF NOT EXISTS entry_continuations (
            child_entry_uuid TEXT PRIMARY KEY,
            parent_entry_uuid TEXT NOT NULL,
            updated_at TEXT
        );
        CREATE TABLE IF NOT EXISTS entry_thread_titles (
            thread_root_uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS entry_thread_summaries (
            thread_root_uuid TEXT PRIMARY KEY,
            summary TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_entry_continuation_tombstones (
            child_entry_uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_entry_thread_title_tombstones (
            thread_root_uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_entry_thread_summary_tombstones (
            thread_root_uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        ",
    )?;
    if table_exists(connection, "ai_conversations")? {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS sync_ai_conversation_tombstones (
                conversation_uuid TEXT PRIMARY KEY,
                deleted_at TEXT NOT NULL
            )",
            [],
        )?;
    }
    Ok(())
}

fn apply_remote_entry_deletes(
    connection: &Connection,
    remote_deleted_uuids: &[String],
    counts: &mut SyncCounts,
) -> Result<()> {
    let now = current_timestamp_minutes();
    let mut seen = HashSet::new();
    for uuid in remote_deleted_uuids
        .iter()
        .filter_map(|uuid| normalize_string(Some(uuid.clone())))
        .filter(|uuid| seen.insert(uuid.clone()))
    {
        let entry_id = connection
            .query_row("SELECT id FROM entries WHERE uuid = ?1", [&uuid], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?;
        if let Some(entry_id) = entry_id {
            delete_entry_for_sync(connection, entry_id, &uuid, &now)?;
            counts.deleted += 1;
        } else {
            record_entry_tombstone(connection, &uuid, &now)?;
        }
    }
    Ok(())
}

fn apply_remote_entries(
    connection: &Connection,
    entries: &[SyncEntry],
    counts: &mut SyncCounts,
) -> Result<()> {
    let mut items = entries.to_vec();
    items.sort_by(|left, right| {
        normalized_timestamp(left.updated_at.as_deref(), &left.created_at)
            .cmp(&normalized_timestamp(
                right.updated_at.as_deref(),
                &right.created_at,
            ))
            .then_with(|| left.uuid.cmp(&right.uuid))
    });

    for item in items {
        let Some(uuid) = normalize_string(item.uuid.clone()) else {
            continue;
        };
        let created_at = normalized_timestamp(Some(&item.created_at), &current_timestamp_minutes());
        let updated_at = normalized_timestamp(item.updated_at.as_deref(), &created_at);
        let text = item.text.replace("\r\n", "\n").replace('\r', "\n");
        if text.trim().is_empty() {
            continue;
        }

        if entry_tombstone_at(connection, &uuid)?.is_some() {
            counts.conflicts.push(format!(
                "Kept local deletion for {uuid}; remote entry still existed in the sync file."
            ));
            continue;
        }

        let existing = connection
            .query_row(
                "SELECT id, created_at, updated_at, text, content_format, title, summary, mood,
                        starred, pinned, hidden
                 FROM entries
                 WHERE uuid = ?1",
                [&uuid],
                |row| {
                    Ok(ExistingEntry {
                        id: row.get(0)?,
                        created_at: row.get(1)?,
                        updated_at: row.get(2)?,
                        text: row.get(3)?,
                        content_format: row.get(4)?,
                        title: row.get(5)?,
                        summary: row.get(6)?,
                        mood: row.get(7)?,
                        starred: row.get::<_, i64>(8)? != 0,
                        pinned: row.get::<_, i64>(9)? != 0,
                        hidden: row.get::<_, i64>(10)? != 0,
                    })
                },
            )
            .optional()?;

        if let Some(existing) = existing {
            let local_updated_at =
                normalized_timestamp(existing.updated_at.as_deref(), &existing.created_at);
            if updated_at < local_updated_at {
                counts.conflicts.push(format!(
                    "Kept local changes for {uuid} because the remote version was older."
                ));
                continue;
            }
            let local_tags = load_entry_tags(connection, existing.id)?;
            let content_format =
                resolve_sync_content_format(&item.content_format, &existing.content_format);
            let title = resolve_sync_optional_text(&item.title, existing.title.as_deref());
            let summary = resolve_sync_optional_text(&item.summary, existing.summary.as_deref());
            let mood = resolve_sync_optional_text(&item.mood, existing.mood.as_deref());
            let tags = resolve_sync_tags(&item.tags, &local_tags);
            let starred = resolve_sync_bool(&item.starred, existing.starred);
            let pinned = resolve_sync_bool(&item.pinned, existing.pinned);
            let hidden = resolve_sync_bool(&item.hidden, existing.hidden);
            if entry_changed(
                &existing,
                &text,
                &content_format,
                title.as_deref(),
                summary.as_deref(),
                mood.as_deref(),
                starred,
                pinned,
                hidden,
                &tags,
                &local_tags,
            ) {
                update_entry_for_sync(
                    connection,
                    existing.id,
                    &text,
                    &content_format,
                    title.as_deref(),
                    summary.as_deref(),
                    mood.as_deref(),
                    starred,
                    pinned,
                    hidden,
                    &updated_at,
                )?;
                replace_entry_tags(connection, existing.id, &tags)?;
                counts.updated += 1;
            }
        } else if !entry_exists_by_created_text(connection, &created_at, &text)? {
            let content_format = resolve_sync_content_format(&item.content_format, "plain");
            let title = resolve_sync_optional_text(&item.title, None);
            let summary = resolve_sync_optional_text(&item.summary, None);
            let mood = resolve_sync_optional_text(&item.mood, None);
            let tags = resolve_sync_tags(&item.tags, &[]);
            let starred = resolve_sync_bool(&item.starred, false);
            let pinned = resolve_sync_bool(&item.pinned, false);
            let hidden = resolve_sync_bool(&item.hidden, false);
            let entry_id = insert_entry_for_sync(
                connection,
                &uuid,
                &created_at,
                &updated_at,
                &text,
                &content_format,
                title.as_deref(),
                summary.as_deref(),
                mood.as_deref(),
                starred,
                pinned,
                hidden,
            )?;
            replace_entry_tags(connection, entry_id, &tags)?;
            counts.imported += 1;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ExistingEntry {
    id: i64,
    created_at: String,
    updated_at: Option<String>,
    text: String,
    content_format: String,
    title: Option<String>,
    summary: Option<String>,
    mood: Option<String>,
    starred: bool,
    pinned: bool,
    hidden: bool,
}

#[allow(clippy::too_many_arguments)]
fn entry_changed(
    existing: &ExistingEntry,
    text: &str,
    content_format: &str,
    title: Option<&str>,
    summary: Option<&str>,
    mood: Option<&str>,
    starred: bool,
    pinned: bool,
    hidden: bool,
    tags: &[String],
    local_tags: &[String],
) -> bool {
    existing.text != text
        || normalize_content_format(Some(&existing.content_format)) != content_format
        || existing.title.as_deref() != title
        || existing.summary.as_deref() != summary
        || existing.mood.as_deref() != mood
        || existing.starred != starred
        || existing.pinned != pinned
        || existing.hidden != hidden
        || local_tags != tags
}

#[allow(clippy::too_many_arguments)]
fn insert_entry_for_sync(
    connection: &Connection,
    uuid: &str,
    created_at: &str,
    updated_at: &str,
    text: &str,
    content_format: &str,
    title: Option<&str>,
    summary: Option<&str>,
    mood: Option<&str>,
    starred: bool,
    pinned: bool,
    hidden: bool,
) -> Result<i64> {
    let text_plain = build_text_plain(text);
    let entry_id = next_entry_id(connection)?;
    connection.execute(
        "INSERT INTO entries
            (id, uuid, created_at, updated_at, text, text_plain, content_format,
             title, summary, mood, starred, pinned, hidden)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            entry_id,
            uuid,
            created_at,
            updated_at,
            text,
            text_plain,
            content_format,
            title,
            summary,
            mood,
            bool_to_int(starred),
            bool_to_int(pinned),
            bool_to_int(hidden),
        ],
    )?;
    refresh_fts_for_entry(connection, entry_id, &text_plain)?;
    Ok(entry_id)
}

#[allow(clippy::too_many_arguments)]
fn update_entry_for_sync(
    connection: &Connection,
    entry_id: i64,
    text: &str,
    content_format: &str,
    title: Option<&str>,
    summary: Option<&str>,
    mood: Option<&str>,
    starred: bool,
    pinned: bool,
    hidden: bool,
    updated_at: &str,
) -> Result<()> {
    let text_plain = build_text_plain(text);
    connection.execute(
        "UPDATE entries
         SET text = ?1,
             text_plain = ?2,
             content_format = ?3,
             title = ?4,
             summary = ?5,
             mood = ?6,
             starred = ?7,
             pinned = ?8,
             hidden = ?9,
             updated_at = ?10
         WHERE id = ?11",
        params![
            text,
            text_plain,
            content_format,
            title,
            summary,
            mood,
            bool_to_int(starred),
            bool_to_int(pinned),
            bool_to_int(hidden),
            updated_at,
            entry_id,
        ],
    )?;
    refresh_fts_for_entry(connection, entry_id, &text_plain)?;
    Ok(())
}

fn delete_entry_for_sync(
    connection: &Connection,
    entry_id: i64,
    uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    record_entry_tombstone(connection, uuid, deleted_at)?;
    delete_if_table_exists(connection, "entry_tags", "entry_id", entry_id)?;
    delete_by_uuid_if_table_exists(connection, "entry_continuations", "child_entry_uuid", uuid)?;
    delete_by_uuid_if_table_exists(connection, "entry_continuations", "parent_entry_uuid", uuid)?;
    delete_by_uuid_if_table_exists(connection, "entry_thread_titles", "thread_root_uuid", uuid)?;
    delete_by_uuid_if_table_exists(
        connection,
        "entry_thread_summaries",
        "thread_root_uuid",
        uuid,
    )?;
    delete_by_uuid_if_table_exists(connection, "plugin_entry_locations", "entry_uuid", uuid)?;
    delete_by_uuid_if_table_exists(connection, "plugin_entry_media", "entry_uuid", uuid)?;
    connection.execute("DELETE FROM entries WHERE id = ?1", [entry_id])?;
    rebuild_entries_fts(connection)?;
    Ok(())
}

fn record_entry_tombstone(connection: &Connection, uuid: &str, deleted_at: &str) -> Result<()> {
    connection.execute(
        "INSERT INTO sync_tombstones (uuid, deleted_at)
         VALUES (?1, ?2)
         ON CONFLICT(uuid)
         DO UPDATE SET deleted_at = excluded.deleted_at",
        params![uuid, deleted_at],
    )?;
    Ok(())
}

fn entry_tombstone_at(connection: &Connection, uuid: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT deleted_at FROM sync_tombstones WHERE uuid = ?1",
            [uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn entry_exists_by_created_text(
    connection: &Connection,
    created_at: &str,
    text: &str,
) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM entries WHERE created_at = ?1 AND text = ?2 LIMIT 1",
            params![created_at, text],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn replace_entry_tags(connection: &Connection, entry_id: i64, tags: &[String]) -> Result<()> {
    if !table_exists(connection, "tags")? || !table_exists(connection, "entry_tags")? {
        return Ok(());
    }
    connection.execute("DELETE FROM entry_tags WHERE entry_id = ?1", [entry_id])?;
    for tag in tags {
        connection.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])?;
        if let Some(tag_id) = connection
            .query_row("SELECT id FROM tags WHERE name = ?1", [tag], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?
        {
            connection.execute(
                "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                params![entry_id, tag_id],
            )?;
        }
    }
    Ok(())
}

fn load_entry_tags(connection: &Connection, entry_id: i64) -> Result<Vec<String>> {
    if !table_exists(connection, "tags")? || !table_exists(connection, "entry_tags")? {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare(
        "SELECT t.name
         FROM tags t
         JOIN entry_tags et ON et.tag_id = t.id
         WHERE et.entry_id = ?1
         ORDER BY lower(t.name) ASC",
    )?;
    let rows = statement.query_map([entry_id], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn refresh_fts_for_entry(connection: &Connection, entry_id: i64, text_plain: &str) -> Result<()> {
    if !table_exists(connection, "entries_fts")? {
        return Ok(());
    }
    connection
        .execute("DELETE FROM entries_fts WHERE rowid = ?1", [entry_id])
        .ok();
    connection
        .execute(
            "INSERT INTO entries_fts(rowid, text) VALUES (?1, ?2)",
            params![entry_id, text_plain],
        )
        .ok();
    Ok(())
}

fn next_entry_id(connection: &Connection) -> Result<i64> {
    connection
        .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM entries", [], |row| {
            row.get(0)
        })
        .context("failed to calculate next entry id")
}

fn rebuild_entries_fts(connection: &Connection) -> Result<()> {
    if !table_exists(connection, "entries_fts")? {
        return Ok(());
    }
    connection.execute("DELETE FROM entries_fts", [])?;
    connection
        .execute(
            "INSERT INTO entries_fts(rowid, text)
             SELECT id, COALESCE(NULLIF(text_plain, ''), text, '') FROM entries",
            [],
        )
        .ok();
    Ok(())
}

fn build_main_payload(
    connection: &Connection,
    mut extra: Map<String, JsonValue>,
    remote_deleted_uuids: Vec<String>,
) -> Result<MainSyncPayload> {
    for reserved in ["version", "entries", "deleted_uuids"] {
        extra.remove(reserved);
    }
    let mut statement = connection.prepare(
        "SELECT id, uuid, created_at, updated_at, title, summary, text, content_format,
                mood, starred, pinned, hidden
         FROM entries
         ORDER BY datetime(created_at) ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SyncEntry {
            id: row.get(0)?,
            uuid: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            title: SyncValue::from_option(row.get(4)?),
            summary: SyncValue::from_option(row.get(5)?),
            text: row.get(6)?,
            content_format: SyncValue::Value(normalize_content_format(
                row.get::<_, Option<String>>(7)?.as_deref(),
            )),
            mood: SyncValue::from_option(row.get(8)?),
            tags: SyncValue::Value(Vec::new()),
            starred: SyncValue::Value(row.get::<_, i64>(9)? != 0),
            pinned: SyncValue::Value(row.get::<_, i64>(10)? != 0),
            hidden: SyncValue::Value(row.get::<_, i64>(11)? != 0),
        })
    })?;
    let mut entries = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    for entry in entries.iter_mut() {
        if let Some(id) = entry.id {
            entry.tags = SyncValue::Value(load_entry_tags(connection, id)?);
        }
    }

    let mut deleted = remote_deleted_uuids
        .into_iter()
        .filter_map(|uuid| normalize_string(Some(uuid)))
        .collect::<HashSet<_>>();
    if table_exists(connection, "sync_tombstones")? {
        let mut statement = connection.prepare("SELECT uuid FROM sync_tombstones")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            deleted.insert(row?);
        }
    }
    let mut deleted_uuids = deleted.into_iter().collect::<Vec<_>>();
    deleted_uuids.sort();

    Ok(MainSyncPayload {
        version: Some(MAIN_SYNC_VERSION),
        entries,
        deleted_uuids,
        extra,
    })
}

fn ensure_image_sync_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS plugin_media_assets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            hash TEXT NOT NULL UNIQUE,
            mime_type TEXT NOT NULL,
            bytes INTEGER NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            storage_backend TEXT NOT NULL,
            storage_key TEXT NOT NULL,
            created_at TEXT NOT NULL,
            deleted_at TEXT
        );
        CREATE TABLE IF NOT EXISTS plugin_entry_media (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_uuid TEXT NOT NULL,
            media_id INTEGER NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            caption TEXT,
            alt_text TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (entry_uuid) REFERENCES entries(uuid) ON DELETE CASCADE,
            FOREIGN KEY (media_id) REFERENCES plugin_media_assets(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS sync_image_tombstones (
            entry_uuid TEXT NOT NULL,
            asset_hash TEXT NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            caption TEXT,
            alt_text TEXT,
            deleted_at TEXT NOT NULL,
            PRIMARY KEY (entry_uuid, asset_hash, position, caption, alt_text)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_media_assets_hash ON plugin_media_assets(hash);
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_media_entry_uuid ON plugin_entry_media(entry_uuid);
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_media_media_id ON plugin_entry_media(media_id);
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_media_position ON plugin_entry_media(entry_uuid, position);
        ",
    )?;
    Ok(())
}

fn apply_images_payload(
    connection: &Connection,
    payload: &ImageSyncPayload,
    counts: &mut SyncCounts,
) -> Result<()> {
    if payload.assets.is_empty()
        && payload.attachments.is_empty()
        && payload.deleted_attachments.is_empty()
    {
        return Ok(());
    }

    ensure_image_sync_schema(connection)?;
    let now = current_timestamp_minutes();
    let mut assets_added = 0;
    let mut attachments_added = 0;
    let mut attachments_removed = 0;

    for asset in payload.assets.iter() {
        let Some(hash) = normalize_string(Some(asset.hash.clone())) else {
            continue;
        };
        let Some(mime_type) = normalize_string(Some(asset.mime_type.clone())) else {
            continue;
        };
        let Some(storage_backend) = normalize_string(Some(asset.storage_backend.clone())) else {
            continue;
        };
        let Some(storage_key) = normalize_string(Some(asset.storage_key.clone())) else {
            continue;
        };
        if asset.bytes <= 0 || asset.width <= 0 || asset.height <= 0 {
            continue;
        }
        let created_at = normalized_timestamp(Some(&asset.created_at), &now);
        let deleted_at = asset
            .deleted_at
            .as_deref()
            .and_then(|value| normalize_optional_text(Some(value)));

        let existing = connection
            .query_row(
                "SELECT id, deleted_at FROM plugin_media_assets WHERE hash = ?1",
                [&hash],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;

        if existing.is_none() {
            connection.execute(
                "INSERT INTO plugin_media_assets
                    (hash, mime_type, bytes, width, height, storage_backend, storage_key, created_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    hash,
                    mime_type,
                    asset.bytes,
                    asset.width,
                    asset.height,
                    storage_backend,
                    storage_key,
                    created_at,
                    deleted_at
                ],
            )?;
            assets_added += 1;
        } else {
            let merged_deleted_at = match (existing.and_then(|item| item.1), deleted_at.clone()) {
                (Some(local), None) => Some(local),
                (_, remote) => remote,
            };
            connection.execute(
                "UPDATE plugin_media_assets
                 SET mime_type = ?1, bytes = ?2, width = ?3, height = ?4,
                     storage_backend = ?5, storage_key = ?6, created_at = ?7, deleted_at = ?8
                 WHERE hash = ?9",
                params![
                    mime_type,
                    asset.bytes,
                    asset.width,
                    asset.height,
                    storage_backend,
                    storage_key,
                    created_at,
                    merged_deleted_at,
                    hash
                ],
            )?;
        }
    }

    for deleted in payload.deleted_attachments.iter() {
        let Some(entry_uuid) = normalize_string(Some(deleted.entry_uuid.clone())) else {
            continue;
        };
        let Some(asset_hash) = normalize_string(Some(deleted.asset_hash.clone())) else {
            continue;
        };
        let position = deleted.position.max(0);
        let caption = normalize_string(deleted.caption.clone()).unwrap_or_default();
        let alt_text = normalize_string(deleted.alt_text.clone()).unwrap_or_default();
        let deleted_at = normalized_timestamp(Some(&deleted.deleted_at), &now);

        connection.execute(
            "INSERT INTO sync_image_tombstones
                (entry_uuid, asset_hash, position, caption, alt_text, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(entry_uuid, asset_hash, position, caption, alt_text)
             DO UPDATE SET deleted_at =
                CASE WHEN excluded.deleted_at > deleted_at THEN excluded.deleted_at ELSE deleted_at END",
            params![entry_uuid, asset_hash, position, caption, alt_text, deleted_at],
        )?;

        let mut statement = connection.prepare(
            "SELECT em.id, em.media_id
             FROM plugin_entry_media em
             JOIN plugin_media_assets ma ON ma.id = em.media_id
             WHERE em.entry_uuid = ?1
               AND ma.hash = ?2
               AND em.position = ?3
               AND COALESCE(em.caption, '') = ?4
               AND COALESCE(em.alt_text, '') = ?5",
        )?;
        let rows = statement
            .query_map(
                params![entry_uuid, asset_hash, position, caption, alt_text],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        for (attachment_id, media_id) in rows {
            connection.execute(
                "DELETE FROM plugin_entry_media WHERE id = ?1",
                [attachment_id],
            )?;
            let remaining = connection.query_row(
                "SELECT COUNT(*) FROM plugin_entry_media WHERE media_id = ?1",
                [media_id],
                |row| row.get::<_, i64>(0),
            )?;
            if remaining == 0 {
                connection.execute(
                    "UPDATE plugin_media_assets SET deleted_at = ?1 WHERE id = ?2",
                    params![deleted_at, media_id],
                )?;
            }
            attachments_removed += 1;
        }
    }

    for attachment in payload.attachments.iter() {
        let Some(entry_uuid) = normalize_string(Some(attachment.entry_uuid.clone())) else {
            continue;
        };
        let Some(asset_hash) = normalize_string(Some(attachment.asset_hash.clone())) else {
            continue;
        };
        if !entry_exists(connection, &entry_uuid)? {
            continue;
        }
        let Some(media_id) = connection
            .query_row(
                "SELECT id FROM plugin_media_assets WHERE hash = ?1",
                [&asset_hash],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        else {
            continue;
        };
        let position = attachment.position.max(0);
        let caption = normalize_string(attachment.caption.clone()).unwrap_or_default();
        let alt_text = normalize_string(attachment.alt_text.clone()).unwrap_or_default();
        let created_at = normalized_timestamp(Some(&attachment.created_at), &now);

        let tombstoned = connection
            .query_row(
                "SELECT 1 FROM sync_image_tombstones
                 WHERE entry_uuid = ?1 AND asset_hash = ?2 AND position = ?3
                   AND caption = ?4 AND alt_text = ?5",
                params![entry_uuid, asset_hash, position, caption, alt_text],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if tombstoned {
            continue;
        }

        let deleted_asset = connection
            .query_row(
                "SELECT id, deleted_at FROM plugin_media_assets
                 WHERE hash = ?1 AND deleted_at IS NOT NULL",
                [&asset_hash],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((deleted_media_id, asset_deleted_at)) = deleted_asset {
            let existing_link = connection
                .query_row(
                    "SELECT 1 FROM plugin_entry_media WHERE entry_uuid = ?1 AND media_id = ?2",
                    params![entry_uuid, deleted_media_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !existing_link {
                connection.execute(
                    "INSERT OR IGNORE INTO sync_image_tombstones
                        (entry_uuid, asset_hash, position, caption, alt_text, deleted_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        entry_uuid,
                        asset_hash,
                        position,
                        caption,
                        alt_text,
                        asset_deleted_at
                    ],
                )?;
                continue;
            }
        }

        let existing_attachment = connection
            .query_row(
                "SELECT 1
                 FROM plugin_entry_media em
                 JOIN plugin_media_assets ma ON ma.id = em.media_id
                 WHERE em.entry_uuid = ?1
                   AND ma.hash = ?2
                   AND em.position = ?3
                   AND COALESCE(em.caption, '') = ?4
                   AND COALESCE(em.alt_text, '') = ?5",
                params![entry_uuid, asset_hash, position, caption, alt_text],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if existing_attachment {
            continue;
        }

        connection.execute(
            "INSERT INTO plugin_entry_media (entry_uuid, media_id, position, caption, alt_text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![entry_uuid, media_id, position, caption, alt_text, created_at],
        )?;
        attachments_added += 1;
    }

    if assets_added + attachments_added + attachments_removed > 0 {
        counts.parts.push(format!(
            "images: {assets_added} assets, {attachments_added} links, {attachments_removed} removed"
        ));
    }
    Ok(())
}

fn build_images_payload(connection: &Connection) -> Result<ImageSyncPayload> {
    if !table_exists(connection, "plugin_media_assets")?
        || !table_exists(connection, "plugin_entry_media")?
    {
        return Ok(ImageSyncPayload::default());
    }

    let mut asset_statement = connection.prepare(
        "SELECT DISTINCT ma.hash, ma.mime_type, ma.bytes, ma.width, ma.height,
                ma.storage_backend, ma.storage_key, ma.created_at, ma.deleted_at
         FROM plugin_media_assets ma
         JOIN plugin_entry_media em ON em.media_id = ma.id
         ORDER BY ma.hash ASC",
    )?;
    let assets = asset_statement
        .query_map([], |row| {
            Ok(ImageAssetPayload {
                hash: row.get(0)?,
                mime_type: row.get(1)?,
                bytes: row.get(2)?,
                width: row.get(3)?,
                height: row.get(4)?,
                storage_backend: row.get(5)?,
                storage_key: row.get(6)?,
                created_at: row.get(7)?,
                deleted_at: row.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut attachment_statement = connection.prepare(
        "SELECT em.entry_uuid, ma.hash, em.position, em.caption, em.alt_text, em.created_at
         FROM plugin_entry_media em
         JOIN plugin_media_assets ma ON ma.id = em.media_id
         ORDER BY em.entry_uuid ASC, em.position ASC, em.id ASC",
    )?;
    let attachments = attachment_statement
        .query_map([], |row| {
            Ok(ImageAttachmentPayload {
                entry_uuid: row.get(0)?,
                asset_hash: row.get(1)?,
                position: row.get(2)?,
                caption: row.get(3)?,
                alt_text: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let deleted_attachments = if table_exists(connection, "sync_image_tombstones")? {
        let mut deleted_statement = connection.prepare(
            "SELECT entry_uuid, asset_hash, position, COALESCE(caption, ''), COALESCE(alt_text, ''), deleted_at
             FROM sync_image_tombstones
             ORDER BY deleted_at ASC, entry_uuid ASC, asset_hash ASC, position ASC",
        )?;
        let rows = deleted_statement
            .query_map([], |row| {
                Ok(DeletedImageAttachment {
                    entry_uuid: row.get(0)?,
                    asset_hash: row.get(1)?,
                    position: row.get(2)?,
                    caption: Some(row.get(3)?),
                    alt_text: Some(row.get(4)?),
                    deleted_at: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    Ok(ImageSyncPayload {
        assets,
        attachments,
        deleted_attachments,
    })
}

fn ensure_location_sync_schema(connection: &Connection) -> Result<()> {
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
        CREATE TABLE IF NOT EXISTS sync_location_tombstones (
            entry_uuid TEXT NOT NULL PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_entry_locations_entry_uuid
            ON plugin_entry_locations(entry_uuid);
        ",
    )?;
    Ok(())
}

fn apply_locations_payload(
    connection: &Connection,
    payload: &LocationSyncPayload,
    counts: &mut SyncCounts,
) -> Result<()> {
    if !payload.supported && payload.locations.is_empty() && payload.deleted_locations.is_empty() {
        return Ok(());
    }

    ensure_location_sync_schema(connection)?;
    let now = current_timestamp_minutes();
    let mut added = 0;
    let mut updated = 0;
    let mut removed = 0;

    for deleted in payload.deleted_locations.iter() {
        let Some(entry_uuid) = normalize_string(Some(deleted.entry_uuid.clone())) else {
            continue;
        };
        let deleted_at = normalized_timestamp(Some(&deleted.deleted_at), &now);
        let existing_created_at = connection
            .query_row(
                "SELECT created_at FROM plugin_entry_locations WHERE entry_uuid = ?1",
                [&entry_uuid],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if existing_created_at
            .as_deref()
            .is_some_and(|created_at| created_at > deleted_at.as_str())
        {
            continue;
        }
        connection.execute(
            "INSERT OR REPLACE INTO sync_location_tombstones (entry_uuid, deleted_at)
             VALUES (?1, ?2)",
            params![entry_uuid, deleted_at],
        )?;
        removed += connection.execute(
            "DELETE FROM plugin_entry_locations WHERE entry_uuid = ?1",
            [&entry_uuid],
        )? as i64;
    }

    for location in payload.locations.iter() {
        let Some(entry_uuid) = normalize_string(Some(location.entry_uuid.clone())) else {
            continue;
        };
        if !entry_exists(connection, &entry_uuid)? {
            continue;
        }
        if !location.latitude.is_finite() || !location.longitude.is_finite() {
            continue;
        }
        let source =
            normalize_string(Some(location.source.clone())).unwrap_or_else(|| "auto".to_string());
        let created_at = normalized_timestamp(Some(&location.created_at), &now);
        if let Some(tombstone_at) = location_tombstone_at(connection, &entry_uuid)? {
            if created_at <= tombstone_at {
                continue;
            }
            connection.execute(
                "DELETE FROM sync_location_tombstones WHERE entry_uuid = ?1",
                [&entry_uuid],
            )?;
        }

        let existing = connection
            .query_row(
                "SELECT id, created_at, weather_fetched_at FROM plugin_entry_locations WHERE entry_uuid = ?1",
                [&entry_uuid],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;

        if existing.is_none() {
            connection.execute(
                "INSERT INTO plugin_entry_locations (
                    entry_uuid, latitude, longitude, place_name, place_details, source,
                    weather_temp_c, weather_temp_f, weather_condition, weather_icon,
                    weather_humidity, weather_wind_kph, weather_fetched_at, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    entry_uuid,
                    location.latitude,
                    location.longitude,
                    location.place_name,
                    location.place_details,
                    source,
                    location.weather_temp_c,
                    location.weather_temp_f,
                    location.weather_condition,
                    location.weather_icon,
                    location.weather_humidity,
                    location.weather_wind_kph,
                    location.weather_fetched_at,
                    created_at
                ],
            )?;
            added += 1;
        } else if let Some((_, local_created_at, local_weather_at)) = existing {
            let remote_weather_at = location.weather_fetched_at.as_deref().unwrap_or("");
            let local_weather_at = local_weather_at.as_deref().unwrap_or("");
            let should_update = created_at > local_created_at
                || (created_at == local_created_at && remote_weather_at > local_weather_at);
            if should_update {
                connection.execute(
                    "UPDATE plugin_entry_locations
                     SET latitude = ?1, longitude = ?2, place_name = ?3, place_details = ?4,
                         source = ?5, weather_temp_c = ?6, weather_temp_f = ?7,
                         weather_condition = ?8, weather_icon = ?9, weather_humidity = ?10,
                         weather_wind_kph = ?11, weather_fetched_at = ?12, created_at = ?13
                     WHERE entry_uuid = ?14",
                    params![
                        location.latitude,
                        location.longitude,
                        location.place_name,
                        location.place_details,
                        source,
                        location.weather_temp_c,
                        location.weather_temp_f,
                        location.weather_condition,
                        location.weather_icon,
                        location.weather_humidity,
                        location.weather_wind_kph,
                        location.weather_fetched_at,
                        created_at,
                        entry_uuid
                    ],
                )?;
                updated += 1;
            }
        }
    }

    if added + updated + removed > 0 {
        counts.parts.push(format!(
            "locations: {added} added, {updated} updated, {removed} removed"
        ));
    }
    Ok(())
}

fn build_locations_payload(connection: &Connection) -> Result<LocationSyncPayload> {
    if !table_exists(connection, "plugin_entry_locations")? {
        return Ok(LocationSyncPayload {
            supported: false,
            ..LocationSyncPayload::default()
        });
    }

    let mut statement = connection.prepare(
        "SELECT entry_uuid, latitude, longitude, place_name, place_details, source,
                weather_temp_c, weather_temp_f, weather_condition, weather_icon,
                weather_humidity, weather_wind_kph, weather_fetched_at, created_at
         FROM plugin_entry_locations
         ORDER BY entry_uuid ASC",
    )?;
    let locations = statement
        .query_map([], |row| {
            Ok(LocationPayload {
                entry_uuid: row.get(0)?,
                latitude: row.get(1)?,
                longitude: row.get(2)?,
                place_name: row.get(3)?,
                place_details: row.get(4)?,
                source: row.get(5)?,
                weather_temp_c: row.get(6)?,
                weather_temp_f: row.get(7)?,
                weather_condition: row.get(8)?,
                weather_icon: row.get(9)?,
                weather_humidity: row.get(10)?,
                weather_wind_kph: row.get(11)?,
                weather_fetched_at: row.get(12)?,
                created_at: row.get(13)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let deleted_locations = if table_exists(connection, "sync_location_tombstones")? {
        let mut deleted_statement = connection.prepare(
            "SELECT st.entry_uuid, st.deleted_at
             FROM sync_location_tombstones st
             WHERE NOT EXISTS (
                SELECT 1 FROM plugin_entry_locations pel WHERE pel.entry_uuid = st.entry_uuid
             )
             ORDER BY st.deleted_at ASC, st.entry_uuid ASC",
        )?;
        let rows = deleted_statement
            .query_map([], |row| {
                Ok(DeletedLocation {
                    entry_uuid: row.get(0)?,
                    deleted_at: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    Ok(LocationSyncPayload {
        supported: true,
        locations,
        deleted_locations,
    })
}

fn ensure_library_sync_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS library_templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            description TEXT,
            intro_text TEXT,
            sections_json TEXT NOT NULL DEFAULT '[]',
            is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
            is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS library_prompts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL UNIQUE,
            prompt_text TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'general',
            tags_json TEXT NOT NULL DEFAULT '[]',
            is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
            is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_template_tombstones (
            slug TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_prompt_tombstones (
            slug TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_library_templates_active ON library_templates(is_active, slug);
        CREATE INDEX IF NOT EXISTS idx_library_prompts_active ON library_prompts(is_active, slug);
        ",
    )?;
    Ok(())
}

fn apply_library_payload(
    connection: &Connection,
    payload: &LibrarySyncPayload,
    counts: &mut SyncCounts,
) -> Result<()> {
    if payload.templates.is_empty()
        && payload.prompts.is_empty()
        && payload.deleted_templates.is_empty()
        && payload.deleted_prompts.is_empty()
    {
        return Ok(());
    }

    ensure_library_sync_schema(connection)?;
    let now = current_timestamp_seconds();
    let mut templates_added = 0;
    let mut templates_updated = 0;
    let mut templates_removed = 0;
    let mut prompts_added = 0;
    let mut prompts_updated = 0;
    let mut prompts_removed = 0;

    for item in payload.templates.iter() {
        let Some(slug) = normalize_slug(&item.slug) else {
            continue;
        };
        let Some(name) = normalize_string(Some(item.name.clone())) else {
            continue;
        };
        let incoming_updated = normalized_timestamp(Some(&item.updated_at), &now);
        let created_at = normalized_timestamp(Some(&item.created_at), &incoming_updated);
        let sections_json = serde_json::to_string(&normalized_text_list(&item.sections))?;
        let current = library_template_row(connection, &slug)?;

        if current.is_none() {
            connection.execute(
                "INSERT INTO library_templates
                    (slug, name, description, intro_text, sections_json, is_builtin, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8)",
                params![
                    slug,
                    name,
                    item.description,
                    item.intro_text,
                    sections_json,
                    bool_to_int(item.is_active),
                    created_at,
                    incoming_updated
                ],
            )?;
            connection.execute(
                "DELETE FROM sync_template_tombstones WHERE slug = ?1",
                [&slug],
            )?;
            templates_added += 1;
            continue;
        }

        let Some((is_builtin, local_created, local_updated)) = current else {
            continue;
        };
        if is_builtin {
            continue;
        }
        let local_updated = normalized_timestamp(Some(&local_updated), &local_created);
        if incoming_updated < local_updated {
            continue;
        }
        connection.execute(
            "UPDATE library_templates
             SET name = ?1, description = ?2, intro_text = ?3, sections_json = ?4,
                 is_active = ?5, updated_at = ?6
             WHERE slug = ?7",
            params![
                name,
                item.description,
                item.intro_text,
                sections_json,
                bool_to_int(item.is_active),
                incoming_updated,
                slug
            ],
        )?;
        connection.execute(
            "DELETE FROM sync_template_tombstones WHERE slug = ?1",
            [&slug],
        )?;
        templates_updated += 1;
    }

    for item in payload.prompts.iter() {
        let Some(slug) = normalize_slug(&item.slug) else {
            continue;
        };
        let Some(prompt_text) = normalize_string(Some(item.prompt_text.clone())) else {
            continue;
        };
        let incoming_updated = normalized_timestamp(Some(&item.updated_at), &now);
        let created_at = normalized_timestamp(Some(&item.created_at), &incoming_updated);
        let tags_json = serde_json::to_string(&normalize_tags(item.tags.clone()))?;
        let category =
            normalize_string(Some(item.category.clone())).unwrap_or_else(|| "general".to_string());
        let current = library_prompt_row(connection, &slug)?;

        if current.is_none() {
            connection.execute(
                "INSERT INTO library_prompts
                    (slug, prompt_text, category, tags_json, is_builtin, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7)",
                params![
                    slug,
                    prompt_text,
                    category,
                    tags_json,
                    bool_to_int(item.is_active),
                    created_at,
                    incoming_updated
                ],
            )?;
            connection.execute(
                "DELETE FROM sync_prompt_tombstones WHERE slug = ?1",
                [&slug],
            )?;
            prompts_added += 1;
            continue;
        }

        let Some((is_builtin, local_created, local_updated)) = current else {
            continue;
        };
        if is_builtin {
            continue;
        }
        let local_updated = normalized_timestamp(Some(&local_updated), &local_created);
        if incoming_updated < local_updated {
            continue;
        }
        connection.execute(
            "UPDATE library_prompts
             SET prompt_text = ?1, category = ?2, tags_json = ?3, is_active = ?4, updated_at = ?5
             WHERE slug = ?6",
            params![
                prompt_text,
                category,
                tags_json,
                bool_to_int(item.is_active),
                incoming_updated,
                slug
            ],
        )?;
        connection.execute(
            "DELETE FROM sync_prompt_tombstones WHERE slug = ?1",
            [&slug],
        )?;
        prompts_updated += 1;
    }

    for item in payload.deleted_templates.iter() {
        let Some(slug) = normalize_slug(&item.slug) else {
            continue;
        };
        let deleted_at = normalized_timestamp(Some(&item.deleted_at), &now);
        if let Some((is_builtin, local_created, local_updated)) =
            library_template_row(connection, &slug)?
        {
            let local_updated = normalized_timestamp(Some(&local_updated), &local_created);
            if !is_builtin && deleted_at >= local_updated {
                templates_removed += connection
                    .execute("DELETE FROM library_templates WHERE slug = ?1", [&slug])?
                    as i64;
            }
        }
        connection.execute(
            "INSERT OR REPLACE INTO sync_template_tombstones (slug, deleted_at) VALUES (?1, ?2)",
            params![slug, deleted_at],
        )?;
    }

    for item in payload.deleted_prompts.iter() {
        let Some(slug) = normalize_slug(&item.slug) else {
            continue;
        };
        let deleted_at = normalized_timestamp(Some(&item.deleted_at), &now);
        if let Some((is_builtin, local_created, local_updated)) =
            library_prompt_row(connection, &slug)?
        {
            let local_updated = normalized_timestamp(Some(&local_updated), &local_created);
            if !is_builtin && deleted_at >= local_updated {
                prompts_removed += connection
                    .execute("DELETE FROM library_prompts WHERE slug = ?1", [&slug])?
                    as i64;
            }
        }
        connection.execute(
            "INSERT OR REPLACE INTO sync_prompt_tombstones (slug, deleted_at) VALUES (?1, ?2)",
            params![slug, deleted_at],
        )?;
    }

    let changed = templates_added
        + templates_updated
        + templates_removed
        + prompts_added
        + prompts_updated
        + prompts_removed;
    if changed > 0 {
        counts.parts.push(format!(
            "library: {templates_added}/{templates_updated}/{templates_removed} templates, {prompts_added}/{prompts_updated}/{prompts_removed} prompts"
        ));
    }
    Ok(())
}

fn build_library_payload(connection: &Connection) -> Result<LibrarySyncPayload> {
    if !table_exists(connection, "library_templates")?
        && !table_exists(connection, "library_prompts")?
    {
        return Ok(LibrarySyncPayload::default());
    }

    let templates = if table_exists(connection, "library_templates")? {
        let mut template_statement = connection.prepare(
            "SELECT slug, name, COALESCE(description, ''), COALESCE(intro_text, ''),
                    sections_json, is_active, created_at, updated_at
             FROM library_templates
             WHERE is_builtin = 0
             ORDER BY lower(slug) ASC",
        )?;
        let rows = template_statement
            .query_map([], |row| {
                Ok(LibraryTemplatePayload {
                    slug: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    intro_text: row.get(3)?,
                    sections: json_string_list(row.get::<_, String>(4)?),
                    is_active: row.get::<_, i64>(5)? != 0,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    let prompts = if table_exists(connection, "library_prompts")? {
        let mut prompt_statement = connection.prepare(
            "SELECT slug, prompt_text, category, tags_json, is_active, created_at, updated_at
             FROM library_prompts
             WHERE is_builtin = 0
             ORDER BY lower(slug) ASC",
        )?;
        let rows = prompt_statement
            .query_map([], |row| {
                Ok(LibraryPromptPayload {
                    slug: row.get(0)?,
                    prompt_text: row.get(1)?,
                    category: row.get(2)?,
                    tags: json_string_list(row.get::<_, String>(3)?),
                    is_active: row.get::<_, i64>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    let deleted_templates = query_deleted_library_items(
        connection,
        "sync_template_tombstones",
        "SELECT slug, deleted_at FROM sync_template_tombstones ORDER BY deleted_at ASC, slug ASC",
    )?;
    let deleted_prompts = query_deleted_library_items(
        connection,
        "sync_prompt_tombstones",
        "SELECT slug, deleted_at FROM sync_prompt_tombstones ORDER BY deleted_at ASC, slug ASC",
    )?;

    Ok(LibrarySyncPayload {
        templates,
        prompts,
        deleted_templates,
        deleted_prompts,
    })
}

fn ensure_mood_sync_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS mood_catalog (
            name TEXT PRIMARY KEY COLLATE NOCASE,
            sentiment_score REAL NOT NULL CHECK (sentiment_score >= -1.0 AND sentiment_score <= 1.0),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_mood_tombstones (
            name TEXT PRIMARY KEY COLLATE NOCASE,
            deleted_at TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

fn apply_moods_payload(
    connection: &Connection,
    payload: &MoodSyncPayload,
    counts: &mut SyncCounts,
) -> Result<()> {
    if payload.moods.is_empty() && payload.deleted_moods.is_empty() {
        return Ok(());
    }

    ensure_mood_sync_schema(connection)?;
    let now = current_timestamp_seconds();
    let mut added = 0;
    let mut updated = 0;
    let mut removed = 0;
    let mut entry_moods_cleared = 0;

    for item in payload.moods.iter() {
        let Some(name) = normalize_string(Some(item.name.clone())).map(|name| name.to_lowercase())
        else {
            continue;
        };
        if !item.sentiment_score.is_finite() || !(-1.0..=1.0).contains(&item.sentiment_score) {
            continue;
        }
        let incoming_updated = normalized_timestamp(Some(&item.updated_at), &now);
        let created_at = normalized_timestamp(Some(&item.created_at), &incoming_updated);
        if mood_tombstone_at(connection, &name)?
            .is_some_and(|deleted_at| deleted_at >= incoming_updated)
        {
            continue;
        }

        if let Some((local_created, local_updated)) = mood_catalog_row(connection, &name)? {
            let local_updated = normalized_timestamp(Some(&local_updated), &local_created);
            if incoming_updated < local_updated {
                continue;
            }
            connection.execute(
                "UPDATE mood_catalog
                 SET sentiment_score = ?1, updated_at = ?2
                 WHERE lower(name) = lower(?3)",
                params![item.sentiment_score, incoming_updated, name],
            )?;
            updated += 1;
        } else {
            connection.execute(
                "INSERT INTO mood_catalog (name, sentiment_score, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![name, item.sentiment_score, created_at, incoming_updated],
            )?;
            added += 1;
        }
        connection.execute(
            "DELETE FROM sync_mood_tombstones WHERE lower(name) = lower(?1)",
            [&name],
        )?;
    }

    for item in payload.deleted_moods.iter() {
        let Some(name) = normalize_string(Some(item.name.clone())).map(|name| name.to_lowercase())
        else {
            continue;
        };
        let deleted_at = normalized_timestamp(Some(&item.deleted_at), &now);
        if let Some((local_created, local_updated)) = mood_catalog_row(connection, &name)? {
            let local_updated = normalized_timestamp(Some(&local_updated), &local_created);
            if deleted_at >= local_updated {
                removed += connection.execute(
                    "DELETE FROM mood_catalog WHERE lower(name) = lower(?1)",
                    [&name],
                )? as i64;
            }
        }
        if table_exists(connection, "entries")? {
            entry_moods_cleared += connection.execute(
                "UPDATE entries
                 SET mood = NULL, updated_at = ?1
                 WHERE lower(trim(mood)) = lower(?2)
                   AND COALESCE(NULLIF(updated_at, ''), created_at) <= ?1",
                params![deleted_at, name],
            )? as i64;
        }
        connection.execute(
            "INSERT INTO sync_mood_tombstones (name, deleted_at)
             VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET deleted_at = excluded.deleted_at
             WHERE excluded.deleted_at >= sync_mood_tombstones.deleted_at",
            params![name, deleted_at],
        )?;
    }

    if added + updated + removed + entry_moods_cleared > 0 {
        counts.parts.push(format!(
            "moods: {added} added, {updated} updated, {removed} removed, {entry_moods_cleared} entry values cleared"
        ));
    }
    Ok(())
}

fn build_moods_payload(connection: &Connection) -> Result<MoodSyncPayload> {
    let moods = if table_exists(connection, "mood_catalog")? {
        let mut statement = connection.prepare(
            "SELECT name, sentiment_score, created_at, updated_at
             FROM mood_catalog
             ORDER BY lower(name) ASC",
        )?;
        let moods = statement
            .query_map([], |row| {
                Ok(MoodPayload {
                    name: row.get(0)?,
                    sentiment_score: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        moods
    } else {
        Vec::new()
    };

    let deleted_moods = if table_exists(connection, "sync_mood_tombstones")? {
        let mut statement = connection.prepare(
            "SELECT name, deleted_at
             FROM sync_mood_tombstones
             ORDER BY deleted_at ASC, lower(name) ASC",
        )?;
        let deleted_moods = statement
            .query_map([], |row| {
                Ok(DeletedMood {
                    name: row.get(0)?,
                    deleted_at: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        deleted_moods
    } else {
        Vec::new()
    };

    Ok(MoodSyncPayload {
        moods,
        deleted_moods,
    })
}

fn apply_threads_payload(
    connection: &Connection,
    payload: &ThreadSyncPayload,
    counts: &mut SyncCounts,
) -> Result<()> {
    let mut deleted_continuations = payload.deleted_continuations.clone();
    deleted_continuations.sort_by(|left, right| {
        normalized_timestamp(Some(&left.deleted_at), "")
            .cmp(&normalized_timestamp(Some(&right.deleted_at), ""))
    });
    for item in deleted_continuations {
        let Some(child_uuid) = normalize_string(Some(item.child_entry_uuid)) else {
            continue;
        };
        let deleted_at = normalized_timestamp(Some(&item.deleted_at), &current_timestamp_minutes());
        let existing = continuation_row(connection, &child_uuid)?;
        if let Some(existing) = existing {
            let local_updated = normalized_timestamp(existing.updated_at.as_deref(), "");
            if deleted_at >= local_updated {
                connection.execute(
                    "DELETE FROM entry_continuations WHERE child_entry_uuid = ?1",
                    [&child_uuid],
                )?;
                record_continuation_tombstone(connection, &child_uuid, &deleted_at)?;
                counts.deleted += 1;
            }
        } else {
            record_continuation_tombstone(connection, &child_uuid, &deleted_at)?;
        }
    }

    let mut continuations = payload.continuations.clone();
    continuations.sort_by(|left, right| {
        normalized_timestamp(Some(&left.updated_at), "")
            .cmp(&normalized_timestamp(Some(&right.updated_at), ""))
            .then_with(|| left.child_entry_uuid.cmp(&right.child_entry_uuid))
    });
    for item in continuations {
        let Some(child_uuid) = normalize_string(Some(item.child_entry_uuid)) else {
            continue;
        };
        let Some(parent_uuid) = normalize_string(Some(item.parent_entry_uuid)) else {
            continue;
        };
        if child_uuid == parent_uuid
            || !entry_uuid_exists(connection, &child_uuid)?
            || !entry_uuid_exists(connection, &parent_uuid)?
            || continuation_would_cycle(connection, &child_uuid, &parent_uuid)?
        {
            continue;
        }
        let updated_at = normalized_timestamp(Some(&item.updated_at), &current_timestamp_minutes());
        if let Some(tombstone_at) = continuation_tombstone_at(connection, &child_uuid)? {
            if tombstone_at >= updated_at {
                continue;
            }
        }
        let before = continuation_row(connection, &child_uuid)?;
        if before
            .as_ref()
            .map(|row| normalized_timestamp(row.updated_at.as_deref(), "") > updated_at)
            .unwrap_or(false)
        {
            continue;
        }
        connection.execute(
            "INSERT INTO entry_continuations (child_entry_uuid, parent_entry_uuid, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(child_entry_uuid)
             DO UPDATE SET parent_entry_uuid = excluded.parent_entry_uuid,
                           updated_at = excluded.updated_at",
            params![child_uuid, parent_uuid, updated_at],
        )?;
        connection.execute(
            "DELETE FROM sync_entry_continuation_tombstones WHERE child_entry_uuid = ?1",
            [&child_uuid],
        )?;
        if let Some(before) = before {
            if before.parent_uuid != parent_uuid {
                counts.updated += 1;
            }
        } else {
            counts.imported += 1;
        }
    }

    apply_thread_text_deletes(
        connection,
        "entry_thread_titles",
        "title",
        "sync_entry_thread_title_tombstones",
        &payload.deleted_titles,
        counts,
    )?;
    apply_thread_text_deletes(
        connection,
        "entry_thread_summaries",
        "summary",
        "sync_entry_thread_summary_tombstones",
        &payload.deleted_summaries,
        counts,
    )?;
    apply_thread_titles(connection, &payload.titles, counts)?;
    apply_thread_summaries(connection, &payload.summaries, counts)?;
    Ok(())
}

#[derive(Debug)]
struct ExistingContinuation {
    parent_uuid: String,
    updated_at: Option<String>,
}

#[derive(Debug)]
struct ExistingThreadText {
    value: String,
    updated_at: String,
}

fn continuation_row(
    connection: &Connection,
    child_uuid: &str,
) -> Result<Option<ExistingContinuation>> {
    connection
        .query_row(
            "SELECT parent_entry_uuid, updated_at
             FROM entry_continuations
             WHERE child_entry_uuid = ?1",
            [child_uuid],
            |row| {
                Ok(ExistingContinuation {
                    parent_uuid: row.get(0)?,
                    updated_at: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn record_continuation_tombstone(
    connection: &Connection,
    child_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO sync_entry_continuation_tombstones (child_entry_uuid, deleted_at)
         VALUES (?1, ?2)
         ON CONFLICT(child_entry_uuid)
         DO UPDATE SET deleted_at = excluded.deleted_at",
        params![child_uuid, deleted_at],
    )?;
    Ok(())
}

fn continuation_tombstone_at(connection: &Connection, child_uuid: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT deleted_at FROM sync_entry_continuation_tombstones WHERE child_entry_uuid = ?1",
            [child_uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn apply_thread_text_deletes(
    connection: &Connection,
    table: &str,
    value_column: &str,
    tombstone_table: &str,
    items: &[DeletedThreadText],
    counts: &mut SyncCounts,
) -> Result<()> {
    for item in items {
        let Some(root_uuid) = normalize_string(Some(item.thread_root_uuid.clone())) else {
            continue;
        };
        if !entry_uuid_exists(connection, &root_uuid)? {
            continue;
        }
        let deleted_at = normalized_timestamp(Some(&item.deleted_at), &current_timestamp_minutes());
        let local_updated_at = thread_text_updated_at(connection, table, &root_uuid)?;
        if let Some(local_updated_at) = local_updated_at {
            if deleted_at >= normalized_timestamp(Some(&local_updated_at), "") {
                connection.execute(
                    &format!("DELETE FROM {table} WHERE thread_root_uuid = ?1"),
                    [&root_uuid],
                )?;
                record_thread_text_tombstone(connection, tombstone_table, &root_uuid, &deleted_at)?;
                counts.deleted += 1;
            }
        } else {
            let _ = value_column;
            record_thread_text_tombstone(connection, tombstone_table, &root_uuid, &deleted_at)?;
        }
    }
    Ok(())
}

fn apply_thread_titles(
    connection: &Connection,
    items: &[ThreadTitle],
    counts: &mut SyncCounts,
) -> Result<()> {
    for item in items {
        apply_thread_text_upsert(
            connection,
            "entry_thread_titles",
            "title",
            "sync_entry_thread_title_tombstones",
            &item.thread_root_uuid,
            &item.title,
            &item.updated_at,
            counts,
        )?;
    }
    Ok(())
}

fn apply_thread_summaries(
    connection: &Connection,
    items: &[ThreadSummary],
    counts: &mut SyncCounts,
) -> Result<()> {
    for item in items {
        apply_thread_text_upsert(
            connection,
            "entry_thread_summaries",
            "summary",
            "sync_entry_thread_summary_tombstones",
            &item.thread_root_uuid,
            &item.summary,
            &item.updated_at,
            counts,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_thread_text_upsert(
    connection: &Connection,
    table: &str,
    value_column: &str,
    tombstone_table: &str,
    root_uuid: &str,
    value: &str,
    updated_at: &str,
    counts: &mut SyncCounts,
) -> Result<()> {
    let Some(root_uuid) = normalize_string(Some(root_uuid.to_string())) else {
        return Ok(());
    };
    let Some(value) = normalize_string(Some(value.to_string())) else {
        return Ok(());
    };
    if !entry_can_have_thread_text(connection, &root_uuid)? {
        return Ok(());
    }
    let updated_at = normalized_timestamp(Some(updated_at), &current_timestamp_minutes());
    if let Some(tombstone_at) = thread_text_tombstone_at(connection, tombstone_table, &root_uuid)? {
        if tombstone_at >= updated_at {
            return Ok(());
        }
    }
    let existing = thread_text_row(connection, table, value_column, &root_uuid)?;
    if let Some(existing) = existing.as_ref() {
        let local_updated_at = normalized_timestamp(Some(&existing.updated_at), "");
        if local_updated_at > updated_at
            || (local_updated_at == updated_at && existing.value == value)
        {
            return Ok(());
        }
    }
    connection.execute(
        &format!(
            "INSERT INTO {table} (thread_root_uuid, {value_column}, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(thread_root_uuid)
             DO UPDATE SET {value_column} = excluded.{value_column},
                           updated_at = excluded.updated_at"
        ),
        params![root_uuid, value, updated_at],
    )?;
    connection.execute(
        &format!("DELETE FROM {tombstone_table} WHERE thread_root_uuid = ?1"),
        [&root_uuid],
    )?;
    if existing.is_some() {
        counts.updated += 1;
    } else {
        counts.imported += 1;
    }
    Ok(())
}

fn build_threads_payload(connection: &Connection) -> Result<ThreadSyncPayload> {
    let continuations = query_payload_rows(connection, "entry_continuations", |row| {
        Ok(ThreadContinuation {
            child_entry_uuid: row.get(0)?,
            parent_entry_uuid: row.get(1)?,
            updated_at: normalized_timestamp(row.get::<_, Option<String>>(2)?.as_deref(), ""),
        })
    })?;
    let deleted_continuations =
        query_payload_rows(connection, "sync_entry_continuation_tombstones", |row| {
            Ok(DeletedContinuation {
                child_entry_uuid: row.get(0)?,
                deleted_at: normalized_timestamp(row.get::<_, Option<String>>(1)?.as_deref(), ""),
            })
        })?;
    let titles = query_payload_rows(connection, "entry_thread_titles", |row| {
        Ok(ThreadTitle {
            thread_root_uuid: row.get(0)?,
            title: row.get(1)?,
            updated_at: normalized_timestamp(row.get::<_, Option<String>>(2)?.as_deref(), ""),
        })
    })?;
    let deleted_titles =
        query_payload_rows(connection, "sync_entry_thread_title_tombstones", |row| {
            Ok(DeletedThreadText {
                thread_root_uuid: row.get(0)?,
                deleted_at: normalized_timestamp(row.get::<_, Option<String>>(1)?.as_deref(), ""),
            })
        })?;
    let summaries = query_payload_rows(connection, "entry_thread_summaries", |row| {
        Ok(ThreadSummary {
            thread_root_uuid: row.get(0)?,
            summary: row.get(1)?,
            updated_at: normalized_timestamp(row.get::<_, Option<String>>(2)?.as_deref(), ""),
        })
    })?;
    let deleted_summaries =
        query_payload_rows(connection, "sync_entry_thread_summary_tombstones", |row| {
            Ok(DeletedThreadText {
                thread_root_uuid: row.get(0)?,
                deleted_at: normalized_timestamp(row.get::<_, Option<String>>(1)?.as_deref(), ""),
            })
        })?;

    Ok(ThreadSyncPayload {
        version: thread_sync_version(),
        continuations,
        deleted_continuations,
        titles,
        deleted_titles,
        summaries,
        deleted_summaries,
    })
}

fn query_payload_rows<T>(
    connection: &Connection,
    table: &str,
    map: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    if !table_exists(connection, table)? {
        return Ok(Vec::new());
    }
    let sql = match table {
        "entry_continuations" => {
            "SELECT child_entry_uuid, parent_entry_uuid, updated_at FROM entry_continuations ORDER BY updated_at ASC, child_entry_uuid ASC"
        }
        "sync_entry_continuation_tombstones" => {
            "SELECT child_entry_uuid, deleted_at FROM sync_entry_continuation_tombstones ORDER BY deleted_at ASC, child_entry_uuid ASC"
        }
        "entry_thread_titles" => {
            "SELECT thread_root_uuid, title, updated_at FROM entry_thread_titles ORDER BY updated_at ASC, thread_root_uuid ASC"
        }
        "sync_entry_thread_title_tombstones" => {
            "SELECT thread_root_uuid, deleted_at FROM sync_entry_thread_title_tombstones ORDER BY deleted_at ASC, thread_root_uuid ASC"
        }
        "entry_thread_summaries" => {
            "SELECT thread_root_uuid, summary, updated_at FROM entry_thread_summaries ORDER BY updated_at ASC, thread_root_uuid ASC"
        }
        "sync_entry_thread_summary_tombstones" => {
            "SELECT thread_root_uuid, deleted_at FROM sync_entry_thread_summary_tombstones ORDER BY deleted_at ASC, thread_root_uuid ASC"
        }
        _ => return Err(anyhow!("unsupported sync payload table: {table}")),
    };
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], map)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn apply_ai_chats_payload(
    connection: &Connection,
    payload: &AiChatSyncPayload,
    counts: &mut SyncCounts,
) -> Result<()> {
    if !table_exists(connection, "ai_conversations")?
        || !table_exists(connection, "ai_conversation_messages")?
    {
        return Ok(());
    }
    connection.execute(
        "CREATE TABLE IF NOT EXISTS sync_ai_conversation_tombstones (
            conversation_uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        )",
        [],
    )?;
    ensure_table_column(connection, "ai_conversations", "model", "TEXT")?;

    for item in payload.deleted_conversations.iter() {
        let Some(conversation_uuid) = normalize_string(Some(item.conversation_uuid.clone())) else {
            continue;
        };
        let deleted_at = normalized_timestamp(Some(&item.deleted_at), &current_timestamp_seconds());
        let local = connection
            .query_row(
                "SELECT id, updated_at FROM ai_conversations WHERE uuid = ?1",
                [&conversation_uuid],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((conversation_id, local_updated_at)) = local {
            if deleted_at >= normalized_timestamp(Some(&local_updated_at), "") {
                delete_ai_conversation(
                    connection,
                    conversation_id,
                    &conversation_uuid,
                    &deleted_at,
                )?;
                counts.deleted += 1;
            }
        } else {
            record_ai_conversation_tombstone(connection, &conversation_uuid, &deleted_at)?;
        }
    }

    for item in payload.conversations.iter() {
        let Some(conversation_uuid) = normalize_string(Some(item.uuid.clone())) else {
            continue;
        };
        let created_at = normalized_timestamp(Some(&item.created_at), &current_timestamp_seconds());
        let updated_at = normalized_timestamp(Some(&item.updated_at), &created_at);
        if let Some(tombstone_at) = ai_conversation_tombstone_at(connection, &conversation_uuid)? {
            if tombstone_at >= updated_at {
                continue;
            }
        }
        let existing = connection
            .query_row(
                "SELECT id, updated_at FROM ai_conversations WHERE uuid = ?1",
                [&conversation_uuid],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let title = normalize_optional_text(item.title.as_deref())
            .unwrap_or_else(|| "New chat".to_string());
        let preview = normalize_optional_text(item.preview.as_deref()).unwrap_or_default();
        let cloud_provider = normalize_string(Some(item.cloud_provider.clone()))
            .unwrap_or_else(|| "gemini".to_string())
            .to_lowercase();
        let model = normalize_optional_text(item.model.as_deref());
        let scope =
            normalize_string(Some(item.scope.clone())).unwrap_or_else(|| "search".to_string());
        let scope_identifiers = scope_identifiers_to_string(&item.scope_identifiers);
        if let Some((conversation_id, local_updated_at)) = existing {
            if updated_at > normalized_timestamp(Some(&local_updated_at), &created_at) {
                connection.execute(
                    "UPDATE ai_conversations
                     SET title = ?1,
                        preview = ?2,
                        cloud_provider = ?3,
                         model = ?4,
                         scope = ?5,
                         scope_identifiers = ?6,
                         context_limit = ?7,
                         since = ?8,
                         until = ?9,
                         created_at = ?10,
                         updated_at = ?11
                     WHERE id = ?12",
                    params![
                        title,
                        preview,
                        cloud_provider,
                        model,
                        scope,
                        scope_identifiers,
                        item.context_limit,
                        item.since,
                        item.until,
                        created_at,
                        updated_at,
                        conversation_id,
                    ],
                )?;
                counts.updated += 1;
            }
        } else {
            connection.execute(
                "INSERT INTO ai_conversations (
                    uuid, title, preview, cloud_provider, model, scope, scope_identifiers,
                    context_limit, since, until, created_at, updated_at, last_message_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    conversation_uuid,
                    title,
                    preview,
                    cloud_provider,
                    model,
                    scope,
                    scope_identifiers,
                    item.context_limit,
                    item.since,
                    item.until,
                    created_at,
                    updated_at,
                    updated_at,
                ],
            )?;
            counts.imported += 1;
        }
        connection.execute(
            "DELETE FROM sync_ai_conversation_tombstones WHERE conversation_uuid = ?1",
            [&conversation_uuid],
        )?;
    }

    let conversation_id_by_uuid = ai_conversation_id_by_uuid(connection)?;
    let mut affected_conversations = HashSet::new();
    let mut messages = payload.messages.clone();
    messages.sort_by(|left, right| {
        left.sort_key
            .cmp(&right.sort_key)
            .then_with(|| left.created_at.cmp(&right.created_at))
            .then_with(|| left.uuid.cmp(&right.uuid))
    });
    for item in messages {
        let Some(message_uuid) = normalize_string(Some(item.uuid)) else {
            continue;
        };
        let Some(conversation_uuid) = normalize_string(Some(item.conversation_uuid)) else {
            continue;
        };
        let Some(conversation_id) = conversation_id_by_uuid.get(&conversation_uuid).copied() else {
            continue;
        };
        let created_at = normalized_timestamp(Some(&item.created_at), &current_timestamp_seconds());
        let updated_at = normalized_timestamp(Some(&item.updated_at), &created_at);
        if let Some(tombstone_at) = ai_conversation_tombstone_at(connection, &conversation_uuid)? {
            if tombstone_at >= updated_at {
                continue;
            }
        }
        let role = normalize_ai_role(&item.role);
        let status = normalize_ai_status(&item.status);
        let sort_key = normalize_string(item.sort_key)
            .unwrap_or_else(|| format!("{created_at}:{message_uuid}"));
        let existing = connection
            .query_row(
                "SELECT id, updated_at FROM ai_conversation_messages WHERE uuid = ?1",
                [&message_uuid],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((message_id, local_updated_at)) = existing {
            if updated_at > normalized_timestamp(Some(&local_updated_at), &created_at) {
                connection.execute(
                    "UPDATE ai_conversation_messages
                     SET conversation_id = ?1,
                         role = ?2,
                         content = ?3,
                         status = ?4,
                         created_at = ?5,
                         updated_at = ?6,
                         sort_key = ?7
                     WHERE id = ?8",
                    params![
                        conversation_id,
                        role,
                        item.content,
                        status,
                        created_at,
                        updated_at,
                        sort_key,
                        message_id,
                    ],
                )?;
                affected_conversations.insert(conversation_id);
            }
        } else {
            connection.execute(
                "INSERT INTO ai_conversation_messages (
                    conversation_id, uuid, role, content, status, created_at, updated_at, sort_key
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    conversation_id,
                    message_uuid,
                    role,
                    item.content,
                    status,
                    created_at,
                    updated_at,
                    sort_key,
                ],
            )?;
            affected_conversations.insert(conversation_id);
        }
    }
    for conversation_id in affected_conversations {
        refresh_ai_conversation_summary(connection, conversation_id)?;
    }
    Ok(())
}

fn build_ai_chats_payload(connection: &Connection) -> Result<AiChatSyncPayload> {
    if !table_exists(connection, "ai_conversations")?
        || !table_exists(connection, "ai_conversation_messages")?
    {
        return Ok(AiChatSyncPayload {
            version: ai_chat_sync_version(),
            ..AiChatSyncPayload::default()
        });
    }
    let model_sql = if table_has_column(connection, "ai_conversations", "model")? {
        "model"
    } else {
        "NULL"
    };
    let mut conversation_statement = connection.prepare(&format!(
        "SELECT uuid, title, preview, cloud_provider, {model_sql} AS model, scope, scope_identifiers,
                context_limit, since, until, created_at, updated_at
         FROM ai_conversations
         WHERE COALESCE(uuid, '') != ''
         ORDER BY updated_at ASC, uuid ASC"
    ))?;
    let conversations = conversation_statement
        .query_map([], |row| {
            let scope_identifiers: String = row.get(6)?;
            Ok(AiConversationPayload {
                uuid: row.get(0)?,
                title: row.get(1)?,
                preview: row.get(2)?,
                cloud_provider: row.get(3)?,
                model: row.get(4)?,
                scope: row.get(5)?,
                scope_identifiers: serde_json::from_str(&scope_identifiers)
                    .unwrap_or_else(|_| json!([])),
                context_limit: row.get(7)?,
                since: row.get(8)?,
                until: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut message_statement = connection.prepare(
        "SELECT m.uuid, c.uuid, m.role, m.content, m.status, m.created_at, m.updated_at, m.sort_key
         FROM ai_conversation_messages m
         JOIN ai_conversations c ON c.id = m.conversation_id
         WHERE COALESCE(m.uuid, '') != '' AND COALESCE(c.uuid, '') != ''
         ORDER BY c.updated_at ASC, COALESCE(m.sort_key, m.created_at) ASC, m.uuid ASC",
    )?;
    let messages = message_statement
        .query_map([], |row| {
            Ok(AiMessagePayload {
                uuid: row.get(0)?,
                conversation_uuid: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                sort_key: row.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let deleted_conversations = if table_exists(connection, "sync_ai_conversation_tombstones")? {
        let mut statement = connection.prepare(
            "SELECT conversation_uuid, deleted_at
             FROM sync_ai_conversation_tombstones
             ORDER BY deleted_at ASC, conversation_uuid ASC",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(DeletedAiConversation {
                    conversation_uuid: row.get(0)?,
                    deleted_at: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };

    Ok(AiChatSyncPayload {
        version: ai_chat_sync_version(),
        conversations,
        messages,
        deleted_conversations,
    })
}

fn delete_ai_conversation(
    connection: &Connection,
    conversation_id: i64,
    conversation_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    connection.execute(
        "DELETE FROM ai_conversation_messages WHERE conversation_id = ?1",
        [conversation_id],
    )?;
    connection.execute(
        "DELETE FROM ai_conversations WHERE id = ?1",
        [conversation_id],
    )?;
    record_ai_conversation_tombstone(connection, conversation_uuid, deleted_at)?;
    Ok(())
}

fn record_ai_conversation_tombstone(
    connection: &Connection,
    conversation_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO sync_ai_conversation_tombstones (conversation_uuid, deleted_at)
         VALUES (?1, ?2)
         ON CONFLICT(conversation_uuid)
         DO UPDATE SET deleted_at = excluded.deleted_at",
        params![conversation_uuid, deleted_at],
    )?;
    Ok(())
}

fn ai_conversation_tombstone_at(
    connection: &Connection,
    conversation_uuid: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT deleted_at
             FROM sync_ai_conversation_tombstones
             WHERE conversation_uuid = ?1",
            [conversation_uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn ai_conversation_id_by_uuid(connection: &Connection) -> Result<HashMap<String, i64>> {
    let mut statement = connection.prepare("SELECT uuid, id FROM ai_conversations")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .collect())
}

fn refresh_ai_conversation_summary(connection: &Connection, conversation_id: i64) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT role, content, created_at, updated_at
         FROM ai_conversation_messages
         WHERE conversation_id = ?1
         ORDER BY COALESCE(sort_key, created_at) ASC, id ASC",
    )?;
    let rows = statement
        .query_map([conversation_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.is_empty() {
        return Ok(());
    }
    let title = rows
        .iter()
        .find(|(role, _, _, _)| role == "user")
        .or_else(|| rows.first())
        .map(|(_, content, _, _)| truncate_for_display(content, 80))
        .unwrap_or_else(|| "New chat".to_string());
    let preview = rows
        .last()
        .map(|(_, content, _, _)| truncate_for_display(content, 160))
        .unwrap_or_default();
    let last_message_at = rows
        .iter()
        .map(|(_, _, created_at, updated_at)| {
            normalized_timestamp(
                Some(updated_at),
                &normalized_timestamp(Some(created_at), ""),
            )
        })
        .max()
        .unwrap_or_else(current_timestamp_seconds);
    connection.execute(
        "UPDATE ai_conversations
         SET title = ?1,
             preview = ?2,
             last_message_at = ?3,
             updated_at = CASE WHEN updated_at > ?3 THEN updated_at ELSE ?3 END
         WHERE id = ?4",
        params![title, preview, last_message_at, conversation_id],
    )?;
    Ok(())
}

fn record_sync_success(
    db_path: &Path,
    sync_file: &Path,
    counts: &SyncCounts,
    summary: &str,
    timestamp: &str,
    conflict_summary: Option<String>,
) -> Result<()> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_sync_schema(&connection)?;
    let file_size = fs::metadata(sync_file)
        .ok()
        .map(|metadata| metadata.len() as i64);
    connection.execute(
        "UPDATE sync_status
         SET last_successful_sync_at = ?1,
             last_sync_file_path = ?2,
             last_sync_file_size_bytes = ?3,
             last_sync_imported = ?4,
             last_sync_updated = ?5,
             last_sync_deleted = ?6,
             last_sync_total = ?7,
             last_sync_summary = ?8,
             last_conflict_count = ?9,
             last_conflict_summary = ?10,
             last_sync_error = NULL
         WHERE id = 1",
        params![
            timestamp,
            db::path_to_string(sync_file),
            file_size,
            counts.imported,
            counts.updated,
            counts.deleted,
            counts.exported,
            summary,
            counts.conflicts.len() as i64,
            conflict_summary,
        ],
    )?;
    insert_sync_history(
        &connection,
        timestamp,
        "success",
        Some(sync_file),
        counts.imported,
        counts.updated,
        counts.deleted,
        counts.exported,
        counts.conflicts.len() as i64,
        Some(summary),
        (!counts.conflicts.is_empty()).then(|| counts.conflicts.join("\n")),
        None,
    )?;
    Ok(())
}

fn record_sync_summary_override(db_path: &Path, timestamp: &str, summary: &str) -> Result<()> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_sync_schema(&connection)?;
    connection.execute(
        "UPDATE sync_status SET last_sync_summary = ?1 WHERE id = 1",
        [summary],
    )?;
    connection.execute(
        "UPDATE sync_history
         SET summary = ?1
         WHERE id = (
             SELECT id FROM sync_history
             WHERE timestamp = ?2 AND status = 'success'
             ORDER BY id DESC
             LIMIT 1
         )",
        params![summary, timestamp],
    )?;
    Ok(())
}

fn record_sync_error(db_path: &Path, sync_file: &Path, error: &str) -> Result<()> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_sync_schema(&connection)?;
    connection.execute(
        "UPDATE sync_status SET last_sync_error = ?1 WHERE id = 1",
        [error],
    )?;
    insert_sync_history(
        &connection,
        &current_timestamp_seconds(),
        "failed",
        Some(sync_file),
        0,
        0,
        0,
        0,
        0,
        None,
        None,
        Some(error),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_sync_history(
    connection: &Connection,
    timestamp: &str,
    status: &str,
    sync_file: Option<&Path>,
    imported: i64,
    updated: i64,
    deleted: i64,
    exported: i64,
    conflicts: i64,
    summary: Option<&str>,
    details: Option<String>,
    error: Option<&str>,
) -> Result<()> {
    connection.execute(
        "INSERT INTO sync_history (
            timestamp, status, sync_file_path, imported_count, updated_count,
            deleted_count, exported_count, conflict_count, summary, details, error
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            timestamp,
            status,
            sync_file.map(db::path_to_string),
            imported,
            updated,
            deleted,
            exported,
            conflicts,
            summary,
            details,
            error,
        ],
    )?;
    connection.execute(
        "DELETE FROM sync_history
         WHERE id NOT IN (
            SELECT id FROM sync_history ORDER BY id DESC LIMIT ?1
         )",
        [SYNC_HISTORY_RETENTION_LIMIT],
    )?;
    Ok(())
}

fn sync_summary(counts: &SyncCounts) -> String {
    let mut parts = Vec::new();
    if counts.imported > 0 {
        parts.push(format!("{} merged", counts.imported));
    }
    if counts.updated > 0 {
        parts.push(format!("{} updated", counts.updated));
    }
    if counts.deleted > 0 {
        parts.push(format!("{} deleted", counts.deleted));
    }
    if !counts.parts.is_empty() {
        parts.extend(counts.parts.clone());
    }
    if parts.is_empty() {
        "nothing new".to_string()
    } else {
        parts.join(", ")
    }
}

fn normalize_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn resolve_sync_optional_text(incoming: &SyncValue<String>, local: Option<&str>) -> Option<String> {
    match incoming {
        SyncValue::Missing => normalize_optional_text(local),
        SyncValue::Null => None,
        SyncValue::Value(value) => normalize_optional_text(Some(value)),
    }
}

fn resolve_sync_content_format(incoming: &SyncValue<String>, local: &str) -> String {
    match incoming {
        SyncValue::Value(value) => normalize_content_format(Some(value)),
        SyncValue::Null => normalize_content_format(None),
        SyncValue::Missing => normalize_content_format(Some(local)),
    }
}

fn resolve_sync_tags(incoming: &SyncValue<Vec<String>>, local: &[String]) -> Vec<String> {
    match incoming {
        SyncValue::Value(values) => normalize_tags(values.clone()),
        SyncValue::Null => Vec::new(),
        SyncValue::Missing => normalize_tags(local.to_vec()),
    }
}

fn resolve_sync_bool(incoming: &SyncValue<bool>, local: bool) -> bool {
    match incoming {
        SyncValue::Value(value) => *value,
        SyncValue::Null => false,
        SyncValue::Missing => local,
    }
}

fn normalize_tags(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = values
        .into_iter()
        .filter_map(|tag| normalize_string(Some(tag.to_lowercase())))
        .filter(|tag| seen.insert(tag.clone()))
        .collect::<Vec<_>>();
    tags.sort();
    tags
}

fn normalize_slug(value: &str) -> Option<String> {
    let mut output = String::new();
    let mut last_was_dash = false;
    for character in value.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    let output = output.trim_matches('-').to_string();
    (!output.is_empty()).then_some(output)
}

fn normalized_text_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| normalize_string(Some(value.clone())))
        .collect()
}

fn json_string_list(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| normalize_string(Some(value)))
        .collect()
}

fn entry_exists(connection: &Connection, uuid: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM entries WHERE uuid = ?1 LIMIT 1",
            [uuid],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn location_tombstone_at(connection: &Connection, entry_uuid: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT deleted_at FROM sync_location_tombstones WHERE entry_uuid = ?1",
            [entry_uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn mood_catalog_row(connection: &Connection, name: &str) -> Result<Option<(String, String)>> {
    connection
        .query_row(
            "SELECT created_at, updated_at FROM mood_catalog WHERE lower(name) = lower(?1)",
            [name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn mood_tombstone_at(connection: &Connection, name: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT deleted_at FROM sync_mood_tombstones WHERE lower(name) = lower(?1)",
            [name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn library_template_row(
    connection: &Connection,
    slug: &str,
) -> Result<Option<(bool, String, String)>> {
    connection
        .query_row(
            "SELECT is_builtin, created_at, updated_at FROM library_templates WHERE slug = ?1",
            [slug],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(Into::into)
}

fn library_prompt_row(
    connection: &Connection,
    slug: &str,
) -> Result<Option<(bool, String, String)>> {
    connection
        .query_row(
            "SELECT is_builtin, created_at, updated_at FROM library_prompts WHERE slug = ?1",
            [slug],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(Into::into)
}

fn query_deleted_library_items(
    connection: &Connection,
    table: &str,
    sql: &str,
) -> Result<Vec<DeletedLibraryItem>> {
    if !table_exists(connection, table)? {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map([], |row| {
            Ok(DeletedLibraryItem {
                slug: row.get(0)?,
                deleted_at: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(anyhow::Error::from)?;
    Ok(rows)
}

fn normalize_content_format(value: Option<&str>) -> String {
    match value.unwrap_or("plain").trim().to_lowercase().as_str() {
        "markdown" => "markdown".to_string(),
        "html" => "html".to_string(),
        _ => "plain".to_string(),
    }
}

fn normalized_timestamp(value: Option<&str>, fallback: &str) -> String {
    let raw = value.unwrap_or("").trim();
    let raw = if raw.is_empty() { fallback.trim() } else { raw };
    let mut normalized = raw.trim_end_matches('Z').replace('T', " ");
    if normalized.len() > 19 {
        if let Some(offset_index) = normalized[19..].find(['+', '-']) {
            normalized.truncate(19 + offset_index);
        }
    }
    normalized
}

fn build_text_plain(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn current_timestamp_minutes() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

fn current_timestamp_seconds() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table_name],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn ensure_table_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> Result<()> {
    if table_has_column(connection, table_name, column_name)? {
        return Ok(());
    }
    connection.execute(
        &format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {definition}"),
        [],
    )?;
    Ok(())
}

fn table_has_column(connection: &Connection, table_name: &str, column_name: &str) -> Result<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    let columns = rows.collect::<rusqlite::Result<HashSet<_>>>()?;
    Ok(columns.contains(column_name))
}

fn delete_if_table_exists(
    connection: &Connection,
    table: &str,
    column: &str,
    entry_id: i64,
) -> Result<()> {
    if table_exists(connection, table)? {
        connection.execute(
            &format!("DELETE FROM {table} WHERE {column} = ?1"),
            [entry_id],
        )?;
    }
    Ok(())
}

fn delete_by_uuid_if_table_exists(
    connection: &Connection,
    table: &str,
    column: &str,
    uuid: &str,
) -> Result<()> {
    if table_exists(connection, table)? {
        connection.execute(&format!("DELETE FROM {table} WHERE {column} = ?1"), [uuid])?;
    }
    Ok(())
}

fn entry_uuid_exists(connection: &Connection, uuid: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM entries WHERE uuid = ?1 LIMIT 1",
            [uuid],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn continuation_would_cycle(
    connection: &Connection,
    child_uuid: &str,
    parent_uuid: &str,
) -> Result<bool> {
    let mut current = Some(parent_uuid.to_string());
    let mut seen = HashSet::new();
    while let Some(uuid) = current {
        if uuid == child_uuid {
            return Ok(true);
        }
        if !seen.insert(uuid.clone()) {
            return Ok(false);
        }
        current = connection
            .query_row(
                "SELECT parent_entry_uuid FROM entry_continuations WHERE child_entry_uuid = ?1",
                [uuid],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
    }
    Ok(false)
}

fn thread_text_updated_at(
    connection: &Connection,
    table: &str,
    root_uuid: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            &format!("SELECT updated_at FROM {table} WHERE thread_root_uuid = ?1"),
            [root_uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn thread_text_row(
    connection: &Connection,
    table: &str,
    value_column: &str,
    root_uuid: &str,
) -> Result<Option<ExistingThreadText>> {
    connection
        .query_row(
            &format!("SELECT {value_column}, updated_at FROM {table} WHERE thread_root_uuid = ?1"),
            [root_uuid],
            |row| {
                Ok(ExistingThreadText {
                    value: row.get(0)?,
                    updated_at: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn thread_text_tombstone_at(
    connection: &Connection,
    table: &str,
    root_uuid: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            &format!("SELECT deleted_at FROM {table} WHERE thread_root_uuid = ?1"),
            [root_uuid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn record_thread_text_tombstone(
    connection: &Connection,
    table: &str,
    root_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    connection.execute(
        &format!(
            "INSERT INTO {table} (thread_root_uuid, deleted_at)
             VALUES (?1, ?2)
             ON CONFLICT(thread_root_uuid)
             DO UPDATE SET deleted_at = excluded.deleted_at"
        ),
        params![root_uuid, deleted_at],
    )?;
    Ok(())
}

fn entry_can_have_thread_text(connection: &Connection, root_uuid: &str) -> Result<bool> {
    if !entry_uuid_exists(connection, root_uuid)? {
        return Ok(false);
    }
    let has_parent = connection
        .query_row(
            "SELECT 1 FROM entry_continuations WHERE child_entry_uuid = ?1 LIMIT 1",
            [root_uuid],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if has_parent {
        return Ok(false);
    }
    let child_count = connection.query_row(
        "SELECT COUNT(*) FROM entry_continuations WHERE parent_entry_uuid = ?1",
        [root_uuid],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(child_count > 0)
}

fn scope_identifiers_to_string(value: &JsonValue) -> String {
    match value {
        JsonValue::String(text) => text.clone(),
        JsonValue::Array(_) => serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string()),
        _ => "[]".to_string(),
    }
}

fn normalize_ai_role(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "assistant" => "assistant".to_string(),
        "system" => "system".to_string(),
        _ => "user".to_string(),
    }
}

fn normalize_ai_status(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "streaming" => "interrupted".to_string(),
        "error" => "error".to_string(),
        "interrupted" => "interrupted".to_string(),
        _ => "complete".to_string(),
    }
}

fn truncate_for_display(value: &str, limit: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= limit {
        return normalized;
    }
    let mut output = normalized
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    output.push_str("...");
    output
}

fn thread_sync_version() -> i64 {
    1
}

fn ai_chat_sync_version() -> i64 {
    1
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_sync_test_database(path: &Path) -> Result<()> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "
            CREATE TABLE entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT UNIQUE,
                created_at TEXT NOT NULL,
                updated_at TEXT,
                text TEXT NOT NULL,
                text_plain TEXT,
                content_format TEXT,
                title TEXT,
                summary TEXT,
                mood TEXT,
                starred INTEGER NOT NULL DEFAULT 0,
                pinned INTEGER NOT NULL DEFAULT 0,
                hidden INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE entry_tags (
                entry_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                UNIQUE(entry_id, tag_id)
            );
            CREATE TABLE entries_fts (text);
            CREATE TABLE sync_tombstones (
                uuid TEXT PRIMARY KEY,
                deleted_at TEXT NOT NULL
            );
            INSERT INTO entries
                (uuid, created_at, updated_at, text, text_plain, content_format, starred, pinned, hidden)
            VALUES
                ('entry_remote_deleted', '2026-01-01 09:00', '2026-01-01 09:00',
                 'Local row', 'Local row', 'plain', 0, 0, 0);
            INSERT INTO sync_tombstones (uuid, deleted_at)
            VALUES ('entry_locally_deleted', '2026-01-02 09:00');
            ",
        )?;
        Ok(())
    }

    fn backup_file_count(path: &Path) -> usize {
        fs::read_dir(path)
            .expect("read backup dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("capsule_backup_")
                    && entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("db"))
            })
            .count()
    }

    #[test]
    fn sync_tombstones_delete_rows_and_prevent_remote_resurrection() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");

        let remote_payload = json!({
            "version": 4,
            "entries": [
                {
                    "uuid": "entry_locally_deleted",
                    "created_at": "2026-01-02 08:00",
                    "updated_at": "2026-01-02 08:00",
                    "text": "Remote should not come back",
                    "content_format": "plain",
                    "tags": []
                }
            ],
            "deleted_uuids": ["entry_remote_deleted"]
        });
        fs::write(
            sync_dir.path().join(MAIN_SYNC_FILE),
            serde_json::to_vec_pretty(&remote_payload).expect("json"),
        )
        .expect("sync file");

        let response =
            run_sync_with_retries(&db_path, sync_dir.path()).expect("sync should complete");
        assert_eq!(response.deleted_count, 1);

        let connection = Connection::open(&db_path).expect("open db");
        let remote_deleted_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM entries WHERE uuid = 'entry_remote_deleted'",
                [],
                |row| row.get(0),
            )
            .expect("remote deleted count");
        assert_eq!(remote_deleted_count, 0);

        let locally_deleted_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM entries WHERE uuid = 'entry_locally_deleted'",
                [],
                |row| row.get(0),
            )
            .expect("locally deleted count");
        assert_eq!(locally_deleted_count, 0);

        let output = fs::read(sync_dir.path().join(MAIN_SYNC_FILE)).expect("output sync file");
        let output_payload: MainSyncPayload =
            serde_json::from_slice(&output).expect("output payload");
        assert!(output_payload
            .deleted_uuids
            .iter()
            .any(|uuid| uuid == "entry_remote_deleted"));
        assert!(output_payload
            .deleted_uuids
            .iter()
            .any(|uuid| uuid == "entry_locally_deleted"));
    }

    #[test]
    fn sync_legacy_entry_omissions_preserve_local_metadata() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");

        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute(
                "UPDATE entries
                 SET title = ?1, summary = ?2, mood = ?3, starred = 1, pinned = 1, hidden = 1
                 WHERE uuid = 'entry_remote_deleted'",
                params!["Local title", "Local summary", "calm"],
            )
            .expect("metadata");
        connection
            .execute("INSERT INTO tags (name) VALUES ('preserve-me')", [])
            .expect("tag");
        connection
            .execute(
                "INSERT INTO entry_tags (entry_id, tag_id)
                 SELECT e.id, t.id FROM entries e, tags t
                 WHERE e.uuid = 'entry_remote_deleted' AND t.name = 'preserve-me'",
                [],
            )
            .expect("entry tag");
        drop(connection);

        let legacy_payload = json!({
            "version": 4,
            "entries": [
                {
                    "uuid": "entry_remote_deleted",
                    "created_at": "2026-01-01 09:00",
                    "updated_at": "2026-01-01 09:00",
                    "text": "Local row"
                }
            ],
            "deleted_uuids": []
        });
        fs::write(
            sync_dir.path().join(MAIN_SYNC_FILE),
            serde_json::to_vec_pretty(&legacy_payload).expect("json"),
        )
        .expect("sync file");

        let response =
            run_sync_with_retries(&db_path, sync_dir.path()).expect("sync should complete");
        assert_eq!(response.updated_count, 0);

        let connection = Connection::open(&db_path).expect("open db");
        let row = connection
            .query_row(
                "SELECT title, summary, mood, starred, pinned, hidden
                 FROM entries WHERE uuid = 'entry_remote_deleted'",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .expect("entry metadata");
        assert_eq!(row.0.as_deref(), Some("Local title"));
        assert_eq!(row.1.as_deref(), Some("Local summary"));
        assert_eq!(row.2.as_deref(), Some("calm"));
        assert_eq!((row.3, row.4, row.5), (1, 1, 1));
        assert_eq!(
            load_entry_tags(&connection, 1).expect("tags"),
            vec!["preserve-me".to_string()]
        );

        let output = fs::read(sync_dir.path().join(MAIN_SYNC_FILE)).expect("output sync file");
        let output_payload: MainSyncPayload =
            serde_json::from_slice(&output).expect("output payload");
        assert_eq!(output_payload.version, Some(MAIN_SYNC_VERSION));
    }

    #[test]
    fn sync_explicit_newer_null_clears_entry_title_and_summary() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");

        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute(
                "UPDATE entries
                 SET updated_at = '2026-01-01 09:00:00.100000',
                     title = 'Local title', summary = 'Local summary', mood = 'calm'
                 WHERE uuid = 'entry_remote_deleted'",
                [],
            )
            .expect("metadata");
        drop(connection);

        let payload = json!({
            "version": MAIN_SYNC_VERSION,
            "entries": [
                {
                    "uuid": "entry_remote_deleted",
                    "created_at": "2026-01-01 09:00",
                    "updated_at": "2026-01-01 09:00:00.200000",
                    "title": null,
                    "summary": null,
                    "text": "Local row"
                }
            ],
            "deleted_uuids": []
        });
        fs::write(
            sync_dir.path().join(MAIN_SYNC_FILE),
            serde_json::to_vec_pretty(&payload).expect("json"),
        )
        .expect("sync file");

        let response =
            run_sync_with_retries(&db_path, sync_dir.path()).expect("sync should complete");
        assert_eq!(response.updated_count, 1);

        let connection = Connection::open(&db_path).expect("open db");
        let row = connection
            .query_row(
                "SELECT title, summary, mood, updated_at
                 FROM entries WHERE uuid = 'entry_remote_deleted'",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("entry metadata");
        assert_eq!(row.0, None);
        assert_eq!(row.1, None);
        assert_eq!(row.2.as_deref(), Some("calm"));
        assert_eq!(row.3, "2026-01-01 09:00:00.200000");
    }

    #[test]
    fn timestamp_normalization_preserves_subsecond_ordering() {
        let older = normalized_timestamp(Some("2026-01-01T09:00:00.100000Z"), "");
        let newer = normalized_timestamp(Some("2026-01-01T09:00:00.200000Z"), "");
        assert_eq!(older, "2026-01-01 09:00:00.100000");
        assert!(newer > older);
    }

    #[test]
    fn sync_thread_text_skips_identical_rows() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");

        let connection = Connection::open(&db_path).expect("open db");
        ensure_sync_schema(&connection).expect("sync schema");
        connection
            .execute(
                "INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format, starred, pinned, hidden)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0)",
                params![
                    "entry_thread_root",
                    "2026-01-03 09:00",
                    "2026-01-03 09:00",
                    "Thread root",
                    "Thread root",
                    "plain"
                ],
            )
            .expect("root entry");
        connection
            .execute(
                "INSERT INTO entry_thread_titles (thread_root_uuid, title, updated_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    "entry_thread_root",
                    "Stable thread title",
                    "2026-01-03 10:00"
                ],
            )
            .expect("thread title");
        connection
            .execute(
                "INSERT INTO entry_thread_summaries (thread_root_uuid, summary, updated_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    "entry_thread_root",
                    "Stable thread summary",
                    "2026-01-03 10:00"
                ],
            )
            .expect("thread summary");
        drop(connection);

        let remote_threads = json!({
            "version": thread_sync_version(),
            "titles": [
                {
                    "thread_root_uuid": "entry_thread_root",
                    "title": "Stable thread title",
                    "updated_at": "2026-01-03 10:00"
                }
            ],
            "summaries": [
                {
                    "thread_root_uuid": "entry_thread_root",
                    "summary": "Stable thread summary",
                    "updated_at": "2026-01-03 10:00"
                }
            ]
        });
        fs::write(
            sync_dir.path().join(THREADS_SYNC_FILE),
            serde_json::to_vec_pretty(&remote_threads).expect("json"),
        )
        .expect("threads sync file");

        let response =
            run_sync_with_retries(&db_path, sync_dir.path()).expect("sync should complete");
        assert_eq!(response.updated_count, 0);
        assert_eq!(response.summary, "nothing new");
    }

    #[test]
    fn mobile_notes_stage_entries_tags_moods_and_locations() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");
        let mobile_notes: Vec<MobileNote> = serde_json::from_value(json!([
            {
                "client_id": "3353c210-611a-4eec-ae28-7e35f107bb31",
                "text": "A mobile test",
                "mood": "great",
                "tags": ["Coding", "coding"],
                "when": "2026-07-21 13:24",
                "location": {
                    "latitude": 69.69432754942983,
                    "longitude": 19.003339221916928
                }
            },
            {
                "client_id": "51e83087-6054-456f-94f9-e06a76d2fbdc",
                "text": "Another mobile test",
                "mood": "good",
                "tags": ["vibe-coding"],
                "when": "2026-07-21 14:18"
            }
        ]))
        .expect("mobile notes");

        let acknowledged =
            stage_mobile_notes(sync_dir.path(), &mobile_notes).expect("stage mobile notes");
        assert_eq!(acknowledged.len(), 2);
        stage_mobile_notes(sync_dir.path(), &mobile_notes).expect("stage idempotently");

        let staged =
            read_main_payload(&sync_dir.path().join(MAIN_SYNC_FILE)).expect("staged main payload");
        assert_eq!(
            staged
                .entries
                .iter()
                .filter(|entry| {
                    entry
                        .uuid
                        .as_deref()
                        .is_some_and(|uuid| uuid.starts_with("mobile_"))
                })
                .count(),
            2
        );

        location::set_test_auto_capture_fixture(Some(location::TestAutoCaptureFixture {
            latitude: 0.0,
            longitude: 0.0,
            place_name: Some("Tromso, Norway".to_string()),
            source: "mobile".to_string(),
            weather_temp_c: Some(12.8),
            weather_condition: Some("Partly cloudy".to_string()),
        }));
        let response = run_sync_with_backup_if_needed(&db_path, sync_dir.path())
            .expect("sync should complete");
        location::set_test_auto_capture_fixture(None);
        assert_eq!(response.imported_count, 2);
        assert!(response.summary.contains("1 mobile location enriched"));

        let connection = Connection::open(&db_path).expect("open db");
        let (entry_id, created_at, text, mood) = connection
            .query_row(
                "SELECT id, created_at, text, mood FROM entries WHERE uuid = ?1",
                ["mobile_3353c210-611a-4eec-ae28-7e35f107bb31"],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .expect("mobile entry");
        assert_eq!(created_at, "2026-07-21 13:24");
        assert_eq!(text, "A mobile test");
        assert_eq!(mood.as_deref(), Some("great"));
        assert_eq!(
            load_entry_tags(&connection, entry_id).expect("entry tags"),
            vec!["coding".to_string()]
        );
        let (latitude, longitude, place_name, source, temp_c, temp_f, condition, fetched_at) =
            connection
                .query_row(
                    "SELECT latitude, longitude, place_name, source, weather_temp_c,
                        weather_temp_f, weather_condition, weather_fetched_at
                 FROM plugin_entry_locations WHERE entry_uuid = ?1",
                    ["mobile_3353c210-611a-4eec-ae28-7e35f107bb31"],
                    |row| {
                        Ok((
                            row.get::<_, f64>(0)?,
                            row.get::<_, f64>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<f64>>(4)?,
                            row.get::<_, Option<f64>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                        ))
                    },
                )
                .expect("mobile location");
        assert_eq!(latitude, 69.69432754942983);
        assert_eq!(longitude, 19.003339221916928);
        assert_eq!(place_name.as_deref(), Some("Tromso, Norway"));
        assert_eq!(source, "mobile");
        assert_eq!(temp_c, Some(12.8));
        assert_eq!(temp_f, Some(55.0));
        assert_eq!(condition.as_deref(), Some("Partly cloudy"));
        assert!(fetched_at.is_some());

        let synced = read_main_payload(&sync_dir.path().join(MAIN_SYNC_FILE))
            .expect("enriched sync payload");
        let locations = parse_extra_payload::<LocationSyncPayload>(&synced.extra, "locations")
            .expect("parse locations payload")
            .expect("locations payload");
        let synced_location = locations
            .locations
            .iter()
            .find(|item| item.entry_uuid == "mobile_3353c210-611a-4eec-ae28-7e35f107bb31")
            .expect("synced mobile location");
        assert_eq!(
            synced_location.place_name.as_deref(),
            Some("Tromso, Norway")
        );
        assert_eq!(synced_location.weather_temp_c, Some(12.8));
    }

    #[test]
    fn mobile_note_acknowledgement_preserves_new_and_unknown_queue_items() {
        let acknowledged = HashSet::from([
            "3353c210-611a-4eec-ae28-7e35f107bb31".to_string(),
            "51e83087-6054-456f-94f9-e06a76d2fbdc".to_string(),
        ]);
        let content = serde_json::to_string_pretty(&json!([
            {
                "client_id": "3353c210-611a-4eec-ae28-7e35f107bb31",
                "text": "Imported"
            },
            {
                "client_id": "new-during-sync",
                "text": "Keep this"
            },
            {
                "text": "Keep malformed items for diagnosis"
            }
        ]))
        .expect("queue json");

        let filtered = filter_acknowledged_mobile_notes(&content, &acknowledged)
            .expect("filter queue")
            .expect("queue changed");
        let remaining: Vec<JsonValue> = serde_json::from_str(&filtered).expect("remaining queue");
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0]["client_id"], "new-during-sync");
        assert!(remaining[1].get("client_id").is_none());
    }

    #[test]
    fn sync_merges_mood_sentiments_and_tombstones() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");

        let connection = Connection::open(&db_path).expect("database");
        ensure_mood_sync_schema(&connection).expect("mood schema");
        connection
            .execute(
                "INSERT INTO mood_catalog (name, sentiment_score, created_at, updated_at)
                 VALUES ('calm', 0.1, '2026-07-20 09:00:00', '2026-07-20 09:00:00')",
                [],
            )
            .expect("local mood");
        connection
            .execute(
                "UPDATE entries
                 SET mood = 'focused', updated_at = '2026-07-20 09:00:00'
                 WHERE uuid = 'entry_remote_deleted'",
                [],
            )
            .expect("local entry mood");
        drop(connection);

        let remote_payload = json!({
            "version": MAIN_SYNC_VERSION,
            "entries": [],
            "deleted_uuids": [],
            "moods": {
                "moods": [
                    {
                        "name": "calm",
                        "sentiment_score": 0.7,
                        "created_at": "2026-07-20 09:00:00",
                        "updated_at": "2026-07-22 09:00:00"
                    },
                    {
                        "name": "buoyant",
                        "sentiment_score": 0.45,
                        "created_at": "2026-07-22 09:00:00",
                        "updated_at": "2026-07-22 09:00:00"
                    }
                ],
                "deleted_moods": [
                    {
                        "name": "focused",
                        "deleted_at": "2026-07-22 09:00:00"
                    }
                ]
            }
        });
        fs::write(
            sync_dir.path().join(MAIN_SYNC_FILE),
            serde_json::to_vec_pretty(&remote_payload).expect("json"),
        )
        .expect("sync file");

        let response =
            run_sync_with_retries(&db_path, sync_dir.path()).expect("sync should complete");
        assert!(response.summary.contains("moods: 1 added, 1 updated"));
        assert!(response.summary.contains("1 entry values cleared"));

        let connection = Connection::open(&db_path).expect("database");
        let calm_score: f64 = connection
            .query_row(
                "SELECT sentiment_score FROM mood_catalog WHERE name = 'calm'",
                [],
                |row| row.get(0),
            )
            .expect("calm score");
        assert_eq!(calm_score, 0.7);
        let buoyant_score: f64 = connection
            .query_row(
                "SELECT sentiment_score FROM mood_catalog WHERE name = 'buoyant'",
                [],
                |row| row.get(0),
            )
            .expect("buoyant score");
        assert_eq!(buoyant_score, 0.45);
        let focused_entry_mood: Option<String> = connection
            .query_row(
                "SELECT mood FROM entries WHERE uuid = 'entry_remote_deleted'",
                [],
                |row| row.get(0),
            )
            .expect("focused entry");
        assert!(focused_entry_mood.is_none());
        drop(connection);

        let synced =
            read_main_payload(&sync_dir.path().join(MAIN_SYNC_FILE)).expect("synced payload");
        let moods = parse_extra_payload::<MoodSyncPayload>(&synced.extra, "moods")
            .expect("mood payload")
            .expect("moods");
        assert!(moods.moods.iter().any(|mood| mood.name == "buoyant"));
        assert!(moods
            .deleted_moods
            .iter()
            .any(|mood| mood.name == "focused"));
    }

    #[test]
    fn sync_unchanged_payload_does_not_create_backup() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");

        let (main_payload, thread_payload, ai_payload) =
            build_local_sync_payloads(&db_path, Map::new(), Vec::new())
                .expect("local sync payloads");
        write_json_replace(&sync_dir.path().join(MAIN_SYNC_FILE), &main_payload)
            .expect("main sync file");
        write_json_replace(&sync_dir.path().join(THREADS_SYNC_FILE), &thread_payload)
            .expect("threads sync file");
        write_json_replace(&sync_dir.path().join(AI_CHATS_SYNC_FILE), &ai_payload)
            .expect("ai sync file");

        let before = backup_file_count(db_dir.path());
        let response = run_sync_with_backup_if_needed(&db_path, sync_dir.path())
            .expect("sync should complete");

        assert_eq!(response.summary, "nothing new");
        assert_eq!(response.imported_count, 0);
        assert_eq!(response.updated_count, 0);
        assert_eq!(response.deleted_count, 0);
        assert_eq!(backup_file_count(db_dir.path()), before);
    }

    #[test]
    fn sync_unchanged_payload_backfills_pending_mobile_location() {
        let db_dir = tempdir().expect("db tempdir");
        let sync_dir = tempdir().expect("sync tempdir");
        let db_path = db_dir.path().join("capsule.db");
        create_sync_test_database(&db_path).expect("database");

        let connection = Connection::open(&db_path).expect("open db");
        ensure_location_sync_schema(&connection).expect("location sync schema");
        connection
            .execute(
                "INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format,
                     starred, pinned, hidden)
                 VALUES (?1, ?2, ?2, ?3, ?3, 'plain', 0, 0, 0)",
                params![
                    "mobile_existing-note",
                    "2026-07-21 13:24",
                    "Previously imported mobile note"
                ],
            )
            .expect("mobile entry");
        connection
            .execute(
                "INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, source, created_at)
                 VALUES (?1, ?2, ?3, 'mobile', ?4)",
                params![
                    "mobile_existing-note",
                    69.69432754942983,
                    19.003339221916928,
                    "2026-07-21 13:24"
                ],
            )
            .expect("raw mobile location");
        drop(connection);

        let (main_payload, thread_payload, ai_payload) =
            build_local_sync_payloads(&db_path, Map::new(), Vec::new())
                .expect("local sync payloads");
        write_json_replace(&sync_dir.path().join(MAIN_SYNC_FILE), &main_payload)
            .expect("main sync file");
        write_json_replace(&sync_dir.path().join(THREADS_SYNC_FILE), &thread_payload)
            .expect("threads sync file");
        write_json_replace(&sync_dir.path().join(AI_CHATS_SYNC_FILE), &ai_payload)
            .expect("ai sync file");

        location::set_test_auto_capture_fixture(Some(location::TestAutoCaptureFixture {
            latitude: 0.0,
            longitude: 0.0,
            place_name: Some("Utsikten, Tromso, Norway".to_string()),
            source: "mobile".to_string(),
            weather_temp_c: Some(12.8),
            weather_condition: Some("Partly cloudy".to_string()),
        }));
        let before = backup_file_count(db_dir.path());
        let response = run_sync_with_backup_if_needed(&db_path, sync_dir.path())
            .expect("sync should backfill location");
        location::set_test_auto_capture_fixture(None);

        assert!(response.summary.contains("1 mobile location enriched"));
        assert_eq!(backup_file_count(db_dir.path()), before + 1);
        let connection = Connection::open(&db_path).expect("open db");
        let (place_name, source, weather_temp_c): (Option<String>, String, Option<f64>) =
            connection
                .query_row(
                    "SELECT place_name, source, weather_temp_c
                     FROM plugin_entry_locations WHERE entry_uuid = ?1",
                    ["mobile_existing-note"],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("enriched mobile location");
        assert_eq!(place_name.as_deref(), Some("Utsikten, Tromso, Norway"));
        assert_eq!(source, "mobile");
        assert_eq!(weather_temp_c, Some(12.8));
    }
}
