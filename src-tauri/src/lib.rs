mod ai_chat;
mod ai_config;
mod ai_metadata;
mod ai_providers;
mod backup;
mod db;
mod debug;
mod entries;
mod images;
mod location;
mod models;
mod mood_sentiment;
mod phase6;
mod search;
mod security;
mod settings;
mod stats;
mod sync;
mod threads;

use models::{
    AiApiKeyMutationResponse, AiApiKeyUpdateRequest, AiChatContextPreviewRequest,
    AiChatContextPreviewResponse, AiChatRequest, AiChatRetryRequest, AiChatStreamStartResponse,
    AiConversationDetail, AiConversationListResponse, AiEntryMetadataSuggestionRequest,
    AiEntryMetadataSuggestionResponse, AiMetadataSuggestionRequest, AiMetadataSuggestionResponse,
    AiOverviewResponse, AiProviderStatus, AiSettings, AiSettingsUpdateRequest,
    AnalyticsPeriodRequest, AnalyticsResponse, BackupCreateRequest, BackupCreateResponse,
    BackupListResponse, BackupRestorePreview, BackupRestorePreviewRequest, BackupRestoreRequest,
    BackupRestoreResponse, BulkThreadDetachRequest, BulkThreadLinkRequest, CapsuleConfigResponse,
    ConfigMutationResponse, CoverWallRequest, CoverWallResponse, DatabaseStatus,
    DebugBundleResponse, DebugDiagnosticsResponse, DebugLogRequest, DebugLogResponse,
    DeleteAiConversationResponse, DeleteEntryResponse, Entry, EntryCreate, EntryFilters,
    EntryHistoryResponse, EntryListResponse, EntryMutationResponse, EntryUpdate,
    ExportEntriesRequest, ExportEntriesResponse, GamificationOverviewResponse, ImageAttachRequest,
    ImageEntriesListResponse, ImageEntryListResponse, ImageMutationResponse,
    ImageUploadAttachRequest, ImageUploadResponse, ImageVariant, LibraryListResponse,
    LibraryPromptInput, LibraryPromptMutationResponse, LibraryPromptUpdate, LibraryTemplateInput,
    LibraryTemplateMutationResponse, LibraryTemplateUpdate, LocationConfigUpdateRequest,
    MoodCatalogResponse, MoodDeleteRequest, MoodMutationResponse, MoodRenameRequest,
    PathSettingsResponse, PathSettingsUpdateRequest, PluginOverviewResponse, QuestClaimRequest,
    QuestClaimResponse, RandomEntryFilters, SearchRequest, SearchResponse, SyncOverviewResponse,
    SyncRunRequest, SyncRunResponse, TagCatalogResponse, TagDeleteRequest, TagMergeRequest,
    TagMutationResponse, TagRenameRequest, ThreadListResponse, ThreadMetadataUpdate,
    ThreadMutationResponse, WritingCalendarResponse,
};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
use tauri_plugin_autostart::ManagerExt;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

