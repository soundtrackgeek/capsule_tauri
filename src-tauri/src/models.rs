use serde::{Deserialize, Serialize};

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
