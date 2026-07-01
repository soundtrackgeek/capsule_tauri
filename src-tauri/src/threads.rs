use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::{
    backup, db, entries,
    models::{
        BulkThreadDetachRequest, BulkThreadLinkRequest, ThreadGroup, ThreadListResponse,
        ThreadMetadataUpdate, ThreadMutationResponse,
    },
};

const DEFAULT_LIMIT: i64 = 30;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Clone)]
struct ContinuationLink {
    child_uuid: String,
    parent_uuid: String,
}

#[derive(Debug, Clone)]
struct ThreadMetadataChanges {
    title: Option<Option<String>>,
    summary: Option<Option<String>>,
}

pub fn list_threads(limit: Option<i64>, offset: Option<i64>) -> Result<ThreadListResponse> {
    list_threads_for_database(&db::resolve_database_path(), limit, offset)
}

pub(crate) fn list_threads_for_database(
    db_path: &Path,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ThreadListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = offset.unwrap_or(0).max(0);
    let groups = load_thread_groups(db_path)?;
    let total = groups.len() as i64;
    let threads = groups
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(ThreadListResponse {
        threads,
        total,
        limit,
        offset,
    })
}

pub fn update_thread_title(
    root_uuid: String,
    title: Option<String>,
) -> Result<ThreadMutationResponse> {
    let guarded = backup::with_database_backup("thread.title.update", move |db_path| {
        update_thread_metadata_inner(
            db_path,
            &root_uuid,
            ThreadMetadataChanges {
                title: Some(title),
                summary: None,
            },
        )
    })?;
    let (thread, affected_uuids) = guarded.value;
    Ok(ThreadMutationResponse {
        thread,
        affected_uuids,
        audit: guarded.audit,
    })
}

pub fn update_thread_metadata(
    root_uuid: String,
    input: ThreadMetadataUpdate,
) -> Result<ThreadMutationResponse> {
    let guarded = backup::with_database_backup("thread.metadata.update", move |db_path| {
        update_thread_metadata_inner(
            db_path,
            &root_uuid,
            ThreadMetadataChanges {
                title: input.title.as_optional_value(),
                summary: input.summary.as_optional_value(),
            },
        )
    })?;
    let (thread, affected_uuids) = guarded.value;
    Ok(ThreadMutationResponse {
        thread,
        affected_uuids,
        audit: guarded.audit,
    })
}

pub fn bulk_detach_threads(input: BulkThreadDetachRequest) -> Result<ThreadMutationResponse> {
    let guarded = backup::with_database_backup("thread.detach", move |db_path| {
        bulk_detach_threads_inner(db_path, input)
    })?;
    let (thread, affected_uuids) = guarded.value;
    Ok(ThreadMutationResponse {
        thread,
        affected_uuids,
        audit: guarded.audit,
    })
}

pub fn bulk_link_threads(input: BulkThreadLinkRequest) -> Result<ThreadMutationResponse> {
    let guarded = backup::with_database_backup("thread.link", move |db_path| {
        bulk_link_threads_inner(db_path, input)
    })?;
    let (thread, affected_uuids) = guarded.value;
    Ok(ThreadMutationResponse {
        thread,
        affected_uuids,
        audit: guarded.audit,
    })
}

pub fn disband_thread(root_uuid: String) -> Result<ThreadMutationResponse> {
    let guarded = backup::with_database_backup("thread.disband", move |db_path| {
        disband_thread_inner(db_path, &root_uuid)
    })?;
    let affected_uuids = guarded.value;
    Ok(ThreadMutationResponse {
        thread: None,
        affected_uuids,
        audit: guarded.audit,
    })
}

