use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::{
    backup, db, location,
    models::{
        DeleteEntryResponse, Entry, EntryCreate, EntryFilters, EntryHistoryItem,
        EntryHistoryResponse, EntryListResponse, EntryMutationResponse, EntrySort, EntryThreadInfo,
        EntryUpdate, LocationInfo, MoodInfo, RandomEntryFilters, TagInfo,
    },
};

const DEFAULT_LIMIT: i64 = 40;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Clone)]
struct RawEntry {
    id: i64,
    uuid: String,
    created_at: String,
    updated_at: Option<String>,
    text: String,
    text_plain: String,
    content_format: String,
    title: Option<String>,
    summary: Option<String>,
    mood: Option<String>,
    starred: bool,
    pinned: bool,
    hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntrySnapshot {
    id: i64,
    uuid: String,
    created_at: String,
    updated_at: Option<String>,
    text: String,
    text_plain: String,
    content_format: String,
    title: Option<String>,
    summary: Option<String>,
    mood: Option<String>,
    tags: Vec<String>,
    starred: bool,
    pinned: bool,
    hidden: bool,
}

#[derive(Debug, Clone, Default)]
struct EntryRelations {
    tags_by_entry_id: HashMap<i64, Vec<TagInfo>>,
    locations_by_uuid: HashMap<String, LocationInfo>,
    attachment_counts_by_uuid: HashMap<String, i64>,
    thread_context: ThreadContext,
}

#[derive(Debug, Clone, Default)]
struct ThreadContext {
    parent_by_child: HashMap<String, String>,
    nodes: HashSet<String>,
    title_by_root: HashMap<String, String>,
    summary_by_root: HashMap<String, String>,
    count_by_root: HashMap<String, usize>,
}

impl ThreadContext {
    fn finalize_counts(&mut self) {
        let nodes = self.nodes.iter().cloned().collect::<Vec<_>>();
        for node in nodes {
            let root = self.root_for(&node);
            *self.count_by_root.entry(root).or_insert(0) += 1;
        }
    }

    fn info_for(&self, uuid: &str) -> Option<EntryThreadInfo> {
        if !self.nodes.contains(uuid) {
            return None;
        }

        let root_uuid = self.root_for(uuid);
        Some(EntryThreadInfo {
            parent_uuid: self.parent_by_child.get(uuid).cloned(),
            title: self.title_by_root.get(&root_uuid).cloned(),
            summary: self.summary_by_root.get(&root_uuid).cloned(),
            entry_count: self.count_by_root.get(&root_uuid).copied().unwrap_or(1),
            is_root: root_uuid == uuid,
            root_uuid,
        })
    }

    fn root_for(&self, uuid: &str) -> String {
        let mut current = uuid.to_string();
        let mut seen = HashSet::new();

        while let Some(parent) = self.parent_by_child.get(&current) {
            if !seen.insert(current.clone()) {
                break;
            }
            current = parent.clone();
        }

        current
    }
}

#[derive(Debug, Clone)]
struct QueryParts {
    where_sql: String,
    params: Vec<Value>,
    limit: i64,
    offset: i64,
    sort: EntrySort,
}

#[derive(Debug, Clone)]
struct EntryIdColumnInfo {
    exists: bool,
    primary_key: bool,
}

#[derive(Debug, Clone)]
struct EntryIdRepairRow {
    rowid: i64,
    current_id: Option<i64>,
    reference_id: Option<i64>,
    repaired_id: i64,
}

#[derive(Debug, Clone)]
struct DeletedEntry {
    entry_id: i64,
    entry_uuid: String,
}

#[derive(Debug, Clone, Serialize)]
struct ContinuationSnapshot {
    child_entry_uuid: String,
    parent_entry_uuid: String,
    updated_at: Option<String>,
}

#[derive(Debug, Clone)]
struct ImageAttachmentForDelete {
    media_id: i64,
    hash: String,
    position: i64,
    caption: Option<String>,
    alt_text: Option<String>,
}

pub fn list_entries(filters: EntryFilters) -> Result<EntryListResponse> {
    list_entries_for_database(&db::resolve_database_path(), filters)
}

pub fn list_entries_for_database(
    db_path: &Path,
    filters: EntryFilters,
) -> Result<EntryListResponse> {
    ensure_entry_ids_for_database(db_path)?;
    let connection = open_entries_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    ensure_entries_table(&tables)?;

    let query = build_query(&filters, &tables);
    let total = count_entries(&connection, &query)?;
    let raw_entries = query_entries(&connection, &query, false)?;
    let relations = load_relations(&connection, &tables, &raw_entries)?;
    let entries = raw_entries
        .into_iter()
        .map(|entry| build_entry(entry, &relations))
        .collect();

    Ok(EntryListResponse {
        entries,
        total,
        limit: query.limit,
        offset: query.offset,
    })
}

pub fn get_entry(identifier: String) -> Result<Entry> {
    get_entry_for_database(&db::resolve_database_path(), &identifier)
}

pub fn get_entry_for_database(db_path: &Path, identifier: &str) -> Result<Entry> {
    ensure_entry_ids_for_database(db_path)?;
    let connection = open_entries_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    ensure_entries_table(&tables)?;

    let mut statement = connection.prepare(&format!(
        "{} WHERE e.uuid = ?1 OR CAST(e.id AS TEXT) = ?1 LIMIT 1",
        entry_select_sql()
    ))?;
    let raw_entry = statement
        .query_row([identifier], raw_entry_from_row)
        .with_context(|| format!("entry not found: {identifier}"))?;
    let relations = load_relations(&connection, &tables, std::slice::from_ref(&raw_entry))?;

    Ok(build_entry(raw_entry, &relations))
}

pub(crate) fn list_entries_by_uuids_for_database(
    db_path: &Path,
    uuids: &[String],
) -> Result<Vec<Entry>> {
    if uuids.is_empty() {
        return Ok(Vec::new());
    }

    ensure_entry_ids_for_database(db_path)?;
    let connection = open_entries_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    ensure_entries_table(&tables)?;

    let placeholders = placeholders(uuids.len());
    let sql = format!("{} WHERE e.uuid IN ({placeholders})", entry_select_sql());
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params_from_iter(uuids.iter().cloned().map(Value::Text)),
        raw_entry_from_row,
    )?;
    let raw_entries = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load entries by UUID")?;
    let relations = load_relations(&connection, &tables, &raw_entries)?;
    let entries_by_uuid = raw_entries
        .into_iter()
        .map(|entry| {
            let uuid = entry.uuid.clone();
            (uuid, build_entry(entry, &relations))
        })
        .collect::<HashMap<_, _>>();

    Ok(uuids
        .iter()
        .filter_map(|uuid| entries_by_uuid.get(uuid).cloned())
        .collect())
}

pub fn get_random_entry(filters: RandomEntryFilters) -> Result<Option<Entry>> {
    get_random_entry_for_database(&db::resolve_database_path(), filters)
}

pub fn get_random_entry_for_database(
    db_path: &Path,
    filters: RandomEntryFilters,
) -> Result<Option<Entry>> {
    ensure_entry_ids_for_database(db_path)?;
    let entry_filters = EntryFilters {
        include_hidden: filters.include_hidden,
        tags: filters.tags,
        moods: filters.moods,
        limit: Some(1),
        ..EntryFilters::default()
    };
    let connection = open_entries_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    ensure_entries_table(&tables)?;

    let query = build_query(&entry_filters, &tables);
    let raw_entries = query_entries(&connection, &query, true)?;
    let relations = load_relations(&connection, &tables, &raw_entries)?;

    Ok(raw_entries
        .into_iter()
        .next()
        .map(|entry| build_entry(entry, &relations)))
}

pub fn create_entry(input: EntryCreate) -> Result<EntryMutationResponse> {
    let guarded = backup::with_database_backup("entry.create", move |db_path| {
        create_entry_inner(db_path, input)
    })?;
    Ok(EntryMutationResponse {
        entry: guarded.value,
        audit: guarded.audit,
    })
}

#[cfg(test)]
fn create_entry_for_database(db_path: &Path, input: EntryCreate) -> Result<EntryMutationResponse> {
    let guarded =
        backup::with_database_backup_for_database(db_path, "entry.create", move |path| {
            create_entry_inner(path, input)
        })?;
    Ok(EntryMutationResponse {
        entry: guarded.value,
        audit: guarded.audit,
    })
}

pub fn update_entry(identifier: String, input: EntryUpdate) -> Result<EntryMutationResponse> {
    let guarded = backup::with_database_backup("entry.update", move |db_path| {
        update_entry_inner(db_path, &identifier, input)
    })?;
    Ok(EntryMutationResponse {
        entry: guarded.value,
        audit: guarded.audit,
    })
}

#[cfg(test)]
fn update_entry_for_database(
    db_path: &Path,
    identifier: &str,
    input: EntryUpdate,
) -> Result<EntryMutationResponse> {
    let guarded =
        backup::with_database_backup_for_database(db_path, "entry.update", move |path| {
            update_entry_inner(path, identifier, input)
        })?;
    Ok(EntryMutationResponse {
        entry: guarded.value,
        audit: guarded.audit,
    })
}

pub fn delete_entry(identifier: String) -> Result<DeleteEntryResponse> {
    let guarded = backup::with_database_backup("entry.delete", move |db_path| {
        delete_entry_inner(db_path, &identifier)
    })?;
    Ok(DeleteEntryResponse {
        entry_id: guarded.value.entry_id,
        entry_uuid: guarded.value.entry_uuid,
        audit: guarded.audit,
    })
}

#[cfg(test)]
fn delete_entry_for_database(db_path: &Path, identifier: &str) -> Result<DeleteEntryResponse> {
    let guarded =
        backup::with_database_backup_for_database(db_path, "entry.delete", move |path| {
            delete_entry_inner(path, identifier)
        })?;
    Ok(DeleteEntryResponse {
        entry_id: guarded.value.entry_id,
        entry_uuid: guarded.value.entry_uuid,
        audit: guarded.audit,
    })
}

pub fn star_entry(identifier: String) -> Result<EntryMutationResponse> {
    set_entry_flag(identifier, "starred", true, "entry.star")
}

pub fn unstar_entry(identifier: String) -> Result<EntryMutationResponse> {
    set_entry_flag(identifier, "starred", false, "entry.unstar")
}

pub fn pin_entry(identifier: String) -> Result<EntryMutationResponse> {
    set_entry_flag(identifier, "pinned", true, "entry.pin")
}

pub fn unpin_entry(identifier: String) -> Result<EntryMutationResponse> {
    set_entry_flag(identifier, "pinned", false, "entry.unpin")
}

pub fn hide_entry(identifier: String) -> Result<EntryMutationResponse> {
    set_entry_flag(identifier, "hidden", true, "entry.hide")
}

