//! Desktop compatibility adapters for the shared headless search service.
//!
//! The old Tauri command surface keeps resolving the active database and
//! exposing the same model types, while the query implementation lives in
//! `capsule-core`. The desktop path intentionally retains its historical
//! repair behavior in the adapter; non-desktop callers use the core's
//! repair-free entry point directly.

use anyhow::Result;
use std::path::Path;

use crate::models::{SearchRequest, SearchResponse};

pub fn search_entries(input: SearchRequest) -> Result<SearchResponse> {
    let db_path = crate::db::resolve_database_path();
    crate::entries::ensure_entry_ids_for_database(&db_path)?;
    capsule_core::search::search_entries_for_database(&db_path, input)
}

pub(crate) fn search_entries_for_database(
    db_path: &Path,
    input: SearchRequest,
) -> Result<SearchResponse> {
    // Preserve the desktop convenience behavior that predates the headless
    // boundary. The core implementation remains strictly repair-free.
    crate::entries::ensure_entry_ids_for_database(db_path)?;
    capsule_core::search::search_entries_for_database(db_path, input)
}