fn load_thread_groups(db_path: &Path) -> Result<Vec<ThreadGroup>> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = db::inspect_schema(&connection)?
        .detected_tables
        .into_iter()
        .collect::<HashSet<_>>();
    if !tables.contains("entries") || !tables.contains("entry_continuations") {
        return Ok(Vec::new());
    }

    let links = load_links(&connection)?;
    if links.is_empty() {
        return Ok(Vec::new());
    }

    let parent_by_child = parent_map(&links);
    let mut nodes = links
        .iter()
        .flat_map(|link| [link.child_uuid.clone(), link.parent_uuid.clone()])
        .collect::<HashSet<_>>();
    let mut roots = HashSet::new();
    for node in nodes.clone() {
        roots.insert(root_for(&parent_by_child, &node));
    }
    nodes.extend(roots.iter().cloned());

    let titles = if tables.contains("entry_thread_titles") {
        load_thread_text_map(
            &connection,
            "SELECT thread_root_uuid, title FROM entry_thread_titles",
        )?
    } else {
        HashMap::new()
    };
    let summaries = if tables.contains("entry_thread_summaries") {
        load_thread_text_map(
            &connection,
            "SELECT thread_root_uuid, summary FROM entry_thread_summaries",
        )?
    } else {
        HashMap::new()
    };
    drop(connection);

    let ordered_nodes = nodes.into_iter().collect::<Vec<_>>();
    let loaded_entries = entries::list_entries_by_uuids_for_database(db_path, &ordered_nodes)?;
    let entries_by_uuid = loaded_entries
        .into_iter()
        .map(|entry| (entry.uuid.clone(), entry))
        .collect::<HashMap<_, _>>();
    let children_by_parent = children_map(&links, &entries_by_uuid);

    let mut groups = roots
        .into_iter()
        .filter_map(|root_uuid| {
            if !entries_by_uuid.contains_key(&root_uuid) {
                return None;
            }

            let mut ordered = Vec::new();
            append_thread_entries(
                &root_uuid,
                &children_by_parent,
                &entries_by_uuid,
                &mut ordered,
            );
            if ordered.len() < 2 {
                return None;
            }

            let latest_activity = ordered
                .iter()
                .filter_map(|entry| {
                    entry
                        .updated_at
                        .clone()
                        .or_else(|| Some(entry.created_at.clone()))
                })
                .max();
            Some(ThreadGroup {
                title: titles.get(&root_uuid).cloned().or_else(|| {
                    entries_by_uuid
                        .get(&root_uuid)
                        .and_then(|entry| entry.title.clone())
                }),
                summary: summaries.get(&root_uuid).cloned(),
                entry_count: ordered.len(),
                latest_activity,
                entries: ordered,
                root_uuid,
            })
        })
        .collect::<Vec<_>>();

    groups.sort_by(|left, right| {
        right
            .latest_activity
            .cmp(&left.latest_activity)
            .then_with(|| left.root_uuid.cmp(&right.root_uuid))
    });
    Ok(groups)
}

fn update_thread_metadata_inner(
    db_path: &Path,
    root_uuid: &str,
    changes: ThreadMetadataChanges,
) -> Result<(Option<ThreadGroup>, Vec<String>)> {
    let mut connection = db::open_read_write_connection(db_path)?;
    ensure_thread_tables(&connection)?;
    let tx = connection.transaction()?;
    let requested_uuid = resolve_entry_uuid(&tx, root_uuid)?;
    let actual_root = root_for_connection(&tx, &requested_uuid)?;

    if let Some(title) = changes.title {
        upsert_thread_text(&tx, "entry_thread_titles", "title", &actual_root, title)?;
    }
    if let Some(summary) = changes.summary {
        upsert_thread_text(
            &tx,
            "entry_thread_summaries",
            "summary",
            &actual_root,
            summary,
        )?;
    }
    tx.commit()?;

    let thread = thread_group_for_root(db_path, &actual_root)?;
    Ok((thread, vec![actual_root]))
}