pub fn unhide_entry(identifier: String) -> Result<EntryMutationResponse> {
    set_entry_flag(identifier, "hidden", false, "entry.unhide")
}

pub fn list_entry_history(identifier: String) -> Result<EntryHistoryResponse> {
    list_entry_history_for_database(&db::resolve_database_path(), &identifier)
}

pub fn list_entry_history_for_database(
    db_path: &Path,
    identifier: &str,
) -> Result<EntryHistoryResponse> {
    ensure_entry_ids_for_database(db_path)?;
    let connection = open_entries_connection(db_path)?;
    let (entry_id, _) = resolve_entry_identity(&connection, identifier)?;
    let current_snapshot = get_entry_snapshot(&connection, entry_id)?;
    let current = serde_json::to_value(&current_snapshot)?;

    if !table_exists(&connection, "history")? {
        return Ok(EntryHistoryResponse {
            entry_id,
            current,
            history: Vec::new(),
            count: 0,
        });
    }

    let mut statement = connection.prepare(
        "SELECT id, timestamp, operation_type, old_data
         FROM history
         WHERE entry_id = ?1
           AND operation_type IN ('EDIT_TEXT', 'EDIT_MOOD', 'EDIT_TAGS')
         ORDER BY id ASC",
    )?;
    let rows = statement.query_map([entry_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    let snapshots = rows
        .map(|row| {
            let (id, timestamp, operation_type, old_data_json) = row?;
            let old_data = old_data_json
                .and_then(|value| serde_json::from_str::<JsonValue>(&value).ok())
                .unwrap_or_else(|| json!({}));
            Ok((id, timestamp, operation_type, old_data))
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut history = Vec::new();
    for (index, (id, timestamp, operation_type, old_data)) in snapshots.iter().enumerate() {
        let next_snapshot = snapshots
            .get(index + 1)
            .map(|item| &item.3)
            .unwrap_or(&current);
        history.push(EntryHistoryItem {
            id: *id,
            timestamp: timestamp.clone(),
            operation_type: operation_type.clone(),
            changed_fields: changed_fields(old_data, next_snapshot),
            old_data: old_data.clone(),
        });
    }
    history.reverse();

    Ok(EntryHistoryResponse {
        entry_id,
        count: history.len(),
        current,
        history,
    })
}

fn open_entries_connection(db_path: &Path) -> Result<Connection> {
    let connection = db::open_read_only_connection(db_path)?;
    let schema = db::inspect_schema(&connection)?;
    if !schema.has_entries_table {
        return Err(anyhow!(
            "The active database does not contain an entries table."
        ));
    }

    Ok(connection)
}

fn create_entry_inner(db_path: &Path, input: EntryCreate) -> Result<Entry> {
    ensure_entry_ids_for_database(db_path)?;
    let text = normalize_required_text(&input.text)?;
    let content_format = normalize_content_format(input.content_format.as_deref())?;
    let text_plain = build_text_plain(&text);
    let created_at = normalized_created_at(input.when.as_deref());
    let updated_at = created_at.clone();
    let title = normalize_optional_string(input.title);
    let summary = normalize_optional_string(input.summary);
    let mood = normalize_optional_string(input.mood);
    let tags = normalize_tags(input.tags.as_deref());
    let starred = input.starred.unwrap_or(false);
    let pinned = input.pinned.unwrap_or(false);
    let continue_from_uuid = normalize_optional_string(input.continue_from_uuid);

    let mut connection = db::open_read_write_connection(db_path)?;
    let schema = db::inspect_schema(&connection)?;
    ensure_entries_table(&schema.detected_tables.into_iter().collect())?;

    let tx = connection.transaction()?;
    let uuid = generate_entry_uuid(&tx)?;
    let entry_id = next_entry_id(&tx)?;
    tx.execute(
        "INSERT INTO entries
            (id, uuid, created_at, updated_at, text, text_plain, content_format, title, summary, mood, starred, pinned, hidden)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)",
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
        ],
    )?;
    replace_entry_tags(&tx, entry_id, &tags)?;
    if let Some(parent_uuid) = continue_from_uuid.as_deref() {
        set_entry_continuation(&tx, &uuid, Some(parent_uuid))?;
    }
    refresh_fts_for_entry(&tx, entry_id, &text_plain)?;
    tx.commit()?;

    if let Err(error) = location::auto_capture_location(db_path, &uuid) {
        eprintln!("[Location] Auto-capture failed for {uuid}: {error}");
    }

    get_entry_for_database(db_path, &uuid)
}

fn update_entry_inner(db_path: &Path, identifier: &str, input: EntryUpdate) -> Result<Entry> {
    ensure_entry_ids_for_database(db_path)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let schema = db::inspect_schema(&connection)?;
    ensure_entries_table(&schema.detected_tables.into_iter().collect())?;

    let tx = connection.transaction()?;
    let (entry_id, uuid) = resolve_entry_identity(&tx, identifier)?;

    if input.text.is_some()
        || input.content_format.is_some()
        || input.title.is_present()
        || input.summary.is_present()
    {
        let old_snapshot = get_entry_snapshot(&tx, entry_id)?;
        record_history(&tx, "EDIT_TEXT", entry_id, &old_snapshot)?;
        let text = match input.text.as_deref() {
            Some(value) => normalize_required_text(value)?,
            None => old_snapshot.text.clone(),
        };
        let content_format = normalize_content_format(Some(
            input
                .content_format
                .as_deref()
                .unwrap_or(&old_snapshot.content_format),
        ))?;
        let text_plain = build_text_plain(&text);
        let title = normalize_optional_string(input.title.apply_to(old_snapshot.title.clone()));
        let summary = normalize_optional_string(input.summary.apply_to(old_snapshot.summary));
        tx.execute(
            "UPDATE entries
             SET text = ?1,
                 text_plain = ?2,
                 content_format = ?3,
                 title = ?4,
                 summary = ?5,
                 updated_at = ?6
             WHERE id = ?7",
            params![
                text,
                text_plain,
                content_format,
                title,
                summary,
                current_updated_at(),
                entry_id,
            ],
        )?;
        refresh_fts_for_entry(&tx, entry_id, &text_plain)?;
    }

    if input.mood.is_present() {
        let old_snapshot = get_entry_snapshot(&tx, entry_id)?;
        record_history(&tx, "EDIT_MOOD", entry_id, &old_snapshot)?;
        let mood = normalize_optional_string(input.mood.apply_to(old_snapshot.mood));
        tx.execute(
            "UPDATE entries SET mood = ?1, updated_at = ?2 WHERE id = ?3",
            params![mood, current_updated_at(), entry_id],
        )?;
    }

    if let Some(tags) = input.tags.as_deref() {
        let old_snapshot = get_entry_snapshot(&tx, entry_id)?;
        record_history(&tx, "EDIT_TAGS", entry_id, &old_snapshot)?;
        replace_entry_tags(&tx, entry_id, &normalize_tags(Some(tags)))?;
    }

    if let Some(starred) = input.starred {
        update_flag(&tx, entry_id, "starred", starred)?;
    }
    if let Some(pinned) = input.pinned {
        update_flag(&tx, entry_id, "pinned", pinned)?;
    }
    if let Some(hidden) = input.hidden {
        update_flag(&tx, entry_id, "hidden", hidden)?;
    }
    if let Some(parent_update) = input.continue_from_uuid.as_optional_value() {
        set_entry_continuation(&tx, &uuid, parent_update.as_deref())?;
    }

    tx.commit()?;
    get_entry_for_database(db_path, &uuid)
}

fn delete_entry_inner(db_path: &Path, identifier: &str) -> Result<DeletedEntry> {
    ensure_entry_ids_for_database(db_path)?;
    let connection = db::open_read_write_connection(db_path)?;
    let schema = db::inspect_schema(&connection)?;
    ensure_entries_table(&schema.detected_tables.into_iter().collect())?;

    connection.execute_batch("PRAGMA foreign_keys = OFF")?;
    connection.execute_batch("BEGIN IMMEDIATE")?;

    let result = (|| -> Result<DeletedEntry> {
        let (entry_id, entry_uuid) = resolve_entry_identity(&connection, identifier)?;
        let old_snapshot = get_entry_snapshot(&connection, entry_id)?;
        let affected_ids = load_affected_entry_ids(&connection, entry_id)?;
        let mut additional_data = serde_json::Map::new();
        if !affected_ids.is_empty() {
            additional_data.insert("affected_ids".to_string(), json!(affected_ids));
        }

        let deleted_at = current_history_timestamp();
        ensure_sync_tombstones(&connection)?;
        connection.execute(
            "INSERT OR IGNORE INTO sync_tombstones (uuid, deleted_at) VALUES (?1, ?2)",
            params![&entry_uuid, &deleted_at],
        )?;

        let continuation_rows = load_continuation_rows_for_entry(&connection, &entry_uuid)?;
        if !continuation_rows.is_empty() {
            additional_data.insert("continuations".to_string(), json!(continuation_rows));
        }

        let thread_title_rows = load_thread_text_rows_for_uuids(
            &connection,
            "entry_thread_titles",
            "title",
            &[entry_uuid.clone()],
        )?;
        if !thread_title_rows.is_empty() {
            additional_data.insert("thread_titles".to_string(), json!(thread_title_rows));
        }

        let thread_summary_rows = load_thread_text_rows_for_uuids(
            &connection,
            "entry_thread_summaries",
            "summary",
            &[entry_uuid.clone()],
        )?;
        if !thread_summary_rows.is_empty() {
            additional_data.insert("thread_summaries".to_string(), json!(thread_summary_rows));
        }

        let affected_surviving_roots = continuation_rows
            .iter()
            .filter(|row| row.parent_entry_uuid != entry_uuid)
            .map(|row| row.parent_entry_uuid.clone())
            .collect::<Vec<_>>();
        delete_entry_continuations_for_entry(&connection, &entry_uuid)?;
        let cleared_title_rows = clear_invalid_thread_text_rows(
            &connection,
            "entry_thread_titles",
            "title",
            "sync_entry_thread_title_tombstones",
            &affected_surviving_roots,
            &deleted_at,
        )?;
        if !cleared_title_rows.is_empty() {
            merge_thread_text_snapshots(&mut additional_data, "thread_titles", cleared_title_rows);
        }
        let cleared_summary_rows = clear_invalid_thread_text_rows(
            &connection,
            "entry_thread_summaries",
            "summary",
            "sync_entry_thread_summary_tombstones",
            &affected_surviving_roots,
            &deleted_at,
        )?;
        if !cleared_summary_rows.is_empty() {
            merge_thread_text_snapshots(
                &mut additional_data,
                "thread_summaries",
                cleared_summary_rows,
            );
        }
        delete_thread_text_for_root(&connection, "entry_thread_titles", &entry_uuid)?;
        delete_thread_text_for_root(&connection, "entry_thread_summaries", &entry_uuid)?;

        mark_entry_images_deleted(&connection, &entry_uuid, &deleted_at)?;
        mark_entry_location_deleted(&connection, &entry_uuid, &deleted_at)?;

        let additional_data = if additional_data.is_empty() {
            None
        } else {
            Some(JsonValue::Object(additional_data))
        };
        record_history_with_additional(
            &connection,
            "DELETE",
            entry_id,
            &old_snapshot,
            additional_data.as_ref(),
        )?;

        delete_if_table_exists(&connection, "entry_tags", "entry_id", entry_id)?;
        connection.execute("DELETE FROM entries WHERE id = ?1", [entry_id])?;
        connection.execute("UPDATE entries SET id = id - 1 WHERE id > ?1", [entry_id])?;
        if table_exists(&connection, "entry_tags")? {
            connection.execute(
                "UPDATE entry_tags SET entry_id = entry_id - 1 WHERE entry_id > ?1",
                [entry_id],
            )?;
        }
        update_sqlite_sequence(&connection)?;
        if let Err(error) = rebuild_entries_fts(&connection) {
            eprintln!("[Entries] Rebuilding entries_fts after delete failed: {error}");
        }

        Ok(DeletedEntry {
            entry_id,
            entry_uuid,
        })
    })();

    match result {
        Ok(deleted) => {
            connection.execute_batch("COMMIT")?;
            connection.execute_batch("PRAGMA foreign_keys = ON")?;
            Ok(deleted)
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            let _ = connection.execute_batch("PRAGMA foreign_keys = ON");
            Err(error)
        }
    }
}

fn set_entry_flag(
    identifier: String,
    column: &'static str,
    value: bool,
    operation: &'static str,
) -> Result<EntryMutationResponse> {
    let guarded = backup::with_database_backup(operation, move |db_path| {
        set_entry_flag_inner(db_path, &identifier, column, value)
    })?;
    Ok(EntryMutationResponse {
        entry: guarded.value,
        audit: guarded.audit,
    })
}

fn set_entry_flag_inner(
    db_path: &Path,
    identifier: &str,
    column: &'static str,
    value: bool,
) -> Result<Entry> {
    ensure_entry_ids_for_database(db_path)?;
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    let (entry_id, uuid) = resolve_entry_identity(&tx, identifier)?;
    update_flag(&tx, entry_id, column, value)?;
    tx.commit()?;
    get_entry_for_database(db_path, &uuid)
}

fn update_flag(
    connection: &Connection,
    entry_id: i64,
    column: &'static str,
    value: bool,
) -> Result<()> {
    let column = match column {
        "starred" => "starred",
        "pinned" => "pinned",
        "hidden" => "hidden",
        _ => return Err(anyhow!("unsupported entry flag: {column}")),
    };
    let sql = format!("UPDATE entries SET {column} = ?1, updated_at = ?2 WHERE id = ?3");
    connection.execute(
        &sql,
        params![bool_to_int(value), current_updated_at(), entry_id],
    )?;
    Ok(())
}

fn load_affected_entry_ids(connection: &Connection, deleted_entry_id: i64) -> Result<Vec<i64>> {
    let mut statement =
        connection.prepare("SELECT id FROM entries WHERE id > ?1 ORDER BY id ASC")?;
    let rows = statement.query_map([deleted_entry_id], |row| row.get::<_, i64>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn ensure_sync_tombstones(connection: &Connection) -> Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS sync_tombstones (
            uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn load_continuation_rows_for_entry(
    connection: &Connection,
    entry_uuid: &str,
) -> Result<Vec<ContinuationSnapshot>> {
    if !table_exists(connection, "entry_continuations")? {
        return Ok(Vec::new());
    }

    let columns = table_columns(connection, "entry_continuations")?;
    let updated_at_column = if columns.contains("updated_at") {
        "updated_at"
    } else {
        "NULL"
    };
    let mut statement = connection.prepare(&format!(
        "SELECT child_entry_uuid, parent_entry_uuid, {updated_at_column}
         FROM entry_continuations
         WHERE child_entry_uuid = ?1 OR parent_entry_uuid = ?1
         ORDER BY child_entry_uuid ASC"
    ))?;
    let rows = statement.query_map([entry_uuid], |row| {
        Ok(ContinuationSnapshot {
            child_entry_uuid: row.get(0)?,
            parent_entry_uuid: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn delete_entry_continuations_for_entry(connection: &Connection, entry_uuid: &str) -> Result<()> {
    if !table_exists(connection, "entry_continuations")? {
        return Ok(());
    }
    connection.execute(
        "DELETE FROM entry_continuations
         WHERE child_entry_uuid = ?1 OR parent_entry_uuid = ?1",
        [entry_uuid],
    )?;
    Ok(())
}

fn load_thread_text_rows_for_uuids(
    connection: &Connection,
    table: &str,
    value_column: &str,
    root_uuids: &[String],
) -> Result<Vec<JsonValue>> {
    let (table, value_column) = safe_thread_text_table(table, value_column)?;
    if root_uuids.is_empty() || !table_exists(connection, table)? {
        return Ok(Vec::new());
    }

    let placeholders = placeholders(root_uuids.len());
    let sql = format!(
        "SELECT thread_root_uuid, {value_column}, updated_at
         FROM {table}
         WHERE thread_root_uuid IN ({placeholders})
         ORDER BY thread_root_uuid ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params_from_iter(root_uuids.iter().cloned().map(Value::Text)),
        |row| {
            let root_uuid = row.get::<_, String>(0)?;
            let text = row.get::<_, String>(1)?;
            let updated_at = row.get::<_, Option<String>>(2)?;
            Ok(json!({
                "thread_root_uuid": root_uuid,
                value_column: text,
                "updated_at": updated_at,
            }))
        },
    )?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn clear_invalid_thread_text_rows(
    connection: &Connection,
    table: &str,
    value_column: &str,
    tombstone_table: &str,
    root_uuids: &[String],
    deleted_at: &str,
) -> Result<Vec<JsonValue>> {
    let (table, value_column) = safe_thread_text_table(table, value_column)?;
    let tombstone_table = safe_thread_tombstone_table(tombstone_table)?;
    if root_uuids.is_empty() || !table_exists(connection, table)? {
        return Ok(Vec::new());
    }
    ensure_thread_tombstone_table(connection, tombstone_table)?;

    let rows = load_thread_text_rows_for_uuids(connection, table, value_column, root_uuids)?;
    let mut cleared_rows = Vec::new();
    for row in rows {
        let Some(root_uuid) = row
            .get("thread_root_uuid")
            .and_then(|value| value.as_str())
            .map(str::to_string)
        else {
            continue;
        };
        if entry_can_have_thread_title(connection, &root_uuid)? {
            continue;
        }
        connection.execute(
            &format!("DELETE FROM {table} WHERE thread_root_uuid = ?1"),
            [root_uuid.as_str()],
        )?;
        record_thread_text_tombstone(connection, tombstone_table, &root_uuid, deleted_at)?;
        cleared_rows.push(row);
    }

    Ok(cleared_rows)
}

fn merge_thread_text_snapshots(
    additional_data: &mut serde_json::Map<String, JsonValue>,
    key: &str,
    snapshots: Vec<JsonValue>,
) {
    let mut values_by_root = additional_data
        .get(key)
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| {
            let root_uuid = value
                .get("thread_root_uuid")
                .and_then(|field| field.as_str())
                .map(str::to_string)?;
            Some((root_uuid, value))
        })
        .collect::<HashMap<_, _>>();

    for snapshot in snapshots {
        if let Some(root_uuid) = snapshot
            .get("thread_root_uuid")
            .and_then(|field| field.as_str())
            .map(str::to_string)
        {
            values_by_root.insert(root_uuid, snapshot);
        }
    }

    additional_data.insert(
        key.to_string(),
        JsonValue::Array(values_by_root.into_values().collect()),
    );
}

fn delete_thread_text_for_root(
    connection: &Connection,
    table: &str,
    root_uuid: &str,
) -> Result<()> {
    let table = match table {
        "entry_thread_titles" => "entry_thread_titles",
        "entry_thread_summaries" => "entry_thread_summaries",
        _ => return Err(anyhow!("Unsupported thread metadata table: {table}")),
    };
    if table_exists(connection, table)? {
        connection.execute(
            &format!("DELETE FROM {table} WHERE thread_root_uuid = ?1"),
            [root_uuid],
        )?;
    }
    Ok(())
}

fn entry_can_have_thread_title(connection: &Connection, thread_root_uuid: &str) -> Result<bool> {
    if !table_exists(connection, "entry_continuations")? {
        return Ok(false);
    }
    let entry_exists = connection
        .query_row(
            "SELECT 1 FROM entries WHERE uuid = ?1 LIMIT 1",
            [thread_root_uuid],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !entry_exists {
        return Ok(false);
    }
    let has_parent = connection
        .query_row(
            "SELECT 1 FROM entry_continuations WHERE child_entry_uuid = ?1 LIMIT 1",
            [thread_root_uuid],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if has_parent {
        return Ok(false);
    }
    let child_count = connection.query_row(
        "SELECT COUNT(*) FROM entry_continuations WHERE parent_entry_uuid = ?1",
        [thread_root_uuid],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(child_count > 0)
}

fn ensure_thread_tombstone_table(connection: &Connection, table: &str) -> Result<()> {
    match table {
        "sync_entry_thread_title_tombstones" => connection.execute(
            "CREATE TABLE IF NOT EXISTS sync_entry_thread_title_tombstones (
                thread_root_uuid TEXT PRIMARY KEY,
                deleted_at TEXT NOT NULL
            )",
            [],
        )?,
        "sync_entry_thread_summary_tombstones" => connection.execute(
            "CREATE TABLE IF NOT EXISTS sync_entry_thread_summary_tombstones (
                thread_root_uuid TEXT PRIMARY KEY,
                deleted_at TEXT NOT NULL
            )",
            [],
        )?,
        _ => return Err(anyhow!("Unsupported thread tombstone table: {table}")),
    };
    Ok(())
}

fn record_thread_text_tombstone(
    connection: &Connection,
    table: &str,
    root_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    let table = safe_thread_tombstone_table(table)?;
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

fn mark_entry_images_deleted(
    connection: &Connection,
    entry_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    if !table_exists(connection, "plugin_entry_media")? {
        return Ok(());
    }

    let mut media_ids = Vec::new();
    if table_exists(connection, "plugin_media_assets")? {
        let mut statement = connection.prepare(
            "SELECT em.media_id, ma.hash, em.position, em.caption, em.alt_text
             FROM plugin_entry_media em
             JOIN plugin_media_assets ma ON ma.id = em.media_id
             WHERE em.entry_uuid = ?1",
        )?;
        let rows = statement.query_map([entry_uuid], |row| {
            Ok(ImageAttachmentForDelete {
                media_id: row.get(0)?,
                hash: row.get(1)?,
                position: row.get(2)?,
                caption: row.get(3)?,
                alt_text: row.get(4)?,
            })
        })?;
        let attachments = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        if !attachments.is_empty() {
            connection.execute(
                "CREATE TABLE IF NOT EXISTS sync_image_tombstones (
                    entry_uuid TEXT NOT NULL,
                    asset_hash TEXT NOT NULL,
                    position INTEGER NOT NULL DEFAULT 0,
                    caption TEXT,
                    alt_text TEXT,
                    deleted_at TEXT NOT NULL,
                    PRIMARY KEY (entry_uuid, asset_hash, position, caption, alt_text)
                )",
                [],
            )?;
        }
        for attachment in attachments {
            let media_id = attachment.media_id;
            let hash = attachment.hash;
            let position = attachment.position;
            let caption = attachment.caption.unwrap_or_default();
            let alt_text = attachment.alt_text.unwrap_or_default();
            connection.execute(
                "INSERT OR IGNORE INTO sync_image_tombstones
                    (entry_uuid, asset_hash, position, caption, alt_text, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![entry_uuid, hash, position, caption, alt_text, deleted_at,],
            )?;
            media_ids.push(media_id);
        }
    }

    connection.execute(
        "DELETE FROM plugin_entry_media WHERE entry_uuid = ?1",
        [entry_uuid],
    )?;
    if table_exists(connection, "plugin_media_assets")? {
        for media_id in media_ids {
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
        }
    }
    Ok(())
}

fn mark_entry_location_deleted(
    connection: &Connection,
    entry_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    if !table_exists(connection, "plugin_entry_locations")? {
        return Ok(());
    }
    let had_location = connection
        .query_row(
            "SELECT 1 FROM plugin_entry_locations WHERE entry_uuid = ?1 LIMIT 1",
            [entry_uuid],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if had_location {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS sync_location_tombstones (
                entry_uuid TEXT NOT NULL PRIMARY KEY,
                deleted_at TEXT NOT NULL
            )",
            [],
        )?;
        connection.execute(
            "INSERT OR REPLACE INTO sync_location_tombstones (entry_uuid, deleted_at)
             VALUES (?1, ?2)",
            params![entry_uuid, deleted_at],
        )?;
    }
    connection.execute(
        "DELETE FROM plugin_entry_locations WHERE entry_uuid = ?1",
        [entry_uuid],
    )?;
    Ok(())
}

fn delete_if_table_exists(
    connection: &Connection,
    table: &str,
    column: &str,
    value: i64,
) -> Result<()> {
    let (table, column) = match (table, column) {
        ("entry_tags", "entry_id") => ("entry_tags", "entry_id"),
        _ => return Err(anyhow!("Unsupported delete target: {table}.{column}")),
    };
    if table_exists(connection, table)? {
        connection.execute(&format!("DELETE FROM {table} WHERE {column} = ?1"), [value])?;
    }
    Ok(())
}

fn update_sqlite_sequence(connection: &Connection) -> Result<()> {
    if table_exists(connection, "sqlite_sequence")? {
        let max_id =
            connection.query_row("SELECT COALESCE(MAX(id), 0) FROM entries", [], |row| {
                row.get::<_, i64>(0)
            })?;
        connection.execute(
            "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'entries'",
            [max_id],
        )?;
    }
    Ok(())
}

pub(crate) fn ensure_entry_ids_for_database(db_path: &Path) -> Result<()> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    ensure_entries_table(&tables)?;
    let needs_repair = entry_ids_need_repair(&connection)?;
    drop(connection);

    if !needs_repair {
        return Ok(());
    }

    backup::with_database_backup_for_database(db_path, "entry.ids.repair", |path| {
        let connection = db::open_read_write_connection(path)?;
        repair_entry_ids(&connection)
    })?;
    Ok(())
}

fn entry_ids_need_repair(connection: &Connection) -> Result<bool> {
    let info = entry_id_column_info(connection)?;
    if !info.exists {
        return Ok(true);
    }
    if info.primary_key {
        return Ok(false);
    }

    let missing_count = connection.query_row(
        "SELECT COUNT(*) FROM entries WHERE id IS NULL OR CAST(id AS INTEGER) <= 0",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    if missing_count > 0 {
        return Ok(true);
    }

    let duplicate_count = connection.query_row(
        "SELECT COUNT(*)
         FROM (
            SELECT id
            FROM entries
            WHERE id IS NOT NULL AND CAST(id AS INTEGER) > 0
            GROUP BY id
            HAVING COUNT(*) > 1
         )",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(duplicate_count > 0)
}

fn repair_entry_ids(connection: &Connection) -> Result<()> {
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> Result<()> {
        let info = entry_id_column_info(connection)?;
        let rows = planned_entry_id_repair_rows(connection, info.exists)?;
        let duplicate_id = duplicate_entry_id(connection)?;
        if let Some(id) = duplicate_id {
            return Err(anyhow!(
                "Cannot repair entry numbers because entries.id value {id} is duplicated."
            ));
        }

        if !info.exists {
            connection.execute("ALTER TABLE entries ADD COLUMN id INTEGER", [])?;
        }

        let changed_rows = rows
            .iter()
            .filter(|row| row.current_id.unwrap_or(0) != row.repaired_id)
            .cloned()
            .collect::<Vec<_>>();

        if changed_rows.is_empty() {
            return Ok(());
        }

        update_entry_id_references(connection, &changed_rows)?;
        for row in &changed_rows {
            connection.execute(
                "UPDATE entries SET id = ?1 WHERE rowid = ?2",
                params![row.repaired_id, row.rowid],
            )?;
        }
        rebuild_entries_fts(connection)?;
        update_sqlite_sequence(connection)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn planned_entry_id_repair_rows(
    connection: &Connection,
    has_id_column: bool,
) -> Result<Vec<EntryIdRepairRow>> {
    let sql = if has_id_column {
        "SELECT rowid, id FROM entries ORDER BY datetime(created_at) ASC, rowid ASC"
    } else {
        "SELECT rowid, NULL AS id FROM entries ORDER BY datetime(created_at) ASC, rowid ASC"
    };
    let mut statement = connection.prepare(sql)?;
    let source_rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut next_id = source_rows
        .iter()
        .filter_map(|(_, id)| (*id).filter(|value| *value > 0))
        .max()
        .unwrap_or(0)
        + 1;
    let mut rows = Vec::with_capacity(source_rows.len());
    for (rowid, raw_id) in source_rows {
        let current_id = raw_id.filter(|id| *id > 0);
        let repaired_id = match current_id {
            Some(id) => id,
            None => {
                let id = next_id;
                next_id += 1;
                id
            }
        };
        let reference_id = if has_id_column { raw_id } else { Some(rowid) };
        rows.push(EntryIdRepairRow {
            rowid,
            current_id,
            reference_id,
            repaired_id,
        });
    }
    Ok(rows)
}

fn duplicate_entry_id(connection: &Connection) -> Result<Option<i64>> {
    if !entry_id_column_info(connection)?.exists {
        return Ok(None);
    }

    connection
        .query_row(
            "SELECT id
             FROM entries
             WHERE id IS NOT NULL AND CAST(id AS INTEGER) > 0
             GROUP BY id
             HAVING COUNT(*) > 1
             LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(Into::into)
}

fn update_entry_id_references(
    connection: &Connection,
    changed_rows: &[EntryIdRepairRow],
) -> Result<()> {
    update_entry_id_reference_table(connection, "entry_tags", "entry_id", changed_rows)?;
    update_entry_id_reference_table(connection, "history", "entry_id", changed_rows)?;
    update_entry_id_reference_table(connection, "embeddings", "entry_id", changed_rows)?;
    Ok(())
}

fn update_entry_id_reference_table(
    connection: &Connection,
    table: &str,
    column: &str,
    changed_rows: &[EntryIdRepairRow],
) -> Result<()> {
    if changed_rows.is_empty() || !table_exists(connection, table)? {
        return Ok(());
    }
    if !table_columns(connection, table)?.contains(column) {
        return Ok(());
    }

    let table = match table {
        "entry_tags" => "entry_tags",
        "history" => "history",
        "embeddings" => "embeddings",
        _ => return Err(anyhow!("Unsupported entry ID repair table: {table}")),
    };
    let column = match column {
        "entry_id" => "entry_id",
        _ => return Err(anyhow!("Unsupported entry ID repair column: {column}")),
    };

    for row in changed_rows {
        let Some(previous_reference) = row.reference_id else {
            continue;
        };
        if previous_reference == row.repaired_id {
            continue;
        }
        connection.execute(
            &format!("UPDATE {table} SET {column} = ?1 WHERE {column} = ?2"),
            params![-row.repaired_id, previous_reference],
        )?;
    }
    connection.execute(
        &format!("UPDATE {table} SET {column} = -{column} WHERE {column} < 0"),
        [],
    )?;
    Ok(())
}

fn entry_id_column_info(connection: &Connection) -> Result<EntryIdColumnInfo> {
    let mut statement = connection.prepare("PRAGMA table_info(entries)")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
    })?;

    for row in rows {
        let (name, primary_key) = row?;
        if name.eq_ignore_ascii_case("id") {
            return Ok(EntryIdColumnInfo {
                exists: true,
                primary_key: primary_key > 0,
            });
        }
    }

    Ok(EntryIdColumnInfo {
        exists: false,
        primary_key: false,
    })
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
    connection.execute(
        "INSERT INTO entries_fts(rowid, text)
         SELECT id, COALESCE(NULLIF(text_plain, ''), text, '') FROM entries",
        [],
    )?;
    Ok(())
}

fn safe_thread_text_table<'a>(table: &'a str, value_column: &'a str) -> Result<(&'a str, &'a str)> {
    match (table, value_column) {
        ("entry_thread_titles", "title") => Ok(("entry_thread_titles", "title")),
        ("entry_thread_summaries", "summary") => Ok(("entry_thread_summaries", "summary")),
        _ => Err(anyhow!(
            "Unsupported thread metadata table: {table}.{value_column}"
        )),
    }
}

fn safe_thread_tombstone_table(table: &str) -> Result<&str> {
    match table {
        "sync_entry_thread_title_tombstones" => Ok("sync_entry_thread_title_tombstones"),
        "sync_entry_thread_summary_tombstones" => Ok("sync_entry_thread_summary_tombstones"),
        _ => Err(anyhow!("Unsupported thread tombstone table: {table}")),
    }
}

fn detected_tables(connection: &Connection) -> Result<HashSet<String>> {
    Ok(db::inspect_schema(connection)?
        .detected_tables
        .into_iter()
        .collect())
}

fn ensure_entries_table(tables: &HashSet<String>) -> Result<()> {
    if tables.contains("entries") {
        Ok(())
    } else {
        Err(anyhow!(
            "The active database does not contain an entries table."
        ))
    }
}

fn build_query(filters: &EntryFilters, tables: &HashSet<String>) -> QueryParts {
    let mut conditions = Vec::new();
    let mut params = Vec::new();

    if let Some(text) = normalized_string(filters.text.as_deref()) {
        let pattern = like_pattern(&text);
        conditions.push(
            "(lower(COALESCE(NULLIF(e.text_plain, ''), e.text, '')) LIKE ? \
             OR lower(COALESCE(e.title, '')) LIKE ? \
             OR lower(COALESCE(e.summary, '')) LIKE ?)"
                .to_string(),
        );
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern.clone()));
        params.push(Value::Text(pattern));
    }

    if let Some(since) = normalized_string(filters.since.as_deref()) {
        conditions.push("datetime(e.created_at) >= datetime(?)".to_string());
        params.push(Value::Text(since));
    }

    if let Some(until) = normalized_string(filters.until.as_deref()) {
        conditions.push("datetime(e.created_at) <= datetime(?)".to_string());
        params.push(Value::Text(until));
    }

    if let Some(starred) = filters.starred {
        conditions.push("COALESCE(e.starred, 0) = ?".to_string());
        params.push(Value::Integer(bool_to_int(starred)));
    }

    if let Some(pinned) = filters.pinned {
        conditions.push("COALESCE(e.pinned, 0) = ?".to_string());
        params.push(Value::Integer(bool_to_int(pinned)));
    }

    if let Some(hidden) = filters.hidden {
        conditions.push("COALESCE(e.hidden, 0) = ?".to_string());
        params.push(Value::Integer(bool_to_int(hidden)));
    } else if filters.include_hidden != Some(true) {
        conditions.push("COALESCE(e.hidden, 0) = 0".to_string());
    }

    let tags_available = tables.contains("tags") && tables.contains("entry_tags");
    for tag in normalized_vec(filters.tags.as_deref()) {
        if tags_available {
            conditions.push(
                "EXISTS (
                    SELECT 1
                    FROM entry_tags et
                    JOIN tags t ON t.id = et.tag_id
                    WHERE et.entry_id = e.id AND lower(t.name) = ?
                )"
                .to_string(),
            );
            params.push(Value::Text(tag.to_lowercase()));
        } else {
            conditions.push("1 = 0".to_string());
        }
    }

    for tag in normalized_vec(filters.exclude_tags.as_deref()) {
        if tags_available {
            conditions.push(
                "NOT EXISTS (
                    SELECT 1
                    FROM entry_tags et
                    JOIN tags t ON t.id = et.tag_id
                    WHERE et.entry_id = e.id AND lower(t.name) = ?
                )"
                .to_string(),
            );
            params.push(Value::Text(tag.to_lowercase()));
        }
    }

    add_in_filter(
        &mut conditions,
        &mut params,
        "lower(COALESCE(e.mood, ''))",
        normalized_vec(filters.moods.as_deref()),
        false,
    );
    add_in_filter(
        &mut conditions,
        &mut params,
        "lower(COALESCE(e.mood, ''))",
        normalized_vec(filters.exclude_moods.as_deref()),
        true,
    );

    if let Some(location) = normalized_string(filters.location.as_deref()) {
        if tables.contains("plugin_entry_locations") {
            conditions.push(
                "EXISTS (
                    SELECT 1
                    FROM plugin_entry_locations pel
                    WHERE pel.entry_uuid = e.uuid
                      AND lower(COALESCE(pel.place_name, '')) LIKE ?
                )"
                .to_string(),
            );
            params.push(Value::Text(like_pattern(&location)));
        } else {
            conditions.push("1 = 0".to_string());
        }
    }

    if let Some(has_images) = filters.has_images {
        if tables.contains("plugin_entry_media") {
            let prefix = if has_images { "" } else { "NOT " };
            conditions.push(format!(
                "{prefix}EXISTS (
                    SELECT 1
                    FROM plugin_entry_media pem
                    WHERE pem.entry_uuid = e.uuid
                )"
            ));
        } else if has_images {
            conditions.push("1 = 0".to_string());
        }
    }

    let limit = filters.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = filters.offset.unwrap_or(0).max(0);
    let sort = filters.sort.clone().unwrap_or(EntrySort::Desc);
    let where_sql = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    QueryParts {
        where_sql,
        params,
        limit,
        offset,
        sort,
    }
}

fn add_in_filter(
    conditions: &mut Vec<String>,
    params: &mut Vec<Value>,
    expression: &str,
    values: Vec<String>,
    negated: bool,
) {
    if values.is_empty() {
        return;
    }

    let placeholders = std::iter::repeat("?")
        .take(values.len())
        .collect::<Vec<_>>()
        .join(", ");
    let operator = if negated { "NOT IN" } else { "IN" };
    conditions.push(format!("{expression} {operator} ({placeholders})"));
    params.extend(
        values
            .into_iter()
            .map(|value| Value::Text(value.to_lowercase())),
    );
}

fn count_entries(connection: &Connection, query: &QueryParts) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM entries e{}", query.where_sql);
    connection
        .query_row(&sql, params_from_iter(query.params.clone()), |row| {
            row.get::<_, i64>(0)
        })
        .context("failed to count entries")
}

fn query_entries(
    connection: &Connection,
    query: &QueryParts,
    random_order: bool,
) -> Result<Vec<RawEntry>> {
    let order_by = if random_order {
        "ORDER BY RANDOM()".to_string()
    } else {
        match query.sort {
            EntrySort::Asc => "ORDER BY datetime(e.created_at) ASC, e.id ASC".to_string(),
            EntrySort::Desc => "ORDER BY datetime(e.created_at) DESC, e.id DESC".to_string(),
        }
    };
    let sql = format!(
        "{}{} {order_by} LIMIT ? OFFSET ?",
        entry_select_sql(),
        query.where_sql
    );
    let mut params = query.params.clone();
    params.push(Value::Integer(query.limit));
    params.push(Value::Integer(query.offset));

    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(params), raw_entry_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list entries")
}

fn entry_select_sql() -> &'static str {
    "SELECT
        e.id,
        COALESCE(NULLIF(e.uuid, ''), 'entry_' || e.id) AS uuid,
        e.created_at,
        e.updated_at,
        e.text,
        COALESCE(NULLIF(e.text_plain, ''), e.text) AS text_plain,
        COALESCE(NULLIF(e.content_format, ''), 'plain') AS content_format,
        e.title,
        e.summary,
        e.mood,
        COALESCE(e.starred, 0) AS starred,
        COALESCE(e.pinned, 0) AS pinned,
        COALESCE(e.hidden, 0) AS hidden
     FROM entries e"
}

fn raw_entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawEntry> {
    Ok(RawEntry {
        id: row.get(0)?,
        uuid: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        text: row.get(4)?,
        text_plain: row.get(5)?,
        content_format: row.get(6)?,
        title: row.get(7)?,
        summary: row.get(8)?,
        mood: normalize_optional_string(row.get::<_, Option<String>>(9)?),
        starred: row.get::<_, i64>(10)? != 0,
        pinned: row.get::<_, i64>(11)? != 0,
        hidden: row.get::<_, i64>(12)? != 0,
    })
}

fn load_relations(
    connection: &Connection,
    tables: &HashSet<String>,
    entries: &[RawEntry],
) -> Result<EntryRelations> {
    if entries.is_empty() {
        return Ok(EntryRelations::default());
    }

    Ok(EntryRelations {
        tags_by_entry_id: load_tags(connection, tables, entries)?,
        locations_by_uuid: load_locations(connection, tables, entries)?,
        attachment_counts_by_uuid: load_attachment_counts(connection, tables, entries)?,
        thread_context: load_thread_context(connection, tables)?,
    })
}

fn load_tags(
    connection: &Connection,
    tables: &HashSet<String>,
    entries: &[RawEntry],
) -> Result<HashMap<i64, Vec<TagInfo>>> {
    if !tables.contains("tags") || !tables.contains("entry_tags") {
        return Ok(HashMap::new());
    }

    let ids = entries.iter().map(|entry| entry.id).collect::<Vec<_>>();
    let placeholders = placeholders(ids.len());
    let sql = format!(
        "SELECT et.entry_id, t.id, t.name
         FROM entry_tags et
         JOIN tags t ON t.id = et.tag_id
         WHERE et.entry_id IN ({placeholders})
         ORDER BY lower(t.name)"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params_from_iter(ids.into_iter().map(Value::Integer)),
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                TagInfo {
                    id: row.get(1)?,
                    name: row.get(2)?,
                },
            ))
        },
    )?;

    let mut tags_by_entry_id: HashMap<i64, Vec<TagInfo>> = HashMap::new();
    for row in rows {
        let (entry_id, tag) = row?;
        tags_by_entry_id.entry(entry_id).or_default().push(tag);
    }

    Ok(tags_by_entry_id)
}

fn load_locations(
    connection: &Connection,
    tables: &HashSet<String>,
    entries: &[RawEntry],
) -> Result<HashMap<String, LocationInfo>> {
    if !tables.contains("plugin_entry_locations") {
        return Ok(HashMap::new());
    }

    let columns = table_columns(connection, "plugin_entry_locations")?;
    let source_column = optional_location_column(&columns, "source");
    let weather_condition_column = optional_location_column(&columns, "weather_condition");
    let weather_temp_c_column = optional_location_column(&columns, "weather_temp_c");
    let weather_temp_f_column = optional_location_column(&columns, "weather_temp_f");
    let weather_icon_column = optional_location_column(&columns, "weather_icon");
    let weather_humidity_column = optional_location_column(&columns, "weather_humidity");
    let weather_wind_column = optional_location_column(&columns, "weather_wind_kph");
    let weather_fetched_column = optional_location_column(&columns, "weather_fetched_at");
    let uuids = entries
        .iter()
        .map(|entry| entry.uuid.clone())
        .collect::<Vec<_>>();
    let placeholders = placeholders(uuids.len());
    let sql = format!(
        "SELECT entry_uuid, latitude, longitude, place_name, {source_column},
                {weather_condition_column}, {weather_temp_c_column}, {weather_temp_f_column},
                {weather_icon_column}, {weather_humidity_column},
                {weather_wind_column}, {weather_fetched_column}
         FROM plugin_entry_locations
         WHERE entry_uuid IN ({placeholders})"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params_from_iter(uuids.into_iter().map(Value::Text)),
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                LocationInfo {
                    latitude: row.get(1)?,
                    longitude: row.get(2)?,
                    place_name: row.get(3)?,
                    source: row.get(4)?,
                    weather_condition: row.get(5)?,
                    weather_temp_c: row.get(6)?,
                    weather_temp_f: row.get(7)?,
                    weather_icon: row.get(8)?,
                    weather_humidity: row.get(9)?,
                    weather_wind_kph: row.get(10)?,
                    weather_fetched_at: row.get(11)?,
                },
            ))
        },
    )?;

    let mut locations_by_uuid = HashMap::new();
    for row in rows {
        let (uuid, location) = row?;
        locations_by_uuid.insert(uuid, location);
    }

    Ok(locations_by_uuid)
}

