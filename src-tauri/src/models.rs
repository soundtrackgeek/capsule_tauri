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
    pub weather_condition: Option<String>,
    pub weather_temp_c: Option<f64>,
    pub weather_temp_f: Option<f64>,
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