fn bulk_detach_threads_inner(
    db_path: &Path,
    input: BulkThreadDetachRequest,
) -> Result<(Option<ThreadGroup>, Vec<String>)> {
    let child_inputs = normalize_uuid_list(input.child_uuids);
    if child_inputs.is_empty() {
        return Err(anyhow!("At least one child UUID is required."));
    }

    let mut connection = db::open_read_write_connection(db_path)?;
    ensure_thread_tables(&connection)?;
    let tx = connection.transaction()?;
    let first_child = resolve_entry_uuid(&tx, &child_inputs[0])?;
    let old_root = root_for_connection(&tx, &first_child)?;
    let mut affected = Vec::new();

    for child_input in child_inputs {
        let child_uuid = resolve_entry_uuid(&tx, &child_input)?;
        record_continuation_tombstone_if_linked(&tx, &child_uuid, &current_timestamp())?;
        tx.execute(
            "DELETE FROM entry_continuations WHERE child_entry_uuid = ?1",
            [&child_uuid],
        )?;
        affected.push(child_uuid);
    }

    tx.commit()?;
    let thread = thread_group_for_root(db_path, &old_root)?;
    Ok((thread, affected))
}

fn bulk_link_threads_inner(
    db_path: &Path,
    input: BulkThreadLinkRequest,
) -> Result<(Option<ThreadGroup>, Vec<String>)> {
    let child_inputs = normalize_uuid_list(input.child_uuids);
    if child_inputs.is_empty() {
        return Err(anyhow!("At least one child UUID is required."));
    }

    let mut connection = db::open_read_write_connection(db_path)?;
    ensure_thread_tables(&connection)?;
    let tx = connection.transaction()?;
    let parent_uuid = resolve_entry_uuid(&tx, &input.parent_uuid)?;
    let mut affected = Vec::new();

    for child_input in child_inputs {
        let child_uuid = resolve_entry_uuid(&tx, &child_input)?;
        if child_uuid == parent_uuid {
            return Err(anyhow!("An entry cannot continue itself."));
        }
        if continuation_would_cycle(&tx, &child_uuid, &parent_uuid)? {
            return Err(anyhow!("Continuation would create a thread cycle."));
        }
        tx.execute(
            "INSERT INTO entry_continuations (child_entry_uuid, parent_entry_uuid, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(child_entry_uuid)
             DO UPDATE SET parent_entry_uuid = excluded.parent_entry_uuid,
                           updated_at = excluded.updated_at",
            params![child_uuid, parent_uuid, current_timestamp()],
        )?;
        tx.execute(
            "DELETE FROM sync_entry_continuation_tombstones WHERE child_entry_uuid = ?1",
            [&child_uuid],
        )
        .ok();
        affected.push(child_uuid);
    }

    let root = root_for_connection(&tx, &parent_uuid)?;
    tx.commit()?;
    let thread = thread_group_for_root(db_path, &root)?;
    Ok((thread, affected))
}

fn disband_thread_inner(db_path: &Path, root_uuid: &str) -> Result<Vec<String>> {
    let mut connection = db::open_read_write_connection(db_path)?;
    ensure_thread_tables(&connection)?;
    let tx = connection.transaction()?;
    let requested_uuid = resolve_entry_uuid(&tx, root_uuid)?;
    let actual_root = root_for_connection(&tx, &requested_uuid)?;
    let nodes = thread_nodes_for_root(&tx, &actual_root)?;
    if nodes.len() < 2 {
        return Err(anyhow!("Thread not found: {actual_root}"));
    }

    let placeholders = placeholders(nodes.len());
    let now = current_timestamp();
    for child_uuid in child_uuids_with_continuations(&tx, &nodes)? {
        record_continuation_tombstone(&tx, &child_uuid, &now)?;
    }
    tx.execute(
        &format!(
            "DELETE FROM entry_continuations
             WHERE child_entry_uuid IN ({placeholders})
                OR parent_entry_uuid IN ({placeholders})"
        ),
        params_from_iter(
            nodes
                .iter()
                .cloned()
                .chain(nodes.iter().cloned())
                .map(Value::Text),
        ),
    )?;
    delete_thread_text(&tx, "entry_thread_titles", &actual_root)?;
    delete_thread_text(&tx, "entry_thread_summaries", &actual_root)?;
    tx.commit()?;

    Ok(nodes)
}

