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
    BackupCreateRequest, BackupCreateResponse, BackupListResponse, DatabaseStatus, Entry,
    EntryCreate, EntryFilters, EntryHistoryResponse, EntryListResponse, EntryMutationResponse,
    EntryUpdate, RandomEntryFilters,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_database_status,
            list_backups,
            create_backup,
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
            list_entry_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running Capsule Tauri app");
}