fn optional_location_column(columns: &HashSet<String>, column_name: &str) -> String {
    if columns.contains(column_name) {
        column_name.to_string()
    } else {
        format!("NULL AS {column_name}")
    }
}

fn load_attachment_counts(
    connection: &Connection,
    tables: &HashSet<String>,
    entries: &[RawEntry],
) -> Result<HashMap<String, i64>> {
    if !tables.contains("plugin_entry_media") {
        return Ok(HashMap::new());
    }

    let uuids = entries
        .iter()
        .map(|entry| entry.uuid.clone())
        .collect::<Vec<_>>();
    let placeholders = placeholders(uuids.len());
    let sql = format!(
        "SELECT entry_uuid, COUNT(*)
         FROM plugin_entry_media
         WHERE entry_uuid IN ({placeholders})
         GROUP BY entry_uuid"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params_from_iter(uuids.into_iter().map(Value::Text)),
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;

    let mut counts_by_uuid = HashMap::new();
    for row in rows {
        let (uuid, count) = row?;
        counts_by_uuid.insert(uuid, count);
    }

    Ok(counts_by_uuid)
}

fn load_thread_context(connection: &Connection, tables: &HashSet<String>) -> Result<ThreadContext> {
    if !tables.contains("entry_continuations") {
        return Ok(ThreadContext::default());
    }

    let mut context = ThreadContext::default();
    let mut statement = connection
        .prepare("SELECT child_entry_uuid, parent_entry_uuid FROM entry_continuations")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (child, parent) = row?;
        context.nodes.insert(child.clone());
        context.nodes.insert(parent.clone());
        context.parent_by_child.insert(child, parent);
    }

    if tables.contains("entry_thread_titles") {
        load_thread_text_map(
            connection,
            "SELECT thread_root_uuid, title FROM entry_thread_titles",
            &mut context.title_by_root,
        )?;
    }

    if tables.contains("entry_thread_summaries") {
        load_thread_text_map(
            connection,
            "SELECT thread_root_uuid, summary FROM entry_thread_summaries",
            &mut context.summary_by_root,
        )?;
    }

    context.finalize_counts();
    Ok(context)
}