fn thread_group_for_root(db_path: &Path, root_uuid: &str) -> Result<Option<ThreadGroup>> {
    Ok(load_thread_groups(db_path)?
        .into_iter()
        .find(|thread| thread.root_uuid == root_uuid))
}

fn ensure_thread_tables(connection: &Connection) -> Result<()> {
    for table in ["entries", "entry_continuations"] {
        if !table_exists(connection, table)? {
            return Err(anyhow!(
                "The active database does not contain the required {table} table."
            ));
        }
    }
    Ok(())
}

fn upsert_thread_text(
    connection: &Connection,
    table: &str,
    value_column: &str,
    root_uuid: &str,
    value: Option<String>,
) -> Result<()> {
    let Some(value) = normalize_string(value.as_deref()) else {
        return delete_thread_text(connection, table, root_uuid);
    };
    if !table_exists(connection, table)? {
        return Err(anyhow!(
            "The active database does not contain the required {table} table."
        ));
    }

    let (table, value_column) = match (table, value_column) {
        ("entry_thread_titles", "title") => ("entry_thread_titles", "title"),
        ("entry_thread_summaries", "summary") => ("entry_thread_summaries", "summary"),
        _ => return Err(anyhow!("Unsupported thread metadata table: {table}")),
    };
    let sql = format!(
        "INSERT INTO {table} (thread_root_uuid, {value_column}, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(thread_root_uuid)
         DO UPDATE SET {value_column} = excluded.{value_column},
                       updated_at = excluded.updated_at"
    );
    connection.execute(&sql, params![root_uuid, value, current_timestamp()])?;
    delete_thread_text_tombstone(connection, table, root_uuid)?;
    Ok(())
}

