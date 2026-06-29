use std::{collections::HashSet, path::Path};

use anyhow::{Context, Result};
use rusqlite::{params_from_iter, types::Value};

use crate::{
    db, entries,
    models::{
        EntryFilters, EntrySort, SearchMode, SearchRequest, SearchResponse, StructuredQueryToken,
        StructuredTokenKind,
    },
};

const DEFAULT_LIMIT: i64 = 40;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Clone)]
struct ParsedSearch {
    keyword: Option<String>,
    filters: EntryFilters,
    requested_mode: SearchMode,
    tokens: Vec<StructuredQueryToken>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct SqlFilter {
    where_sql: String,
    params: Vec<Value>,
}

pub fn search_entries(input: SearchRequest) -> Result<SearchResponse> {
    search_entries_for_database(&db::resolve_database_path(), input)
}

pub(crate) fn search_entries_for_database(
    db_path: &Path,
    input: SearchRequest,
) -> Result<SearchResponse> {
    let mut parsed = parse_search_request(input);
    let mode = match parsed.requested_mode {
        SearchMode::Keyword => SearchMode::Keyword,
        SearchMode::Semantic | SearchMode::Hybrid => {
            parsed.warnings.push(
                "Semantic and hybrid search are not implemented yet; using keyword search."
                    .to_string(),
            );
            SearchMode::Keyword
        }
    };

    let connection = db::open_read_only_connection(db_path)?;
    let schema = db::inspect_schema(&connection)?;
    let tables = schema.detected_tables.into_iter().collect::<HashSet<_>>();
    drop(connection);

    if mode == SearchMode::Keyword && parsed.keyword.is_some() && tables.contains("entries_fts") {
        match search_with_fts(db_path, &tables, &parsed) {
            Ok(response) => {
                return Ok(SearchResponse {
                    mode,
                    parsed_tokens: parsed.tokens,
                    warnings: parsed.warnings,
                    ..response
                });
            }
            Err(error) => parsed.warnings.push(format!(
                "FTS search failed, so Capsule used the compatibility search path: {error}"
            )),
        }
    }

    let response = entries::list_entries_for_database(db_path, parsed.filters)?;
    Ok(SearchResponse {
        entries: response.entries,
        total: response.total,
        limit: response.limit,
        offset: response.offset,
        mode,
        used_fts: false,
        parsed_tokens: parsed.tokens,
        warnings: parsed.warnings,
    })
}

fn parse_search_request(input: SearchRequest) -> ParsedSearch {
    let mut filters = EntryFilters {
        location: normalize_string(input.location.as_deref()),
        since: normalize_string(input.since.as_deref()),
        until: normalize_string(input.until.as_deref()),
        tags: normalize_vec(input.tags),
        exclude_tags: normalize_vec(input.exclude_tags),
        moods: normalize_vec(input.moods),
        exclude_moods: normalize_vec(input.exclude_moods),
        starred: input.starred,
        pinned: input.pinned,
        hidden: input.hidden,
        include_hidden: input.include_hidden,
        has_images: input.has_images,
        limit: Some(input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)),
        offset: Some(input.offset.unwrap_or(0).max(0)),
        sort: input.sort,
        ..EntryFilters::default()
    };
    let mut tokens = Vec::new();
    let mut keywords = Vec::new();
    let mut warnings = Vec::new();

