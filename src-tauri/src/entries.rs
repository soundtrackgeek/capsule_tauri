use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use rusqlite::{params_from_iter, types::Value, Connection};

use crate::{
    db,
    models::{
        Entry, EntryFilters, EntryListResponse, EntrySort, EntryThreadInfo, LocationInfo, MoodInfo,
        RandomEntryFilters, TagInfo,
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

pub fn list_entries(filters: EntryFilters) -> Result<EntryListResponse> {
    list_entries_for_database(&db::resolve_database_path(), filters)
}

pub fn list_entries_for_database(
    db_path: &Path,
    filters: EntryFilters,
) -> Result<EntryListResponse> {
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

pub fn get_random_entry(filters: RandomEntryFilters) -> Result<Option<Entry>> {
    get_random_entry_for_database(&db::resolve_database_path(), filters)
}

pub fn get_random_entry_for_database(
    db_path: &Path,
    filters: RandomEntryFilters,
) -> Result<Option<Entry>> {
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

    let uuids = entries
        .iter()
        .map(|entry| entry.uuid.clone())
        .collect::<Vec<_>>();
    let placeholders = placeholders(uuids.len());
    let sql = format!(
        "SELECT entry_uuid, latitude, longitude, place_name, weather_condition, weather_temp_c, weather_temp_f
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
                    weather_condition: row.get(4)?,
                    weather_temp_c: row.get(5)?,
                    weather_temp_f: row.get(6)?,
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
                    weather_condition TEXT,
                    weather_temp_c REAL,
                    weather_temp_f REAL,
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
                INSERT INTO plugin_entry_media (entry_uuid, media_id, position, created_at)
                VALUES ('entry_child', 1, 0, '2026-01-03 08:00'),
                       ('entry_child', 2, 1, '2026-01-03 08:00');
                INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, place_name, weather_condition, weather_temp_c, weather_temp_f, created_at)
                VALUES ('entry_child', 69.65, 18.96, 'Tromso', 'Overcast', 8.0, 46.4, '2026-01-03 08:00');
                ",
            )
            .expect("fixture");
        drop(connection);

        db_path
    }
}