fn delete_thread_text(connection: &Connection, table: &str, root_uuid: &str) -> Result<()> {
    let table = match table {
        "entry_thread_titles" => "entry_thread_titles",
        "entry_thread_summaries" => "entry_thread_summaries",
        _ => return Err(anyhow!("Unsupported thread metadata table: {table}")),
    };
    if table_exists(connection, table)? {
        let existed = connection
            .query_row(
                &format!("SELECT 1 FROM {table} WHERE thread_root_uuid = ?1 LIMIT 1"),
                [root_uuid],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        connection.execute(
            &format!("DELETE FROM {table} WHERE thread_root_uuid = ?1"),
            [root_uuid],
        )?;
        if existed {
            record_thread_text_tombstone(connection, table, root_uuid, &current_timestamp())?;
        }
    }
    Ok(())
}

fn ensure_continuation_tombstones(connection: &Connection) -> Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS sync_entry_continuation_tombstones (
            child_entry_uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn record_continuation_tombstone_if_linked(
    connection: &Connection,
    child_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    let existed = connection
        .query_row(
            "SELECT 1 FROM entry_continuations WHERE child_entry_uuid = ?1 LIMIT 1",
            [child_uuid],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if existed {
        record_continuation_tombstone(connection, child_uuid, deleted_at)?;
    }
    Ok(())
}

fn record_continuation_tombstone(
    connection: &Connection,
    child_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    ensure_continuation_tombstones(connection)?;
    connection.execute(
        "INSERT INTO sync_entry_continuation_tombstones (child_entry_uuid, deleted_at)
         VALUES (?1, ?2)
         ON CONFLICT(child_entry_uuid)
         DO UPDATE SET deleted_at = excluded.deleted_at",
        params![child_uuid, deleted_at],
    )?;
    Ok(())
}

fn child_uuids_with_continuations(
    connection: &Connection,
    nodes: &[String],
) -> Result<Vec<String>> {
    if nodes.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = placeholders(nodes.len());
    let sql = format!(
        "SELECT child_entry_uuid
         FROM entry_continuations
         WHERE child_entry_uuid IN ({placeholders})
            OR parent_entry_uuid IN ({placeholders})"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params_from_iter(
            nodes
                .iter()
                .cloned()
                .chain(nodes.iter().cloned())
                .map(Value::Text),
        ),
        |row| row.get::<_, String>(0),
    )?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn ensure_thread_text_tombstones(connection: &Connection, table: &str) -> Result<()> {
    match table {
        "entry_thread_titles" => connection.execute(
            "CREATE TABLE IF NOT EXISTS sync_entry_thread_title_tombstones (
                thread_root_uuid TEXT PRIMARY KEY,
                deleted_at TEXT NOT NULL
            )",
            [],
        )?,
        "entry_thread_summaries" => connection.execute(
            "CREATE TABLE IF NOT EXISTS sync_entry_thread_summary_tombstones (
                thread_root_uuid TEXT PRIMARY KEY,
                deleted_at TEXT NOT NULL
            )",
            [],
        )?,
        _ => return Err(anyhow!("Unsupported thread metadata table: {table}")),
    };
    Ok(())
}

fn record_thread_text_tombstone(
    connection: &Connection,
    table: &str,
    root_uuid: &str,
    deleted_at: &str,
) -> Result<()> {
    ensure_thread_text_tombstones(connection, table)?;
    let tombstone_table = match table {
        "entry_thread_titles" => "sync_entry_thread_title_tombstones",
        "entry_thread_summaries" => "sync_entry_thread_summary_tombstones",
        _ => return Err(anyhow!("Unsupported thread metadata table: {table}")),
    };
    connection.execute(
        &format!(
            "INSERT INTO {tombstone_table} (thread_root_uuid, deleted_at)
             VALUES (?1, ?2)
             ON CONFLICT(thread_root_uuid)
             DO UPDATE SET deleted_at = excluded.deleted_at"
        ),
        params![root_uuid, deleted_at],
    )?;
    Ok(())
}

fn delete_thread_text_tombstone(
    connection: &Connection,
    table: &str,
    root_uuid: &str,
) -> Result<()> {
    let tombstone_table = match table {
        "entry_thread_titles" => "sync_entry_thread_title_tombstones",
        "entry_thread_summaries" => "sync_entry_thread_summary_tombstones",
        _ => return Err(anyhow!("Unsupported thread metadata table: {table}")),
    };
    if table_exists(connection, tombstone_table)? {
        connection.execute(
            &format!("DELETE FROM {tombstone_table} WHERE thread_root_uuid = ?1"),
            [root_uuid],
        )?;
    }
    Ok(())
}

fn load_links(connection: &Connection) -> Result<Vec<ContinuationLink>> {
    let mut statement = connection.prepare(
        "SELECT child_entry_uuid, parent_entry_uuid
         FROM entry_continuations
         WHERE COALESCE(child_entry_uuid, '') != ''
           AND COALESCE(parent_entry_uuid, '') != ''",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ContinuationLink {
            child_uuid: row.get(0)?,
            parent_uuid: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_thread_text_map(connection: &Connection, sql: &str) -> Result<HashMap<String, String>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut values = HashMap::new();
    for row in rows {
        let (root_uuid, value) = row?;
        if let Some(value) = normalize_string(Some(&value)) {
            values.insert(root_uuid, value);
        }
    }
    Ok(values)
}

fn children_map(
    links: &[ContinuationLink],
    entries_by_uuid: &HashMap<String, crate::models::Entry>,
) -> HashMap<String, Vec<String>> {
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for link in links {
        if entries_by_uuid.contains_key(&link.child_uuid)
            && entries_by_uuid.contains_key(&link.parent_uuid)
        {
            children
                .entry(link.parent_uuid.clone())
                .or_default()
                .push(link.child_uuid.clone());
        }
    }

    for child_uuids in children.values_mut() {
        child_uuids.sort_by(|left, right| {
            let left_key = entries_by_uuid
                .get(left)
                .map(|entry| (entry.created_at.clone(), entry.id))
                .unwrap_or_default();
            let right_key = entries_by_uuid
                .get(right)
                .map(|entry| (entry.created_at.clone(), entry.id))
                .unwrap_or_default();
            left_key.cmp(&right_key)
        });
    }
    children
}

fn append_thread_entries(
    uuid: &str,
    children_by_parent: &HashMap<String, Vec<String>>,
    entries_by_uuid: &HashMap<String, crate::models::Entry>,
    ordered: &mut Vec<crate::models::Entry>,
) {
    if let Some(entry) = entries_by_uuid.get(uuid) {
        ordered.push(entry.clone());
    }
    if let Some(children) = children_by_parent.get(uuid) {
        for child in children {
            append_thread_entries(child, children_by_parent, entries_by_uuid, ordered);
        }
    }
}

fn parent_map(links: &[ContinuationLink]) -> HashMap<String, String> {
    links
        .iter()
        .map(|link| (link.child_uuid.clone(), link.parent_uuid.clone()))
        .collect()
}

fn root_for(parent_by_child: &HashMap<String, String>, uuid: &str) -> String {
    let mut current = uuid.to_string();
    let mut seen = HashSet::new();

    while let Some(parent) = parent_by_child.get(&current) {
        if !seen.insert(current.clone()) {
            break;
        }
        current = parent.clone();
    }

    current
}

fn root_for_connection(connection: &Connection, uuid: &str) -> Result<String> {
    let mut current = uuid.to_string();
    let mut seen = HashSet::new();

    loop {
        if !seen.insert(current.clone()) {
            return Err(anyhow!("Thread cycle detected while resolving {uuid}."));
        }
        let parent = connection
            .query_row(
                "SELECT parent_entry_uuid
                 FROM entry_continuations
                 WHERE child_entry_uuid = ?1",
                [&current],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        match parent {
            Some(parent) => current = parent,
            None => return Ok(current),
        }
    }
}

fn thread_nodes_for_root(connection: &Connection, root_uuid: &str) -> Result<Vec<String>> {
    let links = load_links(connection)?;
    let parent_by_child = parent_map(&links);
    let mut nodes = links
        .iter()
        .flat_map(|link| [link.child_uuid.clone(), link.parent_uuid.clone()])
        .collect::<HashSet<_>>();
    nodes.insert(root_uuid.to_string());

    let mut thread_nodes = nodes
        .into_iter()
        .filter(|uuid| root_for(&parent_by_child, uuid) == root_uuid)
        .collect::<Vec<_>>();
    thread_nodes.sort();
    Ok(thread_nodes)
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

fn resolve_entry_uuid(connection: &Connection, identifier: &str) -> Result<String> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow!("Entry UUID is required."));
    }

    connection
        .query_row(
            "SELECT COALESCE(NULLIF(uuid, ''), 'entry_' || id) AS uuid
             FROM entries
             WHERE uuid = ?1 OR CAST(id AS TEXT) = ?1
             LIMIT 1",
            [identifier],
            |row| row.get::<_, String>(0),
        )
        .with_context(|| format!("entry not found: {identifier}"))
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

fn normalize_uuid_list(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter_map(|value| normalize_string(Some(&value)))
        .filter(|value| seen.insert(value.to_lowercase()))
        .collect()
}

fn normalize_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn current_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
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

    #[test]
    fn list_threads_groups_entries_in_continuation_order() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_thread_fixture(temp_dir.path());

        let response = list_threads_for_database(&db_path, Some(20), Some(0)).expect("threads");

        assert_eq!(response.total, 1);
        assert_eq!(response.threads[0].root_uuid, "entry_root");
        assert_eq!(response.threads[0].title.as_deref(), Some("Thread title"));
        assert_eq!(
            response.threads[0]
                .entries
                .iter()
                .map(|entry| entry.uuid.as_str())
                .collect::<Vec<_>>(),
            vec!["entry_root", "entry_middle", "entry_leaf"]
        );
    }

    #[test]
    fn thread_metadata_update_is_backup_guarded() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_thread_fixture(temp_dir.path());

        let guarded =
            backup::with_database_backup_for_database(&db_path, "thread.metadata.update", |path| {
                update_thread_metadata_inner(
                    path,
                    "entry_root",
                    ThreadMetadataChanges {
                        title: Some(Some("Changed".to_string())),
                        summary: Some(Some("Updated summary".to_string())),
                    },
                )
            })
            .expect("metadata");

        assert!(std::path::PathBuf::from(guarded.audit.backup_path).exists());
        let thread = guarded.value.0.expect("thread");
        assert_eq!(thread.title.as_deref(), Some("Changed"));
        assert_eq!(thread.summary.as_deref(), Some("Updated summary"));
    }

    #[test]
    fn thread_link_rejects_cycles() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_thread_fixture(temp_dir.path());

        let result = bulk_link_threads_inner(
            &db_path,
            BulkThreadLinkRequest {
                parent_uuid: "entry_leaf".to_string(),
                child_uuids: vec!["entry_root".to_string()],
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn detach_and_disband_update_thread_links() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_thread_fixture(temp_dir.path());

        let detached =
            backup::with_database_backup_for_database(&db_path, "thread.detach", |path| {
                bulk_detach_threads_inner(
                    path,
                    BulkThreadDetachRequest {
                        child_uuids: vec!["entry_leaf".to_string()],
                    },
                )
            })
            .expect("detach");
        assert_eq!(detached.value.1, vec!["entry_leaf".to_string()]);

        let after_detach = list_threads_for_database(&db_path, Some(20), Some(0)).expect("threads");
        assert_eq!(after_detach.threads[0].entry_count, 2);

        let disbanded =
            backup::with_database_backup_for_database(&db_path, "thread.disband", |path| {
                disband_thread_inner(path, "entry_root")
            })
            .expect("disband");
        assert_eq!(disbanded.value.len(), 2);

        let after_disband =
            list_threads_for_database(&db_path, Some(20), Some(0)).expect("threads");
        assert_eq!(after_disband.total, 0);
    }

    fn create_thread_fixture(path: &Path) -> std::path::PathBuf {
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
                INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format, title, summary, mood, hidden)
                VALUES
                    ('entry_root', '2026-01-01 08:00', '2026-01-01 08:00', 'Root text', 'Root text', 'markdown', 'Root', NULL, 'happy', 0),
                    ('entry_middle', '2026-01-02 08:00', '2026-01-02 08:00', 'Middle text', 'Middle text', 'markdown', NULL, NULL, 'calm', 0),
                    ('entry_leaf', '2026-01-03 08:00', '2026-01-03 08:00', 'Leaf text', 'Leaf text', 'markdown', NULL, NULL, 'focused', 0),
                    ('entry_solo', '2026-01-04 08:00', '2026-01-04 08:00', 'Solo text', 'Solo text', 'markdown', NULL, NULL, 'quiet', 0);
                INSERT INTO entry_continuations (child_entry_uuid, parent_entry_uuid, updated_at)
                VALUES ('entry_middle', 'entry_root', '2026-01-02 08:00'),
                       ('entry_leaf', 'entry_middle', '2026-01-03 08:00');
                INSERT INTO entry_thread_titles (thread_root_uuid, title, updated_at)
                VALUES ('entry_root', 'Thread title', '2026-01-03 08:00');
                INSERT INTO entry_thread_summaries (thread_root_uuid, summary, updated_at)
                VALUES ('entry_root', 'Thread summary', '2026-01-03 08:00');
                ",
            )
            .expect("fixture");
        drop(connection);

        db_path
    }
}
