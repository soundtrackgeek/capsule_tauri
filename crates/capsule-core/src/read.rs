//! Bounded, read-only journal query services.
//!
//! `entries` still exposes the historical desktop helpers, some of which
//! repair legacy numeric IDs before reading.  This module is the explicit
//! headless boundary for clients such as `cap`: it binds one database path,
//! opens SQLite in read-only/query-only mode for every operation, and refuses
//! unsupported or repair-needed schemas instead of changing the journal.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    db::{self, CapabilityReport},
    entries,
    models::{
        Entry, EntryFilters, EntryListResponse, EntrySort, MoodUsage, SearchRequest,
        SearchResponse, TagUsage,
    },
    search,
};

pub const DEFAULT_LIMIT: i64 = 20;
pub const MAX_LIMIT: i64 = 200;

/// A bounded page with stable offset/total metadata.  The CLI maps this to
/// its own envelope page without copying SQL or entry projections.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadPage<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

/// Metadata pages include non-fatal capability warnings (for example, a
/// fixture without the optional tags table) while retaining the same bounds.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPage<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
    pub warnings: Vec<String>,
}

/// Query bounds shared by lists, discovery and metadata commands.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReadOptions {
    pub limit: i64,
    pub offset: i64,
    pub include_hidden: bool,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            offset: 0,
            include_hidden: false,
        }
    }
}

impl ReadOptions {
    pub fn new(limit: i64, offset: i64, include_hidden: bool) -> Self {
        Self {
            limit: limit.clamp(1, MAX_LIMIT),
            offset: offset.max(0),
            include_hidden,
        }
    }

    pub fn normalized(self) -> Self {
        Self::new(self.limit, self.offset, self.include_hidden)
    }
}

/// Explicit lookup key.  Numeric IDs and UUIDs are kept distinct so a
/// numeric-looking UUID can never silently select a different entry number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKey {
    Number(i64),
    Uuid(String),
}

impl EntryKey {
    pub fn parse(value: &str) -> Result<Self> {
        let value = value.trim();
        if value.is_empty() {
            return Err(anyhow!("entry identifier cannot be blank"));
        }
        if let Ok(number) = value.parse::<i64>() {
            if number > 0 {
                return Ok(Self::Number(number));
            }
            return Err(anyhow!("entry number must be positive"));
        }
        Ok(Self::Uuid(value.to_owned()))
    }
}

/// A path-bound reader.  No method re-resolves environment variables or
/// creates files; callers decide the path through Capsule's resolver first.
#[derive(Debug, Clone)]
pub struct JournalReader {
    db_path: PathBuf,
}