fn load_thread_text_map(
    connection: &Connection,
    sql: &str,
    target: &mut HashMap<String, String>,
) -> Result<()> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (root_uuid, value) = row?;
        target.insert(root_uuid, value);
    }

    Ok(())
}

fn build_entry(raw: RawEntry, relations: &EntryRelations) -> Entry {
    let mood = raw.mood.clone();
    Entry {
        tags: relations
            .tags_by_entry_id
            .get(&raw.id)
            .cloned()
            .unwrap_or_default(),
        location: relations.locations_by_uuid.get(&raw.uuid).cloned(),
        thread: relations.thread_context.info_for(&raw.uuid),
        attachment_count: relations
            .attachment_counts_by_uuid
            .get(&raw.uuid)
            .copied()
            .unwrap_or(0),
        mood_info: MoodInfo {
            label: mood.as_deref().map(mood_label),
            name: mood.clone(),
        },
        id: raw.id,
        uuid: raw.uuid,
        created_at: raw.created_at,
        updated_at: raw.updated_at,
        text: raw.text,
        text_plain: raw.text_plain,
        content_format: raw.content_format,
        title: raw.title,
        summary: raw.summary,
        mood,
        starred: raw.starred,
        pinned: raw.pinned,
        hidden: raw.hidden,
    }
}

