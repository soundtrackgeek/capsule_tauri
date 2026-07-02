mod backup;
mod db;
mod entries;
mod images;
mod location;
mod models;
mod phase6;
mod search;
mod security;
mod settings;
mod stats;
mod sync;
mod threads;

use models::{
    AiMetadataSuggestionRequest, AiMetadataSuggestionResponse, AiOverviewResponse,
    AnalyticsPeriodRequest, AnalyticsResponse, BackupCreateRequest, BackupCreateResponse,
    BackupListResponse, BackupRestorePreview, BackupRestorePreviewRequest, BackupRestoreRequest,
    BackupRestoreResponse, BulkThreadDetachRequest, BulkThreadLinkRequest, CapsuleConfigResponse,
    ConfigMutationResponse, CoverWallRequest, CoverWallResponse, DatabaseStatus,
    DeleteEntryResponse, Entry, EntryCreate, EntryFilters, EntryHistoryResponse, EntryListResponse,
    EntryMutationResponse, EntryUpdate, ExportEntriesRequest, ExportEntriesResponse,
    GamificationOverviewResponse, ImageAttachRequest, ImageEntriesListResponse,
    ImageEntryListResponse, ImageMutationResponse, ImageUploadAttachRequest, ImageUploadResponse,
    ImageVariant, LibraryListResponse, LibraryPromptInput, LibraryPromptMutationResponse,
    LibraryPromptUpdate, LibraryTemplateInput, LibraryTemplateMutationResponse,
    LibraryTemplateUpdate, LocationConfigUpdateRequest, MoodCatalogResponse, MoodDeleteRequest,
    MoodMutationResponse, MoodRenameRequest, PathSettingsResponse, PathSettingsUpdateRequest,
    PluginOverviewResponse, QuestClaimRequest, QuestClaimResponse, RandomEntryFilters,
    SearchRequest, SearchResponse, SyncOverviewResponse, SyncRunRequest, SyncRunResponse,
    TagCatalogResponse, TagDeleteRequest, TagMergeRequest, TagMutationResponse, TagRenameRequest,
    ThreadListResponse, ThreadMetadataUpdate, ThreadMutationResponse, WritingCalendarResponse,
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
async fn delete_entry(identifier: String) -> Result<DeleteEntryResponse, String> {
    tauri::async_runtime::spawn_blocking(move || entries::delete_entry(identifier))
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
async fn list_entry_images(identifier: String) -> Result<ImageEntryListResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images::list_entry_images(identifier))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_image_media_root() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(images::get_image_media_root)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_images_for_entries(uuids: Vec<String>) -> Result<ImageEntriesListResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images::list_images_for_entries(uuids))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_image_data_url(attachment_id: i64, variant: ImageVariant) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || images::get_image_data_url(attachment_id, variant))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_local_image_preview_data_url(file_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        images::get_local_image_preview_data_url(file_path)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn upload_image(file_path: String) -> Result<ImageUploadResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images::upload_image(file_path))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn attach_image(input: ImageAttachRequest) -> Result<ImageMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images::attach_image(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn upload_and_attach_images(
    input: ImageUploadAttachRequest,
) -> Result<ImageMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images::upload_and_attach_images(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn remove_image(
    attachment_id: i64,
    identifier: Option<String>,
) -> Result<ImageMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images::remove_image(attachment_id, identifier))
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
async fn get_analytics(input: Option<AnalyticsPeriodRequest>) -> Result<AnalyticsResponse, String> {
    tauri::async_runtime::spawn_blocking(move || stats::get_analytics(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_writing_calendar(year: Option<i32>) -> Result<WritingCalendarResponse, String> {
    tauri::async_runtime::spawn_blocking(move || stats::get_writing_calendar(year))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_cover_wall(input: Option<CoverWallRequest>) -> Result<CoverWallResponse, String> {
    tauri::async_runtime::spawn_blocking(move || images::list_cover_wall(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_cover_data_url(filename: String, variant: ImageVariant) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || images::get_cover_data_url(filename, variant))
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
async fn set_location_config(
    input: LocationConfigUpdateRequest,
) -> Result<ConfigMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::set_location_config(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_path_settings() -> Result<PathSettingsResponse, String> {
    tauri::async_runtime::spawn_blocking(settings::get_path_settings)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_path_settings(
    input: PathSettingsUpdateRequest,
) -> Result<PathSettingsResponse, String> {
    tauri::async_runtime::spawn_blocking(move || settings::set_path_settings(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn browse_database_path(current_path: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || settings::browse_database_path(current_path))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn browse_directory_path(current_path: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || settings::browse_directory_path(current_path))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn browse_image_path(current_path: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || settings::browse_image_path(current_path))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn browse_image_paths(current_path: Option<String>) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || settings::browse_image_paths(current_path))
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

#[tauri::command]
async fn get_ai_overview() -> Result<AiOverviewResponse, String> {
    tauri::async_runtime::spawn_blocking(phase6::get_ai_overview)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn suggest_ai_metadata(
    input: AiMetadataSuggestionRequest,
) -> Result<AiMetadataSuggestionResponse, String> {
    tauri::async_runtime::spawn_blocking(move || phase6::suggest_ai_metadata(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_sync_overview() -> Result<SyncOverviewResponse, String> {
    tauri::async_runtime::spawn_blocking(phase6::get_sync_overview)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn run_sync(input: Option<SyncRunRequest>) -> Result<SyncRunResponse, String> {
    tauri::async_runtime::spawn_blocking(move || sync::run_sync(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_plugin_overview() -> Result<PluginOverviewResponse, String> {
    tauri::async_runtime::spawn_blocking(phase6::get_plugin_overview)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_gamification_overview() -> Result<GamificationOverviewResponse, String> {
    tauri::async_runtime::spawn_blocking(phase6::get_gamification_overview)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn claim_quest(input: QuestClaimRequest) -> Result<QuestClaimResponse, String> {
    tauri::async_runtime::spawn_blocking(move || phase6::claim_quest(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            delete_entry,
            star_entry,
            unstar_entry,
            pin_entry,
            unpin_entry,
            hide_entry,
            unhide_entry,
            list_entry_history,
            list_entry_images,
            get_image_media_root,
            list_images_for_entries,
            get_image_data_url,
            get_local_image_preview_data_url,
            upload_image,
            attach_image,
            upload_and_attach_images,
            remove_image,
            search_entries,
            get_analytics,
            get_writing_calendar,
            list_cover_wall,
            get_cover_data_url,
            list_threads,
            update_thread_title,
            update_thread_metadata,
            bulk_link_threads,
            bulk_detach_threads,
            disband_thread,
            get_capsule_config,
            set_capsule_config_value,
            delete_capsule_config_value,
            set_location_config,
            get_path_settings,
            set_path_settings,
            browse_database_path,
            browse_directory_path,
            browse_image_path,
            browse_image_paths,
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
            export_entries,
            get_ai_overview,
            suggest_ai_metadata,
            get_sync_overview,
            run_sync,
            get_plugin_overview,
            get_gamification_overview,
            claim_quest
        ])
        .run(tauri::generate_context!())
        .expect("error while running Capsule Tauri app");
}