const APP_ICON_BYTES: &[u8] = include_bytes!("../icons/icon-256.png");
const WINDOWS_APP_USER_MODEL_ID: &str = "com.local.capsule";
const TRAY_OPEN_VIEW_EVENT: &str = "capsule://open-view";
const TRAY_OPEN_INTERFACE_ID: &str = "tray-open-interface";
const TRAY_OPEN_WRITER_ID: &str = "tray-open-writer";
const TRAY_OPEN_SETTINGS_ID: &str = "tray-open-settings";
const TRAY_QUIT_ID: &str = "tray-quit";
const START_IN_TRAY_ARG: &str = "--start-in-tray";

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
fn window_state_flags() -> tauri_plugin_window_state::StateFlags {
    use tauri_plugin_window_state::StateFlags;

    StateFlags::SIZE
        | StateFlags::POSITION
        | StateFlags::MAXIMIZED
        | StateFlags::DECORATIONS
        | StateFlags::FULLSCREEN
}

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
async fn get_debug_diagnostics() -> Result<DebugDiagnosticsResponse, String> {
    tauri::async_runtime::spawn_blocking(debug::get_debug_diagnostics)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn append_debug_log(input: DebugLogRequest) -> Result<DebugLogResponse, String> {
    tauri::async_runtime::spawn_blocking(move || debug::append_debug_log(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_debug_bundle() -> Result<DebugBundleResponse, String> {
    tauri::async_runtime::spawn_blocking(debug::create_debug_bundle)
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
async fn get_path_settings(app: tauri::AppHandle) -> Result<PathSettingsResponse, String> {
    let mut response = tauri::async_runtime::spawn_blocking(settings::get_path_settings)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    response.start_with_windows = start_with_windows_enabled(&app)?;
    Ok(response)
}

#[tauri::command]
async fn set_path_settings(
    app: tauri::AppHandle,
    input: PathSettingsUpdateRequest,
) -> Result<PathSettingsResponse, String> {
    if let Some(enabled) = input.start_with_windows {
        set_start_with_windows(&app, enabled)?;
    }

    let mut response =
        tauri::async_runtime::spawn_blocking(move || settings::set_path_settings(input))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
    response.start_with_windows = start_with_windows_enabled(&app)?;
    Ok(response)
}

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
fn start_with_windows_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("Unable to read the Windows startup setting: {error}"))
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
fn start_with_windows_enabled<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
) -> Result<bool, String> {
    Ok(false)
}

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
fn set_start_with_windows<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|error| format!("Unable to update the Windows startup setting: {error}"))
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
fn set_start_with_windows<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    if enabled {
        Err("Starting with Windows is only available in the desktop app.".to_string())
    } else {
        Ok(())
    }
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
async fn get_ai_settings() -> Result<AiSettings, String> {
    tauri::async_runtime::spawn_blocking(ai_config::get_ai_settings)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_ai_provider_status() -> Result<Vec<AiProviderStatus>, String> {
    tauri::async_runtime::spawn_blocking(ai_config::get_ai_provider_status)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_ai_settings(
    input: AiSettingsUpdateRequest,
) -> Result<ConfigMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_config::update_ai_settings(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn set_ai_api_key(input: AiApiKeyUpdateRequest) -> Result<AiApiKeyMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_config::set_ai_api_key(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn clear_ai_api_key(provider: String) -> Result<AiApiKeyMutationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_config::clear_ai_api_key(provider))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn preview_ai_chat_context(
    input: AiChatContextPreviewRequest,
) -> Result<AiChatContextPreviewResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::preview_ai_chat_context(input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_ai_conversations() -> Result<AiConversationListResponse, String> {
    tauri::async_runtime::spawn_blocking(ai_chat::list_ai_conversations)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_ai_conversation(conversation_id: i64) -> Result<AiConversationDetail, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::get_ai_conversation(conversation_id))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_ai_conversation(
    conversation_id: i64,
) -> Result<DeleteAiConversationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::delete_ai_conversation(conversation_id))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_ai_chat_stream(
    app: tauri::AppHandle,
    input: AiChatRequest,
) -> Result<AiChatStreamStartResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::start_ai_chat_stream(app, input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn retry_ai_chat_stream(
    app: tauri::AppHandle,
    input: AiChatRetryRequest,
) -> Result<AiChatStreamStartResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::retry_ai_chat_stream(app, input))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn cancel_ai_chat_stream(stream_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || ai_chat::cancel_ai_chat_stream(stream_id))
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
async fn suggest_ai_entry_metadata(
    input: AiEntryMetadataSuggestionRequest,
) -> Result<AiEntryMetadataSuggestionResponse, String> {
    tauri::async_runtime::spawn_blocking(move || ai_metadata::suggest_ai_entry_metadata(input))
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
async fn set_update_restart_window_request(requested: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        db::set_show_window_after_update_restart(requested)
    })
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
    set_windows_app_user_model_id();

    let builder = tauri::Builder::default();

    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Err(error) = open_main_window(app) {
            eprintln!("Failed to show running Capsule instance: {error}");
        }
    }));

    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());

    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    let builder = builder.plugin(
        tauri_plugin_autostart::Builder::new()
            .arg(START_IN_TRAY_ARG)
            .app_name("Capsule")
            .build(),
    );

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                if event.state == ShortcutState::Pressed
                    && shortcut.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyW)
                {
                    let _ = open_app_view(app, "writer");
                }
            })
            .build(),
    );

    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    let builder = builder.plugin(
        tauri_plugin_window_state::Builder::default()
            .with_state_flags(window_state_flags())
            .build(),
    );

    builder
        .setup(|app| {
            set_main_window_icon(app)?;
            setup_tray(app)?;
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            setup_global_shortcuts(app.handle());
            show_main_window_on_startup(app.handle());
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_OPEN_INTERFACE_ID => {
                let _ = open_main_window(app);
            }
            TRAY_OPEN_WRITER_ID => {
                let _ = open_app_view(app, "writer");
            }
            TRAY_OPEN_SETTINGS_ID => {
                let _ = open_app_view(app, "settings");
            }
            TRAY_QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|app, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                let _ = open_main_window(app);
            }
            _ => {}
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if db::read_local_path_settings()
                    .minimize_to_tray_on_close
                    .unwrap_or(false)
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_database_status,
            list_backups,
            create_backup,
            preview_restore_backup,
            restore_backup,
            open_backup_folder,
            get_debug_diagnostics,
            append_debug_log,
            create_debug_bundle,
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
            get_ai_settings,
            get_ai_provider_status,
            update_ai_settings,
            set_ai_api_key,
            clear_ai_api_key,
            preview_ai_chat_context,
            list_ai_conversations,
            get_ai_conversation,
            delete_ai_conversation,
            start_ai_chat_stream,
            retry_ai_chat_stream,
            cancel_ai_chat_stream,
            get_ai_overview,
            suggest_ai_metadata,
            suggest_ai_entry_metadata,
            get_sync_overview,
            run_sync,
            get_plugin_overview,
            get_gamification_overview,
            set_update_restart_window_request,
            claim_quest
        ])
        .run(tauri::generate_context!())
        .expect("error while running Capsule Tauri app");
}