fn resolve_entry_identity(connection: &Connection, identifier: &str) -> Result<(i64, String)> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Entry identifier is required."));
    }

    connection
        .query_row(
            "SELECT id, COALESCE(NULLIF(uuid, ''), 'entry_' || id) AS uuid
             FROM entries
             WHERE uuid = ?1 OR CAST(id AS TEXT) = ?1
             LIMIT 1",
            [identifier],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .with_context(|| format!("entry not found: {identifier}"))
}

fn get_entry_snapshot(connection: &Connection, entry_id: i64) -> Result<EntrySnapshot> {
    let mut statement = connection.prepare(&format!("{} WHERE e.id = ?1", entry_select_sql()))?;
    let raw_entry = statement
        .query_row([entry_id], raw_entry_from_row)
        .with_context(|| format!("entry not found: {entry_id}"))?;
    let tags = load_tag_names(connection, entry_id)?;

    Ok(EntrySnapshot {
        id: raw_entry.id,
        uuid: raw_entry.uuid,
        created_at: raw_entry.created_at,
        updated_at: raw_entry.updated_at,
        text: raw_entry.text,
        text_plain: raw_entry.text_plain,
        content_format: raw_entry.content_format,
        title: raw_entry.title,
        summary: raw_entry.summary,
        mood: raw_entry.mood,
        tags,
        starred: raw_entry.starred,
        pinned: raw_entry.pinned,
        hidden: raw_entry.hidden,
    })
}

