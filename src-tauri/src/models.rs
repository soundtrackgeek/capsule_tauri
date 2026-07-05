use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub db_path: String,
    pub db_exists: bool,
    pub db_size_bytes: u64,
    pub db_modified_at: Option<String>,
    pub readable: bool,
    pub schema_summary: SchemaSummary,
    pub entry_count: Option<i64>,
    pub tag_count: Option<i64>,
    pub backup_count: Option<usize>,
    pub last_backup_path: Option<String>,
    pub security: SecurityStatus,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaSummary {
    pub table_count: usize,
    pub detected_tables: Vec<String>,
    pub has_entries_table: bool,
    pub has_tags_table: bool,
    pub has_fts_table: bool,
    pub missing_core_tables: Vec<String>,
}

impl SchemaSummary {
    pub fn empty() -> Self {
        Self {
            table_count: 0,
            detected_tables: Vec::new(),
            has_entries_table: false,
            has_tags_table: false,
            has_fts_table: false,
            missing_core_tables: vec!["entries".to_string(), "tags".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityStatus {
    pub mode: SecurityMode,
    pub locked: bool,
    pub readable: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum SecurityMode {
    Plain,
    Aes,
    Sqlcipher,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub manifest_path: Option<String>,
    pub created_at: Option<String>,
    pub size_bytes: u64,
    pub operation: Option<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupListResponse {
    pub backups: Vec<BackupInfo>,
    pub backup_directory: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BackupCreateRequest {
    pub operation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCreateResponse {
    pub backup: BackupInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub created_at: String,
    pub operation: String,
    pub app: String,
    pub db_path: String,
    pub db_size_bytes: u64,
    pub backup_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestorePreviewRequest {
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestorePreview {
    pub backup: BackupInfo,
    pub db_size_bytes: u64,
    pub db_modified_at: Option<String>,
    pub schema_summary: SchemaSummary,
    pub entry_count: Option<i64>,
    pub tag_count: Option<i64>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreRequest {
    pub backup_path: String,
    pub confirmation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreResponse {
    pub restored_from: BackupInfo,
    pub safety_backup: BackupInfo,
    pub completed_at: String,
    pub status: DatabaseStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleConfigValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleConfigResponse {
    pub config_path: String,
    pub exists: bool,
    pub values: Vec<CapsuleConfigValue>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMutationResponse {
    pub config: CapsuleConfigResponse,
    pub backup_path: Option<String>,
    pub operation: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationConfigUpdateRequest {
    pub auto_capture: bool,
    pub use_default_location: bool,
    pub default_location_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathSettingsResponse {
    pub database_path: String,
    pub image_media_root: String,
    pub cover_wall_root: String,
    pub backup_directory: String,
    pub sync_path: Option<String>,
    pub github_gist_id: Option<String>,
    pub github_gist_token_configured: bool,
    pub auto_sync_enabled: bool,
    pub auto_sync_interval_minutes: i64,
    pub minimize_to_tray_on_close: bool,
    pub settings_path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PathSettingsUpdateRequest {
    pub database_path: Option<String>,
    pub image_media_root: Option<String>,
    pub cover_wall_root: Option<String>,
    pub backup_directory: Option<String>,
    pub sync_path: Option<String>,
    pub github_gist_id: Option<String>,
    pub github_gist_token: Option<String>,
    pub clear_github_gist_token: Option<bool>,
    pub auto_sync_enabled: Option<bool>,
    pub auto_sync_interval_minutes: Option<i64>,
    pub minimize_to_tray_on_close: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub cloud_provider: String,
    pub openai_model: String,
    pub gemini_model: String,
    pub openrouter_model: String,
    pub default_context_limit: Option<i64>,
    pub default_since: Option<String>,
    pub default_until: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsUpdateRequest {
    pub cloud_provider: String,
    pub openai_model: String,
    pub gemini_model: String,
    pub openrouter_model: String,
    pub default_context_limit: Option<i64>,
    pub default_since: Option<String>,
    pub default_until: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub provider: String,
    pub label: String,
    pub configured: bool,
    pub selected_model: String,
    pub available_models: Vec<String>,
    pub missing_reason: Option<String>,
    pub key_source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiApiKeyUpdateRequest {
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiApiKeyMutationResponse {
    pub provider_status: AiProviderStatus,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagUsage {
    pub id: i64,
    pub name: String,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCatalogResponse {
    pub tags: Vec<TagUsage>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRenameRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagMergeRequest {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDeleteRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagMutationResponse {
    pub tags: Vec<TagUsage>,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodUsage {
    pub name: String,
    pub label: String,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodCatalogResponse {
    pub moods: Vec<MoodUsage>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodRenameRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodDeleteRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodMutationResponse {
    pub moods: Vec<MoodUsage>,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTemplate {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub intro_text: String,
    pub sections: Vec<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPrompt {
    pub id: i64,
    pub slug: String,
    pub prompt_text: String,
    pub category: String,
    pub tags: Vec<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryListResponse {
    pub templates: Vec<LibraryTemplate>,
    pub prompts: Vec<LibraryPrompt>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTemplateInput {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub intro_text: Option<String>,
    pub sections: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTemplateUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub intro_text: Option<String>,
    pub sections: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTemplateMutationResponse {
    pub template: Option<LibraryTemplate>,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPromptInput {
    pub slug: String,
    pub prompt_text: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPromptUpdate {
    pub prompt_text: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPromptMutationResponse {
    pub prompt: Option<LibraryPrompt>,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phase6Capability {
    pub key: String,
    pub label: String,
    pub available: bool,
    pub configured: bool,
    pub requires_cloud: bool,
    pub read_only: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConversationSummary {
    pub id: i64,
    pub uuid: Option<String>,
    pub title: String,
    pub preview: String,
    pub cloud_provider: String,
    pub scope: String,
    pub message_count: i64,
    pub last_message_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTimeCapsuleSummary {
    pub id: i64,
    pub trigger_label: String,
    pub due_date: String,
    pub status: String,
    pub source_entry_count: i64,
    pub cloud_provider: String,
    pub llm_model: String,
    pub read_at: Option<String>,
    pub dismissed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingModelSummary {
    pub id: i64,
    pub name: String,
    pub dimensions: i64,
    pub provider: String,
    pub is_active: bool,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiOverviewResponse {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub capabilities: Vec<Phase6Capability>,
    pub conversations: Vec<AiConversationSummary>,
    pub time_capsules: Vec<AiTimeCapsuleSummary>,
    pub embedding_models: Vec<EmbeddingModelSummary>,
    pub conversation_count: i64,
    pub message_count: i64,
    pub time_capsule_count: i64,
    pub embedded_entry_count: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMetadataSuggestionRequest {
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMetadataSuggestionResponse {
    pub entry_uuid: String,
    pub source: String,
    pub suggested_title: Option<String>,
    pub suggested_summary: Option<String>,
    pub suggested_mood: Option<String>,
    pub suggested_tags: Vec<String>,
    pub confidence: f64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusSummary {
    pub last_successful_sync_at: Option<String>,
    pub last_sync_file_path: Option<String>,
    pub last_sync_file_size_bytes: Option<i64>,
    pub last_sync_imported: i64,
    pub last_sync_updated: i64,
    pub last_sync_deleted: i64,
    pub last_sync_total: i64,
    pub last_sync_summary: Option<String>,
    pub last_conflict_count: i64,
    pub last_conflict_summary: Option<String>,
    pub last_sync_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncHistoryItem {
    pub id: i64,
    pub timestamp: String,
    pub status: String,
    pub sync_file_path: Option<String>,
    pub imported_count: i64,
    pub updated_count: i64,
    pub deleted_count: i64,
    pub exported_count: i64,
    pub conflict_count: i64,
    pub summary: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTombstoneCount {
    pub table: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOverviewResponse {
    pub configured: bool,
    pub sync_path: Option<String>,
    pub sync_file_path: Option<String>,
    pub github_gist_id: Option<String>,
    pub github_gist_token_configured: bool,
    pub auto_sync_enabled: bool,
    pub auto_sync_interval_minutes: i64,
    pub status: Option<SyncStatusSummary>,
    pub recent_history: Vec<SyncHistoryItem>,
    pub tombstones: Vec<SyncTombstoneCount>,
    pub capabilities: Vec<Phase6Capability>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunRequest {
    pub sync_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunResponse {
    pub sync_path: String,
    pub sync_file_path: String,
    pub github_gist_pulled: bool,
    pub github_gist_pushed: bool,
    pub imported_count: i64,
    pub updated_count: i64,
    pub deleted_count: i64,
    pub exported_count: i64,
    pub conflict_count: i64,
    pub summary: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub key: String,
    pub label: String,
    pub enabled: bool,
    pub installed_version: Option<String>,
    pub source: String,
    pub updated_at: Option<String>,
    pub implemented: bool,
    pub table_name: Option<String>,
    pub row_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginOverviewResponse {
    pub plugins: Vec<PluginInfo>,
    pub capabilities: Vec<Phase6Capability>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMutationRequest {
    pub plugin_name: String,
    pub enabled: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMutationResponse {
    pub plugin: PluginInfo,
    pub plugins: Vec<PluginInfo>,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamificationProfileSummary {
    pub hero_sprite_path: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamificationQuest {
    pub instance_id: String,
    pub quest_key: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub enemy_sprite_path: Option<String>,
    pub target_value: i64,
    pub progress_value: i64,
    pub reward_xp: i64,
    pub status: String,
    pub period_key: String,
    pub starts_at: String,
    pub expires_at: Option<String>,
    pub completed_at: Option<String>,
    pub claimed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamificationXpEvent {
    pub id: i64,
    pub source_type: String,
    pub source_key: String,
    pub amount: i64,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamificationBadge {
    pub badge_key: String,
    pub unlocked_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamificationOverviewResponse {
    pub profile: Option<GamificationProfileSummary>,
    pub total_xp: i64,
    pub level: i64,
    pub xp_to_next_level: i64,
    pub event_count: i64,
    pub recent_events: Vec<GamificationXpEvent>,
    pub quests: Vec<GamificationQuest>,
    pub badges: Vec<GamificationBadge>,
    pub capabilities: Vec<Phase6Capability>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestClaimRequest {
    pub instance_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestClaimResponse {
    pub quest: GamificationQuest,
    pub total_xp: i64,
    pub level: i64,
    pub xp_to_next_level: i64,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEntriesRequest {
    pub format: ExportFormat,
    pub uuids: Option<Vec<String>>,
    pub search: Option<SearchRequest>,
    pub filters: Option<EntryFilters>,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEntriesResponse {
    pub path: String,
    pub format: ExportFormat,
    pub entry_count: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInfo {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodInfo {
    pub name: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationInfo {
    pub latitude: f64,
    pub longitude: f64,
    pub place_name: Option<String>,
    pub source: Option<String>,
    pub weather_condition: Option<String>,
    pub weather_temp_c: Option<f64>,
    pub weather_temp_f: Option<f64>,
    pub weather_icon: Option<String>,
    pub weather_humidity: Option<i64>,
    pub weather_wind_kph: Option<f64>,
    pub weather_fetched_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryThreadInfo {
    pub root_uuid: String,
    pub parent_uuid: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub entry_count: usize,
    pub is_root: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: i64,
    pub uuid: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub text: String,
    pub text_plain: String,
    pub content_format: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub mood: Option<String>,
    pub mood_info: MoodInfo,
    pub tags: Vec<TagInfo>,
    pub starred: bool,
    pub pinned: bool,
    pub hidden: bool,
    pub location: Option<LocationInfo>,
    pub thread: Option<EntryThreadInfo>,
    pub attachment_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryListResponse {
    pub entries: Vec<Entry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EntryFilters {
    pub text: Option<String>,
    pub location: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
    pub moods: Option<Vec<String>>,
    pub exclude_moods: Option<Vec<String>>,
    pub starred: Option<bool>,
    pub pinned: Option<bool>,
    pub hidden: Option<bool>,
    pub include_hidden: Option<bool>,
    pub has_images: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort: Option<EntrySort>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntrySort {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RandomEntryFilters {
    pub include_hidden: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub moods: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EntryCreate {
    pub text: String,
    pub content_format: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub mood: Option<String>,
    pub tags: Option<Vec<String>>,
    pub when: Option<String>,
    pub starred: Option<bool>,
    pub pinned: Option<bool>,
    pub continue_from_uuid: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EntryUpdate {
    pub text: Option<String>,
    pub content_format: Option<String>,
    #[serde(default)]
    pub title: NullableStringUpdate,
    #[serde(default)]
    pub summary: NullableStringUpdate,
    #[serde(default)]
    pub mood: NullableStringUpdate,
    pub tags: Option<Vec<String>>,
    pub starred: Option<bool>,
    pub pinned: Option<bool>,
    pub hidden: Option<bool>,
    #[serde(default)]
    pub continue_from_uuid: NullableStringUpdate,
}

#[derive(Debug, Clone, Default)]
pub enum NullableStringUpdate {
    #[default]
    Missing,
    Null,
    Value(String),
}

impl NullableStringUpdate {
    pub fn is_present(&self) -> bool {
        !matches!(self, Self::Missing)
    }

    pub fn apply_to(&self, current: Option<String>) -> Option<String> {
        match self {
            Self::Missing => current,
            Self::Null => None,
            Self::Value(value) => Some(value.clone()),
        }
    }

    pub fn as_optional_value(&self) -> Option<Option<String>> {
        match self {
            Self::Missing => None,
            Self::Null => Some(None),
            Self::Value(value) => Some(Some(value.clone())),
        }
    }
}

impl<'de> Deserialize<'de> for NullableStringUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NullableStringVisitor;

        impl<'de> de::Visitor<'de> for NullableStringVisitor {
            type Value = NullableStringUpdate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a string, null, or an omitted field")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NullableStringUpdate::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NullableStringUpdate::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                String::deserialize(deserializer).map(NullableStringUpdate::Value)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NullableStringUpdate::Value(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NullableStringUpdate::Value(value))
            }
        }

        deserializer.deserialize_option(NullableStringVisitor)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationAudit {
    pub backup_path: String,
    pub operation: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryMutationResponse {
    pub entry: Entry,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEntryResponse {
    pub entry_id: i64,
    pub entry_uuid: String,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryHistoryItem {
    pub id: i64,
    pub timestamp: String,
    pub operation_type: String,
    pub old_data: JsonValue,
    pub changed_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryHistoryResponse {
    pub entry_id: i64,
    pub current: JsonValue,
    pub history: Vec<EntryHistoryItem>,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SearchMode {
    Keyword,
    Semantic,
    Hybrid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub mode: Option<SearchMode>,
    pub location: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
    pub moods: Option<Vec<String>>,
    pub exclude_moods: Option<Vec<String>>,
    pub starred: Option<bool>,
    pub pinned: Option<bool>,
    pub hidden: Option<bool>,
    pub include_hidden: Option<bool>,
    pub has_images: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort: Option<EntrySort>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StructuredTokenKind {
    Keyword,
    Tag,
    ExcludeTag,
    Mood,
    ExcludeMood,
    Before,
    After,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredQueryToken {
    pub kind: StructuredTokenKind,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub entries: Vec<Entry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub mode: SearchMode,
    pub used_fts: bool,
    pub parsed_tokens: Vec<StructuredQueryToken>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadGroup {
    pub root_uuid: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub latest_activity: Option<String>,
    pub entry_count: usize,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListResponse {
    pub threads: Vec<ThreadGroup>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMetadataUpdate {
    #[serde(default)]
    pub title: NullableStringUpdate,
    #[serde(default)]
    pub summary: NullableStringUpdate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkThreadDetachRequest {
    pub child_uuids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkThreadLinkRequest {
    pub parent_uuid: String,
    pub child_uuids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMutationResponse {
    pub thread: Option<ThreadGroup>,
    pub affected_uuids: Vec<String>,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub id: i64,
    pub hash: String,
    pub mime_type: String,
    pub bytes: i64,
    pub width: i64,
    pub height: i64,
    pub storage_backend: String,
    pub storage_key: String,
    pub created_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAttachment {
    pub attachment_id: i64,
    pub entry_uuid: String,
    pub media_id: i64,
    pub position: i64,
    pub caption: Option<String>,
    pub alt_text: Option<String>,
    pub created_at: String,
    pub hash: String,
    pub mime_type: String,
    pub bytes: i64,
    pub width: i64,
    pub height: i64,
    pub storage_backend: String,
    pub storage_key: String,
    pub deleted_at: Option<String>,
    pub thumbnail_available: bool,
    pub original_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageEntryListResponse {
    pub entry_uuid: String,
    pub images: Vec<ImageAttachment>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageEntriesItem {
    pub entry_uuid: String,
    pub images: Vec<ImageAttachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageEntriesListResponse {
    pub entries: Vec<ImageEntriesItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAttachRequest {
    pub identifier: String,
    pub media_id: i64,
    pub caption: Option<String>,
    pub alt_text: Option<String>,
    pub position: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUploadAttachItem {
    pub file_path: String,
    pub caption: Option<String>,
    pub alt_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUploadAttachRequest {
    pub identifier: String,
    pub images: Vec<ImageUploadAttachItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUploadResponse {
    pub asset: ImageAsset,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMutationResponse {
    pub entry_uuid: String,
    pub images: Vec<ImageAttachment>,
    pub audit: MutationAudit,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImageVariant {
    Thumb,
    Full,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsPeriodRequest {
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsOverview {
    pub total_entries: i64,
    pub total_words: i64,
    pub average_words: f64,
    pub average_mood_sentiment: Option<f64>,
    pub mood_sentiment_count: i64,
    pub total_images: i64,
    pub entries_with_images: i64,
    pub entries_with_location: i64,
    pub longest_streak_days: i64,
    pub current_streak_days: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsTrendPoint {
    pub period: String,
    pub entry_count: i64,
    pub word_count: i64,
    pub average_mood_sentiment: Option<f64>,
    pub mood_sentiment_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsBreakdownItem {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordCount {
    pub word: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsResponse {
    pub overview: AnalyticsOverview,
    pub monthly_trend: Vec<AnalyticsTrendPoint>,
    pub mood_breakdown: Vec<AnalyticsBreakdownItem>,
    pub tag_breakdown: Vec<AnalyticsBreakdownItem>,
    pub location_breakdown: Vec<AnalyticsBreakdownItem>,
    pub weather_breakdown: Vec<AnalyticsBreakdownItem>,
    pub top_words: Vec<WordCount>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingCalendarDay {
    pub date: String,
    pub entry_count: i64,
    pub word_count: i64,
    pub image_count: i64,
    pub moods: Vec<String>,
    pub average_mood_sentiment: Option<f64>,
    pub mood_sentiment_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingCalendarResponse {
    pub year: i32,
    pub days: Vec<WritingCalendarDay>,
    pub total_days: i64,
    pub active_days: i64,
    pub max_entry_count: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoverWallRequest {
    #[serde(rename = "type")]
    pub cover_type: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub tags: Option<Vec<String>>,
    pub moods: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverEntrySummary {
    pub id: i64,
    pub uuid: String,
    pub created_at: String,
    pub title: Option<String>,
    pub mood: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryCover {
    pub filename: String,
    pub cover_type: String,
    pub entry_uuid: String,
    pub bytes: u64,
    pub modified_at: Option<String>,
    pub entry: CoverEntrySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverWallResponse {
    pub covers: Vec<EntryCover>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub available_types: Vec<String>,
    pub orphaned_cover_count: i64,
    pub covers_root: String,
}
