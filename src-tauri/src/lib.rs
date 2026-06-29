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
    EntryFilters, EntryListResponse, RandomEntryFilters,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_database_status,
            list_backups,
            create_backup,
            list_entries,
            get_entry,
            get_random_entry
        ])
        .run(tauri::generate_context!())
        .expect("error while running Capsule Tauri app");
}
