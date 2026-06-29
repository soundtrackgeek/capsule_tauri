mod backup;
mod db;
mod entries;
mod images;
mod models;
mod search;
mod security;
mod settings;
mod stats;
mod threads;

use models::{
    BackupCreateRequest, BackupCreateResponse, BackupListResponse, BackupRestorePreview,
    BackupRestorePreviewRequest, BackupRestoreRequest, BackupRestoreResponse,
    BulkThreadDetachRequest, BulkThreadLinkRequest, CapsuleConfigResponse, ConfigMutationResponse,
    DatabaseStatus, Entry, EntryCreate, EntryFilters, EntryHistoryResponse, EntryListResponse,
    EntryMutationResponse, EntryUpdate, ExportEntriesRequest, ExportEntriesResponse,
    LibraryListResponse, LibraryPromptInput, LibraryPromptMutationResponse, LibraryPromptUpdate,
    LibraryTemplateInput, LibraryTemplateMutationResponse, LibraryTemplateUpdate,
    MoodCatalogResponse, MoodDeleteRequest, MoodMutationResponse, MoodRenameRequest,
    RandomEntryFilters, SearchRequest, SearchResponse, TagCatalogResponse, TagDeleteRequest,
    TagMergeRequest, TagMutationResponse, TagRenameRequest, ThreadListResponse,
    ThreadMetadataUpdate, ThreadMutationResponse,
};

#[tauri::command]
async fn get_database_status() -> Result<DatabaseStatus, String> {
    tauri::async_runtime::spawn_blocking(db::database_status)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_backups() -> Result<BackupListResponse, String> {
    tauri::async_runtime::spawn_blocking(backup::list_backups)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_backup(input: Option<BackupCreateRequest>) -> Result<BackupCreateResponse, String> {
    tauri::async_runtime::spawn_blocking(move || backup::create_backup(input.unwrap_or_default()))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn preview_restore_backup(
    input: BackupRestorePreviewRequest,
) -> Result<BackupRestorePreview, String> {
    tauri::async_runtime::spawn_blocking(move || backup::preview_restore_backup(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn restore_backup(input: BackupRestoreRequest) -> Result<BackupRestoreResponse, String> {
    tauri::async_runtime::spawn_blocking(move || backup::restore_backup(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn open_backup_folder() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(backup::open_backup_folder)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_entries(filters: Option<EntryFilters>) -> Result<EntryListResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::list_entries(filters.unwrap_or_default()))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_entry(identifier: String) -> Result<Entry, String> {
    tauri::async_runtime::spawn_blocking(move || entries::get_entry(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_random_entry(filters: Option<RandomEntryFilters>) -> Result<Option<Entry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        entries::get_random_entry(filters.unwrap_or_default())
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_entry(input: EntryCreate) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::create_entry(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_entry(
    identifier: String,
    input: EntryUpdate,
) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::update_entry(identifier, input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn star_entry(identifier: String) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::star_entry(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn unstar_entry(identifier: String) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::unstar_entry(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn pin_entry(identifier: String) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::pin_entry(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn unpin_entry(identifier: String) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::unpin_entry(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn hide_entry(identifier: String) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::hide_entry(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn unhide_entry(identifier: String) -> Result<EntryMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::unhide_entry(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_entry_history(identifier: String) -> Result<EntryHistoryResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::list_entry_history(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn search_entries(input: SearchRequest) -> Result<SearchResponse, String> {
    tauri::async_runtime::spawn_blocking(move || search::search_entries(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_threads(
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ThreadListResponse, String> {
    tauri::async_runtime::spawn_blocking(move || threads::list_threads(limit, offset))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_thread_title(
    root_uuid: String,
    title: Option<String>,
) -> Result<ThreadMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || threads::update_thread_title(root_uuid, title))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_thread_metadata(
    root_uuid: String,
    input: ThreadMetadataUpdate,
) -> Result<ThreadMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || threads::update_thread_metadata(root_uuid, input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn bulk_link_threads(input: BulkThreadLinkRequest) -> Result<ThreadMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || threads::bulk_link_threads(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn bulk_detach_threads(
    input: BulkThreadDetachRequest,
) -> Result<ThreadMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || threads::bulk_detach_threads(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn disband_thread(root_uuid: String) -> Result<ThreadMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || threads::disband_thread(root_uuid))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_capsule_config() -> Result<CapsuleConfigResponse, String> {
    tauri::async_runtime::spawn_blocking(settings::get_capsule_config)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_capsule_config_value(
    key: String,
    value: String,
) -> Result<ConfigMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::set_capsule_config_value(key, value))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_capsule_config_value(key: String) -> Result<ConfigMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::delete_capsule_config_value(key))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_tags() -> Result<TagCatalogResponse, String> {
    tauri::async_runtime::spawn_blocking(settings::list_tags)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn rename_tag(input: TagRenameRequest) -> Result<TagMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::rename_tag(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn merge_tag(input: TagMergeRequest) -> Result<TagMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::merge_tag(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_tag(input: TagDeleteRequest) -> Result<TagMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::delete_tag(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_moods() -> Result<MoodCatalogResponse, String> {
    tauri::async_runtime::spawn_blocking(settings::list_moods)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn rename_mood(input: MoodRenameRequest) -> Result<MoodMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::rename_mood(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_mood(input: MoodDeleteRequest) -> Result<MoodMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::delete_mood(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_library_items() -> Result<LibraryListResponse, String> {
    tauri::async_runtime::spawn_blocking(settings::list_library_items)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_template(
    input: LibraryTemplateInput,
) -> Result<LibraryTemplateMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::create_template(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_template(
    slug: String,
    input: LibraryTemplateUpdate,
) -> Result<LibraryTemplateMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::update_template(slug, input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_template(slug: String) -> Result<LibraryTemplateMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::delete_template(slug))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_prompt(input: LibraryPromptInput) -> Result<LibraryPromptMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::create_prompt(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_prompt(
    slug: String,
    input: LibraryPromptUpdate,
) -> Result<LibraryPromptMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::update_prompt(slug, input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_prompt(slug: String) -> Result<LibraryPromptMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::delete_prompt(slug))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_entries(input: ExportEntriesRequest) -> Result<ExportEntriesResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::export_entries(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_database_status,
            list_backups,
            create_backup,
            preview_restore_backup,
            restore_backup,
            open_backup_folder,
            list_entries,
            get_entry,
            get_random_entry,
            create_entry,
            update_entry,
            star_entry,
            unstar_entry,
            pin_entry,
            unpin_entry,
            hide_entry,
            unhide_entry,
            list_entry_history,
            search_entries,
            list_threads,
            update_thread_title,
            update_thread_metadata,
            bulk_link_threads,
            bulk_detach_threads,
            disband_thread,
            get_capsule_config,
            set_capsule_config_value,
            delete_capsule_config_value,
            list_tags,
            rename_tag,
            merge_tag,
            delete_tag,
            list_moods,
            rename_mood,
            delete_mood,
            list_library_items,
            create_template,
            update_template,
            delete_template,
            create_prompt,
            update_prompt,
            delete_prompt,
            export_entries
        ])
        .run(tauri::generate_context!())
        .expect("error while running Capsule Tauri app");
}