fn load_tag_names(connection: &Connection, entry_id: i64) -> Result<Vec<String>> {
    if !table_exists(connection, "tags")? || !table_exists(connection, "entry_tags")? {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        "SELECT t.name
         FROM entry_tags et
         JOIN tags t ON t.id = et.tag_id
         WHERE et.entry_id = ?1
         ORDER BY lower(t.name)",
    )?;
    let rows = statement.query_map([entry_id], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn record_history(
    connection: &Connection,
    operation_type: &str,
    entry_id: i64,
    old_snapshot: &EntrySnapshot,
) -> Result<()> {
    record_history_with_additional(connection, operation_type, entry_id, old_snapshot, None)
}

fn record_history_with_additional(
    connection: &Connection,
    operation_type: &str,
    entry_id: i64,
    old_snapshot: &EntrySnapshot,
    additional_data: Option<&JsonValue>,
) -> Result<()> {
    if !table_exists(connection, "history")? {
        return Ok(());
    }

    connection.execute("DELETE FROM history WHERE COALESCE(undone, 0) = 1", [])?;
    connection.execute(
        "INSERT INTO history (timestamp, operation_type, entry_id, old_data, additional_data, undone, redo_data)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, NULL)",
        params![
            current_history_timestamp(),
            operation_type,
            entry_id,
            serde_json::to_string(old_snapshot)?,
            additional_data
                .map(serde_json::to_string)
                .transpose()?,
        ],
    )?;
    Ok(())
}

fn replace_entry_tags(connection: &Connection, entry_id: i64, tags: &[String]) -> Result<()> {
    if tags.is_empty()
        && (!table_exists(connection, "tags")? || !table_exists(connection, "entry_tags")?)
    {
        return Ok(());
    }
    if !table_exists(connection, "tags")? || !table_exists(connection, "entry_tags")? {
        return Err(anyhow!(
            "The active database does not contain tags and entry_tags tables."
        ));
    }

    connection.execute("DELETE FROM entry_tags WHERE entry_id = ?1", [entry_id])?;
    for tag in tags {
        let tag_id = connection
            .query_row(
                "SELECT id FROM tags WHERE lower(name) = lower(?1) LIMIT 1",
                [tag],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let tag_id = match tag_id {
            Some(id) => id,
            None => {
                connection.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])?;
                connection.query_row("SELECT id FROM tags WHERE name = ?1", [tag], |row| {
                    row.get::<_, i64>(0)
                })?
            }
        };
        connection.execute(
            "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
            params![entry_id, tag_id],
        )?;
    }
    connection.execute(
        "UPDATE entries SET updated_at = ?1 WHERE id = ?2",
        params![current_updated_at(), entry_id],
    )?;
    Ok(())
}

fn set_entry_continuation(
    connection: &Connection,
    child_uuid: &str,
    parent_identifier: Option<&str>,
) -> Result<()> {
    if !table_exists(connection, "entry_continuations")? {
        if parent_identifier.is_none() {
            return Ok(());
        }
        return Err(anyhow!(
            "The active database does not contain an entry_continuations table."
        ));
    }

    let child_uuid = child_uuid.trim();
    if child_uuid.is_empty() {
        return Err(anyhow!("Continuation child UUID is required."));
    }

    let parent_uuid = match parent_identifier.and_then(|value| normalized_string(Some(value))) {
        Some(value) => resolve_entry_identity(connection, &value)?.1,
        None => {
            connection.execute(
                "DELETE FROM entry_continuations WHERE child_entry_uuid = ?1",
                [child_uuid],
            )?;
            return Ok(());
        }
    };

    if parent_uuid == child_uuid {
        return Err(anyhow!("An entry cannot continue itself."));
    }
    if continuation_would_cycle(connection, child_uuid, &parent_uuid)? {
        return Err(anyhow!("Continuation would create a thread cycle."));
    }

    connection.execute(
        "INSERT INTO entry_continuations (child_entry_uuid, parent_entry_uuid, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(child_entry_uuid)
         DO UPDATE SET parent_entry_uuid = excluded.parent_entry_uuid,
                       updated_at = excluded.updated_at",
        params![child_uuid, parent_uuid, current_history_timestamp()],
    )?;
    Ok(())
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
                "SELECT parent_entry_uuid
                 FROM entry_continuations
                 WHERE child_entry_uuid = ?1",
                [uuid],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
    }

    Ok(false)
}

fn refresh_fts_for_entry(connection: &Connection, entry_id: i64, text_plain: &str) -> Result<()> {
    if !table_exists(connection, "entries_fts")? {
        return Ok(());
    }

    connection.execute(
        "INSERT OR REPLACE INTO entries_fts(rowid, text) VALUES (?1, ?2)",
        params![entry_id, text_plain],
    )?;
    Ok(())
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1
             FROM sqlite_master
             WHERE type = 'table' AND name = ?1
             LIMIT 1",
            [table_name],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn table_columns(connection: &Connection, table_name: &str) -> Result<HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
}

fn generate_entry_uuid(connection: &Connection) -> Result<String> {
    let seed = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000) as u64;

    for offset in 0..10_000_u64 {
        let candidate = format!("entry_{}", base36_8(seed.wrapping_add(offset)));
        let exists = connection
            .query_row(
                "SELECT 1 FROM entries WHERE uuid = ?1 LIMIT 1",
                [&candidate],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Ok(candidate);
        }
    }

    Err(anyhow!("Unable to generate a unique entry UUID."))
}