fn set_main_window_icon<R: tauri::Runtime>(
    app: &mut tauri::App<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_icon(load_app_icon()?)?;
    }

    Ok(())
}

fn setup_tray<R: tauri::Runtime>(
    app: &mut tauri::App<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    let menu = MenuBuilder::new(app)
        .text(TRAY_OPEN_INTERFACE_ID, "Open Interface")
        .text(TRAY_OPEN_WRITER_ID, "Writer")
        .text(TRAY_OPEN_SETTINGS_ID, "Settings")
        .text(TRAY_QUIT_ID, "Quit")
        .build()?;

    TrayIconBuilder::new()
        .icon(load_app_icon()?)
        .tooltip("Capsule")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .build(app)?;

    Ok(())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn setup_global_shortcuts<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Err(error) = app.global_shortcut().register("ctrl+alt+w") {
        eprintln!("Failed to register Ctrl+Alt+W global shortcut: {error}");
    }
}

fn show_main_window_on_startup<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let show_after_update = match db::consume_show_window_after_update_restart() {
        Ok(requested) => requested,
        Err(error) => {
            eprintln!("Failed to read Capsule update restart window request: {error}");
            false
        }
    };

    if !should_show_main_window_on_startup(std::env::args_os(), show_after_update) {
        return;
    }

    if let Err(error) = open_main_window(app) {
        eprintln!("Failed to show Capsule on startup: {error}");
    }
}

fn should_show_main_window_on_startup<I, S>(args: I, show_after_update: bool) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    show_after_update
        || !args
            .into_iter()
            .any(|arg| arg.as_ref() == std::ffi::OsStr::new(START_IN_TRAY_ARG))
}

fn open_app_view<R: tauri::Runtime>(app: &tauri::AppHandle<R>, view: &str) -> tauri::Result<()> {
    open_main_window(app)?;
    app.emit(TRAY_OPEN_VIEW_EVENT, view)?;
    Ok(())
}

fn open_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
    }

    Ok(())
}

fn load_app_icon() -> Result<tauri::image::Image<'static>, ::image::ImageError> {
    let icon_rgba = ::image::load_from_memory(APP_ICON_BYTES)?.into_rgba8();
    let (width, height) = icon_rgba.dimensions();
    Ok(tauri::image::Image::new_owned(
        icon_rgba.into_raw(),
        width,
        height,
    ))
}

#[cfg(windows)]
fn set_windows_app_user_model_id() {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id = WINDOWS_APP_USER_MODEL_ID
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();

    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());
    }
}

#[cfg(not(windows))]
fn set_windows_app_user_model_id() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_launch_opens_the_main_window() {
        assert!(should_show_main_window_on_startup(
            ["capsule-tauri.exe"],
            false
        ));
    }

    #[test]
    fn windows_startup_launch_stays_in_the_tray() {
        assert!(!should_show_main_window_on_startup(
            ["capsule-tauri.exe", START_IN_TRAY_ARG],
            false
        ));
    }

    #[test]
    fn update_restart_overrides_the_tray_start_argument() {
        assert!(should_show_main_window_on_startup(
            ["capsule-tauri.exe", START_IN_TRAY_ARG],
            true
        ));
    }
}