    let mode = input.mode.unwrap_or(SearchMode::Keyword);
    let raw_tokens = input.query.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < raw_tokens.len() {
        let mut negated = false;
        let mut token = raw_tokens[index];
        if token.eq_ignore_ascii_case("NOT") && index + 1 < raw_tokens.len() {
            negated = true;
            index += 1;
            token = raw_tokens[index];
        }

        if let Some(value) = token
            .strip_prefix("tag:")
            .and_then(|value| normalize_string(Some(value)))
        {
            if negated {
                push_filter_value(&mut filters.exclude_tags, value.clone());
                tokens.push(StructuredQueryToken {
                    kind: StructuredTokenKind::ExcludeTag,
                    value,
                });
            } else {
                push_filter_value(&mut filters.tags, value.clone());
                tokens.push(StructuredQueryToken {
                    kind: StructuredTokenKind::Tag,
                    value,
                });
            }
        } else if let Some(value) = token
            .strip_prefix("mood:")
            .and_then(|value| normalize_string(Some(value)))
        {
            if negated {
                push_filter_value(&mut filters.exclude_moods, value.clone());
                tokens.push(StructuredQueryToken {
                    kind: StructuredTokenKind::ExcludeMood,
                    value,
                });
            } else {
                push_filter_value(&mut filters.moods, value.clone());
                tokens.push(StructuredQueryToken {
                    kind: StructuredTokenKind::Mood,
                    value,
                });
            }
        } else if let Some(value) = token
            .strip_prefix("before:")
            .and_then(|value| normalize_string(Some(value)))
        {
            filters.until = Some(value.clone());
            tokens.push(StructuredQueryToken {
                kind: StructuredTokenKind::Before,
                value,
            });
        } else if let Some(value) = token
            .strip_prefix("after:")
            .and_then(|value| normalize_string(Some(value)))
        {
            filters.since = Some(value.clone());
            tokens.push(StructuredQueryToken {
                kind: StructuredTokenKind::After,
                value,
            });
        } else {
            if negated {
                keywords.push("NOT".to_string());
            }
            keywords.push(token.to_string());
        }

        index += 1;
    }

    let keyword = normalize_string(Some(&keywords.join(" ")));
    if let Some(keyword) = keyword.clone() {
        filters.text = Some(keyword.clone());
        tokens.push(StructuredQueryToken {
            kind: StructuredTokenKind::Keyword,
            value: keyword,
        });
    }

    if mode != SearchMode::Keyword {
        warnings.push("Semantic and hybrid modes are visible in the UI but remain disabled until AI search lands.".to_string());
    }

    ParsedSearch {
        keyword,
        filters,
        requested_mode: mode,
        tokens,
        warnings,
    }
}

fn search_with_fts(
    db_path: &Path,
    tables: &HashSet<String>,
    parsed: &ParsedSearch,
) -> Result<SearchResponse> {
    let keyword = parsed.keyword.as_deref().unwrap_or_default();
    let fts_query = fts_query_from_keyword(keyword);
    let connection = db::open_read_only_connection(db_path)?;
    let filter = build_sql_filter(&parsed.filters, tables);
    let mut params = vec![Value::Text(fts_query)];
    params.extend(filter.params.clone());

    let total_sql = format!(
        "SELECT COUNT(*)
         FROM entries e
         JOIN entries_fts ON entries_fts.rowid = e.id
         WHERE entries_fts MATCH ?{}",
        filter.where_sql
    );
    let total = connection
        .query_row(&total_sql, params_from_iter(params.clone()), |row| {
            row.get::<_, i64>(0)
        })
        .context("failed to count FTS search results")?;

    let limit = parsed
        .filters
        .limit
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);
    let offset = parsed.filters.offset.unwrap_or(0).max(0);
    let order_by = match parsed.filters.sort.clone().unwrap_or(EntrySort::Desc) {
        EntrySort::Asc => "ORDER BY datetime(e.created_at) ASC, e.id ASC",
        EntrySort::Desc => "ORDER BY datetime(e.created_at) DESC, e.id DESC",
    };
    let query_sql = format!(
        "SELECT COALESCE(NULLIF(e.uuid, ''), 'entry_' || e.id) AS uuid
         FROM entries e
         JOIN entries_fts ON entries_fts.rowid = e.id
         WHERE entries_fts MATCH ?{}
         {order_by}
         LIMIT ? OFFSET ?",
        filter.where_sql
    );
    let mut query_params = params;
    query_params.push(Value::Integer(limit));
    query_params.push(Value::Integer(offset));
    let mut statement = connection.prepare(&query_sql)?;
    let uuids = statement
        .query_map(params_from_iter(query_params), |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list FTS search results")?;
    drop(statement);
    drop(connection);

    let entries = entries::list_entries_by_uuids_for_database(db_path, &uuids)?;
    Ok(SearchResponse {
        entries,
        total,
        limit,
        offset,
        mode: SearchMode::Keyword,
        used_fts: true,
        parsed_tokens: Vec::new(),
        warnings: Vec::new(),
    })
}