fn base36_8(mut value: u64) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buffer = [b'0'; 8];
    for index in (0..8).rev() {
        buffer[index] = ALPHABET[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8_lossy(&buffer).to_string()
}

fn normalize_required_text(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        return Err(anyhow!("Entry text is required."));
    }
    Ok(value.replace("\r\n", "\n").replace('\r', "\n"))
}

fn normalize_content_format(value: Option<&str>) -> Result<String> {
    match normalized_string(value)
        .unwrap_or_else(|| "markdown".to_string())
        .to_lowercase()
        .as_str()
    {
        "plain" => Ok("plain".to_string()),
        "markdown" => Ok("markdown".to_string()),
        other => Err(anyhow!("Unsupported entry content format: {other}")),
    }
}

fn normalize_tags(values: Option<&[String]>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = Vec::new();

    for value in values.unwrap_or_default() {
        if let Some(tag) = normalized_string(Some(value)) {
            let tag = tag.to_lowercase();
            if seen.insert(tag.clone()) {
                tags.push(tag);
            }
        }
    }

    tags
}

fn build_text_plain(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalized_created_at(value: Option<&str>) -> String {
    normalized_string(value)
        .map(|value| value.replace('T', " "))
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d %H:%M").to_string())
}

fn current_updated_at() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string()
}

fn current_history_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn changed_fields(old_data: &JsonValue, next_data: &JsonValue) -> Vec<String> {
    [
        "text",
        "text_plain",
        "content_format",
        "title",
        "summary",
        "mood",
        "tags",
        "starred",
        "pinned",
        "hidden",
    ]
    .into_iter()
    .filter(|field| old_data.get(field) != next_data.get(field))
    .map(str::to_string)
    .collect()
}

fn normalized_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| normalized_string(Some(&value)))
}

fn normalized_vec(values: Option<&[String]>) -> Vec<String> {
    values
        .unwrap_or_default()
        .iter()
        .filter_map(|value| normalized_string(Some(value)))
        .collect()
}

fn like_pattern(value: &str) -> String {
    format!("%{}%", value.to_lowercase())
}