impl JournalReader {
    /// Bind and preflight an explicit existing database path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let db_path = path.into();
        if !db_path.is_file() {
            return Err(anyhow!(
                "explicit database path does not exist: {}",
                db_path.display()
            ));
        }
        let reader = Self { db_path };
        reader.ensure_readable_schema()?;
        Ok(reader)
    }

    pub fn database_path(&self) -> &Path {
        &self.db_path
    }

    /// Reinspect the bound path read-only.  This is useful for doctor output
    /// and detects a replaced/unsupported file before a query is attempted.
    pub fn capabilities(&self) -> Result<CapabilityReport> {
        db::capability_report_for_database(&self.db_path)
    }

    pub fn list(&self, options: ReadOptions) -> Result<ReadPage<Entry>> {
        self.list_with_filters(
            EntryFilters {
                include_hidden: Some(options.include_hidden),
                sort: Some(EntrySort::Desc),
                ..EntryFilters::default()
            },
            options,
        )
    }

    pub fn recent(&self, options: ReadOptions) -> Result<ReadPage<Entry>> {
        self.list_with_filters(
            EntryFilters {
                include_hidden: Some(options.include_hidden),
                sort: Some(EntrySort::Desc),
                ..EntryFilters::default()
            },
            options,
        )
    }

    /// Return entries on the host's current local date.  The boundary is
    /// injectable so fixture tests never depend on the wall clock.
    pub fn today(&self, options: ReadOptions) -> Result<ReadPage<Entry>> {
        let date = Local::now().date_naive();
        self.today_for_date(date, options)
    }

    pub fn today_for_date(&self, date: NaiveDate, options: ReadOptions) -> Result<ReadPage<Entry>> {
        let day = date.format("%Y-%m-%d").to_string();
        self.list_with_filters(
            EntryFilters {
                since: Some(format!("{day} 00:00:00")),
                until: Some(format!("{day} 23:59:59")),
                include_hidden: Some(options.include_hidden),
                sort: Some(EntrySort::Asc),
                ..EntryFilters::default()
            },
            options,
        )
    }

    /// Parse and validate a YYYY-MM-DD date before constructing SQL values.
    pub fn today_for_date_string(
        &self,
        date: &str,
        options: ReadOptions,
    ) -> Result<ReadPage<Entry>> {
        let date = NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d")
            .with_context(|| format!("invalid date `{date}`; expected YYYY-MM-DD"))?;
        self.today_for_date(date, options)
    }

    pub fn get(&self, identifier: &str, include_hidden: bool) -> Result<Entry> {
        let key = EntryKey::parse(identifier)?;
        self.get_by_key(key, include_hidden)
    }

    pub fn get_by_key(&self, key: EntryKey, include_hidden: bool) -> Result<Entry> {
        self.ensure_readable_schema()?;
        let connection = db::open_read_only_connection(&self.db_path)?;
        let (uuid, raw_uuid, id, hidden) = resolve_entry_key(&connection, &key)?;
        if hidden && !include_hidden {
            return Err(anyhow!(
                "entry `{}` is hidden; pass --include-hidden to read it",
                uuid
            ));
        }

        // UUID projection avoids the legacy helper's `uuid OR id` ambiguity.
        // A legacy row with a blank UUID is addressed by its numeric ID.
        let lookup = if raw_uuid.trim().is_empty() {
            id.to_string()
        } else {
            raw_uuid
        };
        entries::list_entries_by_uuids_read_only_for_database(&self.db_path, &[lookup.clone()])
            .and_then(|items| {
                items
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("entry not found: {lookup}"))
            })
    }

    pub fn search(&self, query: impl Into<String>, options: ReadOptions) -> Result<SearchResponse> {
        self.ensure_readable_schema()?;
        search::search_entries_for_database(
            &self.db_path,
            SearchRequest {
                query: query.into(),
                mode: None,
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
                include_hidden: Some(options.include_hidden),
                has_images: None,
                limit: Some(options.limit),
                offset: Some(options.offset),
                sort: Some(EntrySort::Desc),
            },
        )
    }

    pub fn tags(&self, options: ReadOptions) -> Result<MetadataPage<TagUsage>> {
        self.ensure_readable_schema()?;
        let connection = db::open_read_only_connection(&self.db_path)?;
        let tables = db::inspect_schema(&connection)?.detected_tables;
        let mut warnings = Vec::new();
        if !tables.iter().any(|table| table == "tags") {
            warnings.push("The tags table was not found.".to_string());
            return Ok(MetadataPage {
                items: Vec::new(),
                total: 0,
                limit: options.normalized().limit,
                offset: options.normalized().offset,
                has_more: false,
                warnings,
            });
        }

        let options = options.normalized();
        let (count_sql, list_sql, has_entry_filter) = if tables
            .iter()
            .any(|table| table == "entries")
            && tables.iter().any(|table| table == "entry_tags")
        {
            let hidden = if options.include_hidden {
                "1 = 1"
            } else {
                "COALESCE(e.hidden, 0) = 0"
            };
            (
                format!(
                    "SELECT COUNT(*) FROM (SELECT t.id FROM tags t JOIN entry_tags et ON et.tag_id = t.id JOIN entries e ON e.id = et.entry_id WHERE {hidden} GROUP BY t.id)"
                ),
                format!(
                    "SELECT t.id, t.name, COUNT(et.entry_id) AS entry_count
                     FROM tags t
                     JOIN entry_tags et ON et.tag_id = t.id
                     JOIN entries e ON e.id = et.entry_id
                     WHERE {hidden}
                     GROUP BY t.id, t.name
                     ORDER BY lower(t.name) ASC, t.id ASC
                     LIMIT ?1 OFFSET ?2"
                ),
                true,
            )
        } else {
            warnings
                .push("The entry_tags relation was not found; tag counts are zero.".to_string());
            (
                "SELECT COUNT(*) FROM tags".to_string(),
                "SELECT id, name, 0 AS entry_count FROM tags ORDER BY lower(name) ASC, id ASC LIMIT ?1 OFFSET ?2".to_string(),
                false,
            )
        };
        let total = connection.query_row(&count_sql, [], |row| row.get::<_, i64>(0))?;
        let mut statement = connection.prepare(&list_sql)?;
        let items = statement
            .query_map(params![options.limit, options.offset], |row| {
                Ok(TagUsage {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entry_count: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if has_entry_filter && items.is_empty() && total > 0 {
            warnings.push("No visible tags were present on this page.".to_string());
        }
        Ok(metadata_page(items, total, options, warnings))
    }

    pub fn moods(&self, options: ReadOptions) -> Result<MetadataPage<MoodUsage>> {
        self.ensure_readable_schema()?;
        let connection = db::open_read_only_connection(&self.db_path)?;
        let tables = db::inspect_schema(&connection)?.detected_tables;
        let options = options.normalized();
        let mut warnings = Vec::new();
        let mut moods = BTreeMap::<String, (i64, Option<f64>)>::new();

        if tables.iter().any(|table| table == "entries") {
            let hidden = if options.include_hidden {
                "1 = 1"
            } else {
                "COALESCE(hidden, 0) = 0"
            };
            let mut statement = connection.prepare(&format!(
                "SELECT lower(trim(mood)), COUNT(*)
                 FROM entries
                 WHERE mood IS NOT NULL AND trim(mood) != '' AND {hidden}
                 GROUP BY lower(trim(mood))
                 ORDER BY lower(trim(mood)) ASC"
            ))?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (name, count) = row?;
                moods.entry(name).or_insert((count, None)).0 = count;
            }
        } else {
            warnings.push("The entries table was not found.".to_string());
        }

        if tables.iter().any(|table| table == "mood_catalog") {
            let mut statement = connection.prepare(
                "SELECT lower(trim(name)), sentiment_score FROM mood_catalog ORDER BY lower(name) ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<f64>>(1)?))
            })?;
            for row in rows {
                let (name, score) = row?;
                if !name.trim().is_empty() {
                    moods.entry(name).or_insert((0, score)).1 = score;
                }
            }
        }

        let total = i64::try_from(moods.len()).unwrap_or(i64::MAX);
        let items = moods
            .into_iter()
            .skip(usize::try_from(options.offset).unwrap_or(usize::MAX))
            .take(usize::try_from(options.limit).unwrap_or(0))
            .map(|(name, (entry_count, sentiment_score))| MoodUsage {
                label: labelize(&name),
                name,
                entry_count,
                sentiment_score,
            })
            .collect::<Vec<_>>();
        Ok(metadata_page(items, total, options, warnings))
    }

    /// Load the shared, safe context policy for this bound database.  The
    /// caller supplies the resolver's exact config path, so this method never
    /// re-resolves process environment or sibling candidates.
    pub fn context_settings(
        &self,
        config_path: Option<&Path>,
    ) -> Result<crate::location::ContextSettings> {
        self.ensure_readable_schema()?;
        crate::location::load_context_settings(&self.db_path, config_path)
    }

    fn list_with_filters(
        &self,
        mut filters: EntryFilters,
        options: ReadOptions,
    ) -> Result<ReadPage<Entry>> {
        self.ensure_readable_schema()?;
        let options = options.normalized();
        filters.limit = Some(options.limit);
        filters.offset = Some(options.offset);
        filters.include_hidden = Some(options.include_hidden);
        let response = entries::list_entries_read_only_for_database(&self.db_path, filters)?;
        Ok(entry_page(response, options))
    }

    fn ensure_readable_schema(&self) -> Result<()> {
        let connection = db::open_read_only_connection(&self.db_path)?;
        let report = db::inspect_capabilities(&connection)?;
        if !report.schema.supports_read {
            return Err(anyhow!(
                "database schema is unsupported for read-only journal queries: {}",
                report
                    .schema
                    .missing_required_columns
                    .iter()
                    .map(|column| format!("entries.{column}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        entries::ensure_read_only_for_database(&self.db_path)
    }
}

fn entry_page(response: EntryListResponse, options: ReadOptions) -> ReadPage<Entry> {
    let total = response.total.max(0);
    let has_more = response
        .offset
        .saturating_add(response.entries.len() as i64)
        < total;
    ReadPage {
        items: response.entries,
        total,
        limit: options.limit,
        offset: options.offset,
        has_more,
    }
}

fn metadata_page<T>(
    items: Vec<T>,
    total: i64,
    options: ReadOptions,
    warnings: Vec<String>,
) -> MetadataPage<T> {
    MetadataPage {
        has_more: options.offset.saturating_add(items.len() as i64) < total,
        items,
        total,
        limit: options.limit,
        offset: options.offset,
        warnings,
    }
}

fn resolve_entry_key(
    connection: &Connection,
    key: &EntryKey,
) -> Result<(String, String, i64, bool)> {
    let (uuid_match, id_match) = match key {
        EntryKey::Uuid(value) => {
            let uuid_match = connection
                .query_row(
                    "SELECT COALESCE(uuid, ''), id, COALESCE(hidden, 0) FROM entries WHERE uuid = ?1 LIMIT 1",
                    [value],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, bool>(2)?)),
                )
                .optional()?;
            (uuid_match, None)
        }
        EntryKey::Number(number) => {
            // A numeric-looking UUID is legal in legacy databases. Resolve
            // both forms before selecting so a collision is an actionable
            // ambiguity instead of whichever row SQLite happens to return.
            let uuid_match = connection
                .query_row(
                    "SELECT COALESCE(uuid, ''), id, COALESCE(hidden, 0) FROM entries WHERE uuid = ?1 LIMIT 1",
                    [number.to_string()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, bool>(2)?)),
                )
                .optional()?;
            let id_match = connection
                .query_row(
                    "SELECT COALESCE(uuid, ''), id, COALESCE(hidden, 0) FROM entries WHERE id = ?1 LIMIT 1",
                    [number],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, bool>(2)?)),
                )
                .optional()?;
            (uuid_match, id_match)
        }
    };

    if let (Some(uuid), Some(id)) = (&uuid_match, &id_match) {
        if uuid.1 != id.1 {
            return Err(anyhow!(
                "entry identifier is ambiguous: it matches UUID `{}` and entry number {}; use an explicit UUID or number",
                uuid.0,
                id.1
            ));
        }
    }
    let selected = uuid_match.or(id_match).ok_or_else(|| match key {
        EntryKey::Uuid(value) => anyhow!("entry not found: {value}"),
        EntryKey::Number(value) => anyhow!("entry number not found: {value}"),
    })?;
    let display_uuid = if selected.0.trim().is_empty() {
        format!("entry_{}", selected.1)
    } else {
        selected.0.clone()
    };
    Ok((display_uuid, selected.0, selected.1, selected.2))
}

fn labelize(value: &str) -> String {
    value
        .split_whitespace()
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn fixture(include_fts: bool) -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("capsule.db");
        let connection = Connection::open(&db_path).expect("open");
        connection
            .execute_batch(
                "
                CREATE TABLE entries (
                    id INTEGER PRIMARY KEY,
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
                CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
                CREATE TABLE entry_tags (entry_id INTEGER NOT NULL, tag_id INTEGER NOT NULL);
                INSERT INTO entries (id, uuid, created_at, updated_at, text, text_plain, content_format, title, mood, hidden)
                VALUES
                  (1, 'entry-one', '2026-09-14 08:00', '2026-09-14 08:00', 'Rust [day]', 'Rust [day]', 'plain', 'One', 'calm', 0),
                  (2, 'entry-two', '2026-09-14 09:00', '2026-09-14 09:00', 'Hidden note', 'Hidden note', 'plain', 'Two', 'secret', 1),
                  (3, '123', '2026-09-14 10:00', '2026-09-14 10:00', 'Numeric uuid', 'Numeric uuid', 'plain', 'Three', 'calm', 0);
                INSERT INTO tags (id, name) VALUES (1, 'work'), (2, 'private');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1), (2, 2);
                ",
            )
            .expect("fixture schema");
        if include_fts {
            connection
                .execute_batch(
                    "CREATE VIRTUAL TABLE entries_fts USING fts5(text); INSERT INTO entries_fts(rowid, text) SELECT id, text_plain FROM entries;",
                )
                .expect("fts");
        }
        drop(connection);
        (dir, db_path)
    }

    #[test]
    fn bounded_lists_hide_by_default_and_today_boundary_is_injected() {
        let (_dir, db_path) = fixture(false);
        let reader = JournalReader::open(&db_path).expect("reader");
        let page = reader
            .today_for_date_string("2026-09-14", ReadOptions::new(20, 0, false))
            .expect("today");
        assert_eq!(page.total, 2);
        assert_eq!(page.items[0].uuid, "entry-one");
        let hidden = reader.get("2", false);
        assert!(hidden.is_err());
        assert_eq!(reader.get("2", true).expect("hidden").uuid, "entry-two");
    }

    #[test]
    fn numeric_uuid_is_not_confused_with_a_number_lookup() {
        let (_dir, db_path) = fixture(false);
        let reader = JournalReader::open(&db_path).expect("reader");
        assert_eq!(reader.get("123", false).expect("uuid").uuid, "123");
        assert_eq!(
            reader
                .get_by_key(EntryKey::Number(3), false)
                .expect("number")
                .uuid,
            "123"
        );
    }

    #[test]
    fn metadata_pages_are_bounded_and_exclude_hidden_rows() {
        let (_dir, db_path) = fixture(false);
        let reader = JournalReader::open(&db_path).expect("reader");
        let tags = reader.tags(ReadOptions::new(1, 0, false)).expect("tags");
        assert_eq!(tags.items.len(), 1);
        assert_eq!(tags.items[0].name, "work");
        let moods = reader.moods(ReadOptions::default()).expect("moods");
        assert_eq!(moods.total, 1);
        assert_eq!(moods.items[0].name, "calm");
    }

    #[test]
    fn context_report_is_safe_and_never_reads_network() {
        let (dir, db_path) = fixture(false);
        let config = dir.path().join("config.json");
        std::fs::write(
            &config,
            br#"{"location.auto_capture":false,"location.default_location_name":"Bergen","token":"secret"}"#,
        )
        .expect("config");
        let settings = JournalReader::open(&db_path)
            .expect("reader")
            .context_settings(Some(&config))
            .expect("settings");
        assert!(!settings.policy.auto_capture);
        assert_eq!(
            settings.policy.default_location_name.as_deref(),
            Some("Bergen")
        );
        let serialized = serde_json::to_string(&settings.policy).expect("json");
        assert!(!serialized.contains("secret"));
    }
}