fn build_sql_filter(filters: &EntryFilters, tables: &HashSet<String>) -> SqlFilter {
    let mut conditions = Vec::new();
    let mut params = Vec::new();

    if let Some(since) = normalize_string(filters.since.as_deref()) {
        conditions.push("datetime(e.created_at) >= datetime(?)".to_string());
        params.push(Value::Text(since));
    }

    if let Some(until) = normalize_string(filters.until.as_deref()) {
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

    if let Some(location) = normalize_string(filters.location.as_deref()) {
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
            params.push(Value::Text(format!("%{}%", location.to_lowercase())));
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

    SqlFilter {
        where_sql: if conditions.is_empty() {
            String::new()
        } else {
            format!(" AND {}", conditions.join(" AND "))
        },
        params,
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

fn fts_query_from_keyword(keyword: &str) -> String {
    keyword
        .split_whitespace()
        .filter_map(|term| normalize_string(Some(term)))
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_filter_value(target: &mut Option<Vec<String>>, value: String) {
    let values = target.get_or_insert_with(Vec::new);
    if !values.iter().any(|item| item.eq_ignore_ascii_case(&value)) {
        values.push(value);
    }
}

fn normalize_vec(values: Option<Vec<String>>) -> Option<Vec<String>> {
    let normalized = values
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| normalize_string(Some(&value)))
        .collect::<Vec<_>>();
    (!normalized.is_empty()).then_some(normalized)
}

fn normalized_vec(values: Option<&[String]>) -> Vec<String> {
    values
        .unwrap_or_default()
        .iter()
        .filter_map(|value| normalize_string(Some(value)))
        .collect()
}

fn normalize_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn keyword_search_uses_fts_and_structured_tokens() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_search_fixture(temp_dir.path(), true);

        let response = search_entries_for_database(
            &db_path,
            SearchRequest {
                query: "rust tag:work NOT tag:home mood:focused after:2026-01-01 before:2026-01-31"
                    .to_string(),
                mode: Some(SearchMode::Keyword),
                include_hidden: Some(false),
                limit: Some(20),
                ..base_request()
            },
        )
        .expect("search");

        assert!(response.used_fts);
        assert_eq!(response.total, 1);
        assert_eq!(response.entries[0].uuid, "entry_rust");
        assert!(response
            .parsed_tokens
            .iter()
            .any(|token| token.kind == StructuredTokenKind::ExcludeTag && token.value == "home"));
    }

    #[test]
    fn keyword_search_falls_back_without_fts() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_search_fixture(temp_dir.path(), false);

        let response = search_entries_for_database(
            &db_path,
            SearchRequest {
                query: "capsule tag:work".to_string(),
                mode: Some(SearchMode::Keyword),
                limit: Some(20),
                ..base_request()
            },
        )
        .expect("search");

        assert!(!response.used_fts);
        assert_eq!(response.total, 1);
        assert_eq!(response.entries[0].uuid, "entry_capsule");
    }

    fn base_request() -> SearchRequest {
        SearchRequest {
            query: String::new(),
            mode: Some(SearchMode::Keyword),
            location: None,
            since: None,
            until: None,
            tags: None,
            exclude_tags: None,
            moods: None,
            exclude_moods: None,
            starred: None,
            pinned: None,
            hidden: None,
            include_hidden: None,
            has_images: None,
            limit: None,
            offset: None,
            sort: Some(EntrySort::Desc),
        }
    }

    fn create_search_fixture(path: &Path, with_fts: bool) -> std::path::PathBuf {
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
                INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format, title, summary, mood, hidden)
                VALUES
                    ('entry_rust', '2026-01-10 08:00', '2026-01-10 08:00', 'Rust search work', 'Rust search work', 'markdown', 'Rust', NULL, 'focused', 0),
                    ('entry_capsule', '2026-01-11 08:00', '2026-01-11 08:00', 'Capsule work note', 'Capsule work note', 'markdown', 'Capsule', NULL, 'calm', 0),
                    ('entry_hidden', '2026-01-12 08:00', '2026-01-12 08:00', 'Hidden rust', 'Hidden rust', 'markdown', 'Hidden', NULL, 'focused', 1);
                INSERT INTO tags (name) VALUES ('work'), ('home');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1), (2, 1), (3, 2);
                ",
            )
            .expect("fixture");

        if with_fts {
            connection
                .execute_batch(
                    "
                    CREATE VIRTUAL TABLE entries_fts USING fts5(text);
                    INSERT INTO entries_fts(rowid, text)
                    SELECT id, text_plain FROM entries;
                    ",
                )
                .expect("fts fixture");
        }
        drop(connection);

        db_path
    }
}