fn mood_label(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn placeholders(count: usize) -> String {
    std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NullableStringUpdate;
    use rusqlite::Connection;

    #[test]
    fn list_entries_returns_visible_entries_with_relations() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let response = list_entries_for_database(
            &db_path,
            EntryFilters {
                limit: Some(10),
                ..EntryFilters::default()
            },
        )
        .expect("entries");

        assert_eq!(response.total, 3);
        assert_eq!(response.entries[0].uuid, "entry_child");
        assert_eq!(response.entries[0].tags[0].name, "work");
        assert_eq!(response.entries[0].attachment_count, 2);
        assert_eq!(
            response.entries[0]
                .location
                .as_ref()
                .unwrap()
                .place_name
                .as_deref(),
            Some("Tromso")
        );
        assert_eq!(
            response.entries[0]
                .location
                .as_ref()
                .unwrap()
                .weather_icon
                .as_deref(),
            Some("cloudy")
        );
        assert_eq!(
            response.entries[0]
                .location
                .as_ref()
                .unwrap()
                .weather_humidity,
            Some(82)
        );
        assert_eq!(
            response.entries[0]
                .location
                .as_ref()
                .unwrap()
                .weather_wind_kph,
            Some(11.4)
        );
        assert_eq!(
            response.entries[0]
                .thread
                .as_ref()
                .unwrap()
                .title
                .as_deref(),
            Some("Thread title")
        );
    }

    #[test]
    fn list_entries_filters_tags_and_hides_hidden_by_default() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let response = list_entries_for_database(
            &db_path,
            EntryFilters {
                tags: Some(vec!["personal".to_string()]),
                ..EntryFilters::default()
            },
        )
        .expect("entries");

        assert_eq!(response.total, 1);
        assert_eq!(response.entries[0].uuid, "entry_root");

        let hidden_response = list_entries_for_database(
            &db_path,
            EntryFilters {
                include_hidden: Some(true),
                sort: Some(EntrySort::Asc),
                ..EntryFilters::default()
            },
        )
        .expect("entries");

        assert_eq!(hidden_response.total, 4);
        assert!(hidden_response
            .entries
            .iter()
            .any(|entry| entry.uuid == "entry_hidden"));
    }

    #[test]
    fn list_entries_exposes_entry_numbers_from_ids() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let response = list_entries_for_database(
            &db_path,
            EntryFilters {
                include_hidden: Some(true),
                sort: Some(EntrySort::Asc),
                ..EntryFilters::default()
            },
        )
        .expect("entries");

        let ids = response
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![1, 2, 3, 4]);
    }

    #[test]
    fn create_entry_returns_next_entry_number() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let response = create_entry_for_database(
            &db_path,
            EntryCreate {
                text: "Numbered new entry".to_string(),
                when: Some("2026-02-01T09:30".to_string()),
                ..EntryCreate::default()
            },
        )
        .expect("create entry");

        assert_eq!(response.entry.id, 5);
    }

    #[test]
    fn ensure_entry_ids_repairs_nullable_legacy_ids() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_nullable_id_fixture_database(temp_dir.path());

        ensure_entry_ids_for_database(&db_path).expect("repair ids");

        let response = list_entries_for_database(
            &db_path,
            EntryFilters {
                include_hidden: Some(true),
                sort: Some(EntrySort::Asc),
                ..EntryFilters::default()
            },
        )
        .expect("entries");
        assert_eq!(
            response
                .entries
                .iter()
                .map(|entry| (entry.id, entry.uuid.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "legacy_one"), (2, "legacy_two"), (3, "legacy_three")]
        );
    }

    #[test]
    fn get_entry_accepts_uuid_or_numeric_id() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let by_uuid = get_entry_for_database(&db_path, "entry_root").expect("by uuid");
        let by_id = get_entry_for_database(&db_path, "1").expect("by id");

        assert_eq!(by_uuid.uuid, "entry_root");
        assert_eq!(by_id.uuid, "entry_root");
    }

    #[test]
    fn random_entry_uses_read_only_filters() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let entry = get_random_entry_for_database(
            &db_path,
            RandomEntryFilters {
                tags: Some(vec!["work".to_string()]),
                ..RandomEntryFilters::default()
            },
        )
        .expect("random");

        assert!(entry.is_some());
        assert_eq!(entry.unwrap().uuid, "entry_child");
    }

    #[test]
    fn create_entry_writes_backup_tags_and_continuation() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let response = create_entry_for_database(
            &db_path,
            EntryCreate {
                text: "New markdown\n\nentry".to_string(),
                content_format: Some("markdown".to_string()),
                title: Some("New entry".to_string()),
                summary: Some("A safe write".to_string()),
                mood: Some("proud".to_string()),
                tags: Some(vec![
                    "Work".to_string(),
                    "rust".to_string(),
                    "work".to_string(),
                ]),
                when: Some("2026-02-01T09:30".to_string()),
                starred: Some(true),
                pinned: Some(true),
                continue_from_uuid: Some("entry_child".to_string()),
            },
        )
        .expect("create entry");

        assert!(std::path::PathBuf::from(&response.audit.backup_path).exists());
        assert_eq!(response.audit.operation, "entry.create");
        assert_eq!(response.entry.title.as_deref(), Some("New entry"));
        assert_eq!(response.entry.text_plain, "New markdown entry");
        assert!(response.entry.starred);
        assert!(response.entry.pinned);
        assert_eq!(response.entry.tags.len(), 2);
        assert_eq!(
            response
                .entry
                .thread
                .as_ref()
                .unwrap()
                .parent_uuid
                .as_deref(),
            Some("entry_child")
        );
    }

    #[test]
    fn create_entry_auto_captures_location_and_weather() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());
        std::fs::write(
            temp_dir.path().join("config.json"),
            r#"{"location.auto_capture": "true"}"#,
        )
        .expect("write config");
        crate::location::set_test_auto_capture_fixture(Some(
            crate::location::TestAutoCaptureFixture {
                latitude: 69.65,
                longitude: 18.96,
                place_name: Some("Tromso, Norway".to_string()),
                source: "default".to_string(),
                weather_temp_c: Some(-2.0),
                weather_condition: Some("Snow".to_string()),
            },
        ));

        let response = create_entry_for_database(
            &db_path,
            EntryCreate {
                text: "Weather capture entry".to_string(),
                when: Some("2026-02-01T09:30".to_string()),
                ..EntryCreate::default()
            },
        )
        .expect("create entry");
        crate::location::set_test_auto_capture_fixture(None);

        let location = response.entry.location.expect("location");
        assert_eq!(location.place_name.as_deref(), Some("Tromso, Norway"));
        assert_eq!(location.weather_condition.as_deref(), Some("Snow"));
        assert_eq!(location.weather_temp_c, Some(-2.0));
        assert_eq!(location.weather_temp_f, Some(28.4));
    }

    #[test]
    fn update_entry_records_history_and_preserves_backup_audit() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let response = update_entry_for_database(
            &db_path,
            "entry_root",
            EntryUpdate {
                text: Some("Changed text".to_string()),
                content_format: Some("markdown".to_string()),
                title: NullableStringUpdate::Value("Changed title".to_string()),
                summary: NullableStringUpdate::Null,
                mood: NullableStringUpdate::Value("focused".to_string()),
                tags: Some(vec!["Capsule".to_string(), "Rust".to_string()]),
                starred: Some(true),
                ..EntryUpdate::default()
            },
        )
        .expect("update entry");

        assert!(std::path::PathBuf::from(&response.audit.backup_path).exists());
        assert_eq!(response.entry.title.as_deref(), Some("Changed title"));
        assert_eq!(response.entry.summary, None);
        assert_eq!(response.entry.mood.as_deref(), Some("focused"));
        assert!(response.entry.starred);
        assert_eq!(
            response
                .entry
                .tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect::<Vec<_>>(),
            vec!["capsule", "rust"]
        );

        let history = list_entry_history_for_database(&db_path, "entry_root").expect("history");
        assert_eq!(history.count, 3);
        assert!(history
            .history
            .iter()
            .any(|item| item.changed_fields.iter().any(|field| field == "text")));
        assert!(history
            .history
            .iter()
            .any(|item| item.changed_fields.iter().any(|field| field == "tags")));
    }

    #[test]
    fn hide_entry_uses_backup_guard_without_hard_delete() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let hidden = backup::with_database_backup_for_database(&db_path, "entry.hide", |path| {
            set_entry_flag_inner(path, "entry_root", "hidden", true)
        })
        .expect("hide entry");

        assert!(hidden.value.hidden);
        assert!(std::path::PathBuf::from(hidden.audit.backup_path).exists());

        let visible = get_entry_for_database(&db_path, "entry_root").expect("entry");
        assert!(visible.hidden);
    }

    #[test]
    fn delete_entry_resequences_ids_and_clears_thread_relations() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        let response = delete_entry_for_database(&db_path, "entry_middle").expect("delete entry");

        assert!(std::path::PathBuf::from(&response.audit.backup_path).exists());
        assert_eq!(response.audit.operation, "entry.delete");
        assert_eq!(response.entry_id, 2);
        assert_eq!(response.entry_uuid, "entry_middle");

        let connection = Connection::open(&db_path).expect("open db");
        let missing_middle = connection
            .query_row(
                "SELECT 1 FROM entries WHERE uuid = 'entry_middle'",
                [],
                |_| Ok(()),
            )
            .optional()
            .expect("missing middle");
        assert!(missing_middle.is_none());

        let child_id = connection
            .query_row(
                "SELECT id FROM entries WHERE uuid = 'entry_child'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("child id");
        assert_eq!(child_id, 2);
        let child_tag_entry_id = connection
            .query_row(
                "SELECT entry_id FROM entry_tags WHERE tag_id = 2",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("entry tag");
        assert_eq!(child_tag_entry_id, 2);

        let continuation_count = connection
            .query_row("SELECT COUNT(*) FROM entry_continuations", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("continuations");
        assert_eq!(continuation_count, 0);
        let title_count = connection
            .query_row("SELECT COUNT(*) FROM entry_thread_titles", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("titles");
        assert_eq!(title_count, 0);
        let tombstone_count = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_tombstones WHERE uuid = 'entry_middle'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("sync tombstone");
        assert_eq!(tombstone_count, 1);
        let fts_count = connection
            .query_row("SELECT COUNT(*) FROM entries_fts", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("fts count");
        assert_eq!(fts_count, 3);

        let (operation_type, additional_data): (String, Option<String>) = connection
            .query_row(
                "SELECT operation_type, additional_data FROM history ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("history");
        assert_eq!(operation_type, "DELETE");
        let additional: JsonValue =
            serde_json::from_str(&additional_data.expect("additional data")).expect("json");
        assert!(additional.get("affected_ids").is_some());
        assert_eq!(
            additional
                .get("continuations")
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(2),
        );
        assert_eq!(
            additional
                .get("thread_titles")
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(1),
        );
    }

    #[test]
    fn delete_entry_removes_media_location_rows_and_tombstones() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_fixture_database(temp_dir.path());

        delete_entry_for_database(&db_path, "entry_child").expect("delete entry");

        let connection = Connection::open(&db_path).expect("open db");
        let media_count = connection
            .query_row(
                "SELECT COUNT(*) FROM plugin_entry_media WHERE entry_uuid = 'entry_child'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("entry media");
        assert_eq!(media_count, 0);
        let image_tombstones = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_image_tombstones WHERE entry_uuid = 'entry_child'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("image tombstones");
        assert_eq!(image_tombstones, 2);
        let deleted_assets = connection
            .query_row(
                "SELECT COUNT(*) FROM plugin_media_assets WHERE deleted_at IS NOT NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("deleted assets");
        assert_eq!(deleted_assets, 2);
        let location_count = connection
            .query_row(
                "SELECT COUNT(*) FROM plugin_entry_locations WHERE entry_uuid = 'entry_child'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("location");
        assert_eq!(location_count, 0);
        let location_tombstones = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_location_tombstones WHERE entry_uuid = 'entry_child'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("location tombstones");
        assert_eq!(location_tombstones, 1);
    }

    fn create_nullable_id_fixture_database(path: &Path) -> std::path::PathBuf {
        let db_path = path.join("legacy-nullable-id.db");
        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (
                    id INTEGER,
                    uuid TEXT UNIQUE,
                    created_at TEXT NOT NULL,
                    updated_at TEXT,
                    text TEXT NOT NULL,
                    text_plain TEXT NOT NULL DEFAULT '',
                    content_format TEXT NOT NULL DEFAULT 'plain',
                    title TEXT,
                    summary TEXT,
                    mood TEXT,
                    starred INTEGER DEFAULT 0,
                    pinned INTEGER DEFAULT 0,
                    hidden INTEGER DEFAULT 0
                );
                CREATE TABLE tags (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE
                );
                CREATE TABLE entry_tags (
                    entry_id INTEGER NOT NULL,
                    tag_id INTEGER NOT NULL,
                    PRIMARY KEY (entry_id, tag_id)
                );
                CREATE TABLE entries_fts (text);
                INSERT INTO entries
                    (id, uuid, created_at, updated_at, text, text_plain, content_format, hidden)
                VALUES
                    (NULL, 'legacy_two', '2026-01-02 08:00', '2026-01-02 08:00', 'Two', 'Two', 'plain', 0),
                    (NULL, 'legacy_one', '2026-01-01 08:00', '2026-01-01 08:00', 'One', 'One', 'plain', 0),
                    (NULL, 'legacy_three', '2026-01-03 08:00', '2026-01-03 08:00', 'Three', 'Three', 'plain', 0);
                INSERT INTO tags (name) VALUES ('legacy');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1), (2, 1), (3, 1);
                INSERT INTO entries_fts(rowid, text) VALUES (1, 'Two'), (2, 'One'), (3, 'Three');
                ",
            )
            .expect("fixture");
        drop(connection);
        std::fs::write(
            path.join("config.json"),
            r#"{"location.auto_capture": "false"}"#,
        )
        .expect("config");

        db_path
    }

    fn create_fixture_database(path: &Path) -> std::path::PathBuf {
        let db_path = path.join("capsule.db");
        let connection = Connection::open(&db_path).expect("open db");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    uuid TEXT UNIQUE,
                    created_at TEXT NOT NULL,
                    updated_at TEXT,
                    text TEXT NOT NULL,
                    text_plain TEXT NOT NULL DEFAULT '',
                    content_format TEXT NOT NULL DEFAULT 'plain',
                    title TEXT,
                    summary TEXT,
                    mood TEXT,
                    starred INTEGER DEFAULT 0,
                    pinned INTEGER DEFAULT 0,
                    hidden INTEGER DEFAULT 0
                );
                CREATE TABLE tags (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE
                );
                CREATE TABLE entry_tags (
                    entry_id INTEGER NOT NULL,
                    tag_id INTEGER NOT NULL,
                    PRIMARY KEY (entry_id, tag_id)
                );
                CREATE TABLE entry_continuations (
                    child_entry_uuid TEXT PRIMARY KEY,
                    parent_entry_uuid TEXT NOT NULL,
                    updated_at TEXT
                );
                CREATE TABLE entry_thread_titles (
                    thread_root_uuid TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE entry_thread_summaries (
                    thread_root_uuid TEXT PRIMARY KEY,
                    summary TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp TEXT NOT NULL,
                    operation_type TEXT NOT NULL,
                    entry_id INTEGER NOT NULL,
                    old_data TEXT NOT NULL,
                    additional_data TEXT,
                    undone INTEGER DEFAULT 0,
                    redo_data TEXT
                );
                CREATE TABLE entries_fts (text);
                CREATE TABLE plugin_media_assets (
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
                CREATE TABLE plugin_entry_media (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    entry_uuid TEXT NOT NULL,
                    media_id INTEGER NOT NULL,
                    position INTEGER NOT NULL DEFAULT 0,
                    caption TEXT,
                    alt_text TEXT,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE plugin_entry_locations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    entry_uuid TEXT NOT NULL UNIQUE,
                    latitude REAL NOT NULL,
                    longitude REAL NOT NULL,
                    place_name TEXT,
                    place_details TEXT,
                    source TEXT NOT NULL DEFAULT 'auto',
                    weather_condition TEXT,
                    weather_temp_c REAL,
                    weather_temp_f REAL,
                    weather_icon TEXT,
                    weather_humidity INTEGER,
                    weather_wind_kph REAL,
                    weather_fetched_at TEXT,
                    created_at TEXT NOT NULL
                );

                INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format, title, summary, mood, starred, pinned, hidden)
                VALUES
                    ('entry_root', '2026-01-01 08:00', '2026-01-01 08:00', 'Root text', 'Root text', 'markdown', 'Root', NULL, 'happy', 0, 0, 0),
                    ('entry_middle', '2026-01-02 08:00', '2026-01-02 08:00', 'Middle text', 'Middle text', 'plain', NULL, NULL, 'calm', 0, 0, 0),
                    ('entry_child', '2026-01-03 08:00', '2026-01-03 08:00', 'Child text', 'Child text', 'plain', 'Child', 'Summary', 'focused', 0, 0, 0),
                    ('entry_hidden', '2026-01-04 08:00', '2026-01-04 08:00', 'Hidden text', 'Hidden text', 'plain', NULL, NULL, 'quiet', 0, 0, 1);

                INSERT INTO tags (name) VALUES ('personal'), ('work');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1), (3, 2);
                INSERT INTO entry_continuations (child_entry_uuid, parent_entry_uuid, updated_at)
                VALUES ('entry_middle', 'entry_root', '2026-01-02 08:00'),
                       ('entry_child', 'entry_middle', '2026-01-03 08:00');
                INSERT INTO entry_thread_titles (thread_root_uuid, title, updated_at)
                VALUES ('entry_root', 'Thread title', '2026-01-03 08:00');
                INSERT INTO entry_thread_summaries (thread_root_uuid, summary, updated_at)
                VALUES ('entry_root', 'Thread summary', '2026-01-03 08:00');
                INSERT INTO entries_fts(rowid, text) SELECT id, text_plain FROM entries;
                INSERT INTO plugin_media_assets
                    (id, hash, mime_type, bytes, width, height, storage_backend, storage_key, created_at)
                VALUES
                    (1, 'asset-one', 'image/jpeg', 100, 400, 300, 'local_fs', 'aa/asset-one.jpg', '2026-01-03 08:00'),
                    (2, 'asset-two', 'image/jpeg', 120, 400, 300, 'local_fs', 'aa/asset-two.jpg', '2026-01-03 08:00');
                INSERT INTO plugin_entry_media (entry_uuid, media_id, position, created_at)
                VALUES ('entry_child', 1, 0, '2026-01-03 08:00'),
                       ('entry_child', 2, 1, '2026-01-03 08:00');
                INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, place_name, source,
                     weather_condition, weather_temp_c, weather_temp_f, weather_icon,
                     weather_humidity, weather_wind_kph, weather_fetched_at, created_at)
                VALUES ('entry_child', 69.65, 18.96, 'Tromso', 'manual',
                        'Overcast', 8.0, 46.4, 'cloudy', 82, 11.4,
                        '2026-01-03 08:05', '2026-01-03 08:00');
                ",
            )
            .expect("fixture");
        drop(connection);
        std::fs::write(
            path.join("config.json"),
            r#"{"location.auto_capture": "false"}"#,
        )
        .expect("config");

        db_path
    }
}
