//! Read-only memory metrics shared by Capsule clients.
//!
//! The desktop analytics screen has historically owned a much larger set of
//! charts.  This module is the small, deterministic surface needed by the cap
//! memory commands.  It deliberately opens the caller's database through the
//! same query-only connection as [`JournalReader`], applies Capsule's hidden
//! entry rule, and uses the stored local timestamp date (the first ten
//! characters of `created_at`) for day boundaries.

use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration as StdDuration, Instant},
};

use anyhow::{anyhow, Context, Result};
pub use chrono::NaiveDate;
use chrono::{Datelike, Duration};
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    db,
    models::{WritingCalendarDay, WritingCalendarResponse},
};

/// The first meaningful garden tier is also the daily glint threshold.  The
/// thresholds are intentionally word based, not XP/badge based.
pub const DAILY_MILESTONE_WORDS: i64 = 50;
/// A weekly bloom-sized writing total makes the weekly glint explainable.
pub const WEEKLY_MILESTONE_WORDS: i64 = 500;

pub fn local_today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

pub fn weekday_index(date: NaiveDate) -> u32 {
    date.weekday().num_days_from_monday()
}

/// Query controls for the bounded memory read surface.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryQuery {
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub since: Option<NaiveDate>,
    #[serde(default)]
    pub until: Option<NaiveDate>,
    /// Optional explicit result bound.  Aggregate helpers do not use this
    /// field; they stream their complete matching projection instead of
    /// silently truncating metrics.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Number of matching rows to skip when `limit` is set.
    #[serde(default)]
    pub offset: u64,
}

impl MemoryQuery {
    pub fn visible() -> Self {
        Self::default()
    }

    pub fn including_hidden(mut self) -> Self {
        self.include_hidden = true;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn between(mut self, since: NaiveDate, until: NaiveDate) -> Self {
        self.since = Some(since);
        self.until = Some(until);
        self
    }

    pub fn page(mut self, limit: u32, offset: u64) -> Self {
        self.limit = Some(limit);
        self.offset = offset;
        self
    }
}

/// A light entry projection for memory experiences.  `text` preserves the
/// authored value while `text_plain` is the Capsule display projection used
/// for word counts.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub id: i64,
    /// Legacy rows may have a blank UUID.  Preserve that fact instead of
    /// manufacturing an `entry_{id}` identity.
    pub uuid: Option<String>,
    pub created_at: String,
    pub date: String,
    pub text: String,
    pub text_plain: String,
    pub mood: Option<String>,
    pub hidden: bool,
}

impl MemoryEntry {
    pub fn word_count(&self) -> i64 {
        count_words(if self.text_plain.trim().is_empty() {
            &self.text
        } else {
            &self.text_plain
        })
    }

    pub fn local_date(&self) -> Option<NaiveDate> {
        parse_local_date(&self.created_at)
    }
}

/// One local calendar day.  Calendar responses include inactive days with
/// zeroes so a renderer can draw a complete month without inventing data.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDay {
    pub date: String,
    pub entry_count: i64,
    pub word_count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCalendar {
    pub month: String,
    pub year: i32,
    pub days_in_month: u32,
    pub days: Vec<ActivityDay>,
    pub active_days: i64,
    pub total_entries: i64,
    pub total_words: i64,
    pub max_entry_count: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatsPeriod {
    Week,
    Month,
    Year,
}

impl StatsPeriod {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStats {
    pub period: StatsPeriod,
    pub from: String,
    pub to: String,
    pub total_entries: i64,
    pub total_words: i64,
    pub active_days: i64,
    pub current_streak_days: i64,
    pub longest_streak_days: i64,
    pub days: Vec<ActivityDay>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GardenGrowth {
    Bare,
    Seed,
    Sprout,
    Leaf,
    Bloom,
}

impl GardenGrowth {
    pub const fn from_words(words: i64) -> Self {
        match words {
            0 => Self::Bare,
            1..=49 => Self::Seed,
            50..=199 => Self::Sprout,
            200..=499 => Self::Leaf,
            _ => Self::Bloom,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GardenDay {
    pub date: String,
    pub entry_count: i64,
    pub word_count: i64,
    pub growth: GardenGrowth,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryGarden {
    pub from: String,
    pub to: String,
    pub days: Vec<GardenDay>,
    pub total_entries: i64,
    pub total_words: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPage {
    pub entries: Vec<MemoryEntry>,
    pub total: i64,
    pub limit: u32,
    pub offset: u64,
    pub has_more: bool,
}

/// Counts used by cap's once-only milestone receipt hook.  The core remains
/// pure: it never stores glint state and never writes XP, badge, quest, or
/// journal rows.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneInputs {
    pub day: String,
    pub week_start: String,
    pub daily_entries: i64,
    pub daily_words: i64,
    pub weekly_entries: i64,
    pub weekly_words: i64,
    pub daily_threshold: i64,
    pub weekly_threshold: i64,
}

/// A post-commit snapshot paired with the exact committed entry contribution.
/// Subtracting that contribution gives a conservative `before` value, so an
/// old total that already exceeded a threshold can never be reported as a new
/// glint.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneCrossing {
    pub inputs: MilestoneInputs,
    pub committed_uuid: String,
    pub committed_words: i64,
    pub daily_before_words: i64,
    pub weekly_before_words: i64,
    pub daily_crossed: bool,
    pub weekly_crossed: bool,
}

/// Load memory candidates through a single bounded, read-only query.  A tag
/// filter is ignored only when the database has no tag relation; in that case
/// no candidate can match and an empty result is returned.
pub fn memory_entries_for_database(path: &Path, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
    let connection = db::open_read_only_connection(path)?;
    memory_entries_from_connection(&connection, query)
}

/// Same projection as [`memory_entries_for_database`] for callers that already
/// hold a read-only connection in a larger metric operation.
pub fn memory_entries_from_connection(
    connection: &Connection,
    query: MemoryQuery,
) -> Result<Vec<MemoryEntry>> {
    let (columns, where_sql, values) = memory_filter(connection, &query)?;

    let text_sql = if columns.has_text_plain {
        "COALESCE(e.text_plain, '')"
    } else {
        "''"
    };
    let uuid_sql = if columns.has_uuid {
        "COALESCE(e.uuid, '')"
    } else {
        "''"
    };
    let mood_sql = if columns.has_mood { "e.mood" } else { "NULL" };
    let hidden_sql = if columns.has_hidden {
        "COALESCE(e.hidden, 0)"
    } else {
        "0"
    };
    let page_sql = query.limit.map_or_else(String::new, |limit| {
        format!(
            " LIMIT {} OFFSET {}",
            i64::from(limit),
            i64::try_from(query.offset).unwrap_or(i64::MAX)
        )
    });
    let sql = format!(
        "SELECT e.id,
                {uuid_sql},
                e.created_at,
                e.text,
                {text_sql},
                {mood_sql},
                {hidden_sql}
         FROM entries e
         {where_sql}
         ORDER BY datetime(e.created_at) ASC, e.id ASC{page_sql}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values), |row| {
        let id = row.get::<_, i64>(0)?;
        let uuid = row.get::<_, String>(1)?;
        let created_at = row.get::<_, String>(2)?;
        let text = row.get::<_, String>(3)?;
        let text_plain = row.get::<_, String>(4)?;
        let mood = row.get::<_, Option<String>>(5)?;
        let hidden = row.get::<_, bool>(6)?;
        Ok(MemoryEntry {
            id,
            uuid: (!uuid.trim().is_empty()).then_some(uuid),
            date: created_at.get(..10).unwrap_or(&created_at).to_owned(),
            created_at,
            text,
            text_plain,
            mood,
            hidden,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load memory entries")
}

#[derive(Debug, Clone, Copy)]
struct MemoryColumns {
    has_text_plain: bool,
    has_uuid: bool,
    has_mood: bool,
    has_hidden: bool,
    has_tags: bool,
}

fn memory_filter(
    connection: &Connection,
    query: &MemoryQuery,
) -> Result<(MemoryColumns, String, Vec<Value>)> {
    let columns = table_columns(connection, "entries")?;
    if !columns.contains("id") || !columns.contains("created_at") || !columns.contains("text") {
        return Err(anyhow!(
            "database schema is missing required entries columns for memory metrics"
        ));
    }
    let columns = MemoryColumns {
        has_text_plain: columns.contains("text_plain"),
        has_uuid: columns.contains("uuid"),
        has_mood: columns.contains("mood"),
        has_hidden: columns.contains("hidden"),
        has_tags: table_exists(connection, "tags")? && table_exists(connection, "entry_tags")?,
    };
    if query.tag.is_some() && !columns.has_tags {
        return Ok((columns, "WHERE 0 = 1".to_string(), Vec::new()));
    }
    let hidden_sql = if columns.has_hidden {
        "COALESCE(e.hidden, 0)"
    } else {
        "0"
    };
    let mut conditions = Vec::<String>::new();
    let mut values = Vec::<Value>::new();
    if !query.include_hidden {
        conditions.push(format!("{hidden_sql} = 0"));
    }
    if let Some(since) = query.since {
        conditions.push("substr(e.created_at, 1, 10) >= ?".to_string());
        values.push(Value::Text(since.to_string()));
    }
    if let Some(until) = query.until {
        conditions.push("substr(e.created_at, 1, 10) <= ?".to_string());
        values.push(Value::Text(until.to_string()));
    }
    if let Some(tag) = query
        .tag
        .as_deref()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        conditions.push(
            "EXISTS (
                SELECT 1 FROM entry_tags et
                JOIN tags t ON t.id = et.tag_id
                WHERE et.entry_id = e.id AND lower(trim(t.name)) = lower(trim(?))
            )"
            .to_string(),
        );
        values.push(Value::Text(tag.to_owned()));
    }
    let where_sql = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    Ok((columns, where_sql, values))
}

#[derive(Debug, Clone)]
struct MemoryMetadata {
    id: i64,
}

/// Reservoir-sample bounded recall candidates using metadata only, then load
/// the selected bodies in one query.  This keeps recall work bounded for very
/// large journals without silently changing the caller's aggregate metrics.
pub fn recall_candidates_for_database(
    path: &Path,
    query: MemoryQuery,
    seed: u64,
    max_candidates: usize,
) -> Result<Vec<MemoryEntry>> {
    let connection = db::open_read_only_connection(path)?;
    recall_candidates_from_connection(&connection, query, seed, max_candidates)
}

fn recall_candidates_from_connection(
    connection: &Connection,
    query: MemoryQuery,
    mut seed: u64,
    max_candidates: usize,
) -> Result<Vec<MemoryEntry>> {
    let (columns, where_sql, values) = memory_filter(connection, &query)?;
    let sql = format!(
        "SELECT e.id
         FROM entries e {where_sql}
         ORDER BY datetime(e.created_at) ASC, e.id ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values), |row| {
        Ok(MemoryMetadata { id: row.get(0)? })
    })?;
    let capacity = max_candidates.max(1);
    let mut sample = Vec::with_capacity(capacity.min(64));
    let mut seen = 0_u64;
    for row in rows {
        let row = row?;
        seen = seen.saturating_add(1);
        if sample.len() < capacity {
            sample.push(row);
            continue;
        }
        // A tiny deterministic LCG avoids adding a random dependency while
        // retaining reservoir-sampling behavior over the complete journal.
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let index = seed % seen;
        if index < capacity as u64 {
            sample[index as usize] = row;
        }
    }
    let ids = sample.iter().map(|row| row.id).collect::<Vec<_>>();
    fetch_entries_by_ids(connection, &ids, columns)
}

fn fetch_entries_by_ids(
    connection: &Connection,
    ids: &[i64],
    columns: MemoryColumns,
) -> Result<Vec<MemoryEntry>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat_n("?", ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let uuid_sql = if columns.has_uuid {
        "COALESCE(e.uuid, '')"
    } else {
        "''"
    };
    let text_plain_sql = if columns.has_text_plain {
        "COALESCE(e.text_plain, '')"
    } else {
        "''"
    };
    let mood_sql = if columns.has_mood { "e.mood" } else { "NULL" };
    let hidden_sql = if columns.has_hidden {
        "COALESCE(e.hidden, 0)"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT e.id, {uuid_sql}, e.created_at, e.text, {text_plain_sql}, {mood_sql}, {hidden_sql}
         FROM entries e WHERE e.id IN ({placeholders}) ORDER BY e.id ASC"
    );
    let values = ids.iter().copied().map(Value::Integer).collect::<Vec<_>>();
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values), |row| {
        let id = row.get::<_, i64>(0)?;
        let uuid = row.get::<_, String>(1)?;
        let created_at = row.get::<_, String>(2)?;
        Ok(MemoryEntry {
            id,
            uuid: (!uuid.trim().is_empty()).then_some(uuid),
            date: created_at.get(..10).unwrap_or(&created_at).to_owned(),
            created_at,
            text: row.get(3)?,
            text_plain: row.get(4)?,
            mood: row.get(5)?,
            hidden: row.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load recall candidates")
}

pub fn memory_calendar_for_database(
    path: &Path,
    month: NaiveDate,
    include_hidden: bool,
) -> Result<MemoryCalendar> {
    let month_start = NaiveDate::from_ymd_opt(month.year(), month.month(), 1)
        .ok_or_else(|| anyhow!("invalid calendar month"))?;
    let next_month = shift_month(month_start, 1)?;
    let month_end = next_month - Duration::days(1);
    let summary = activity_summary_for_database(
        path,
        MemoryQuery {
            include_hidden,
            since: Some(month_start),
            until: Some(month_end),
            ..MemoryQuery::default()
        },
    )?;
    let buckets = summary.buckets;
    let mut days = Vec::with_capacity(month_end.day() as usize);
    for day in 1..=month_end.day() {
        let date = NaiveDate::from_ymd_opt(month_start.year(), month_start.month(), day)
            .expect("day in month");
        days.push(buckets.get(&date).cloned().unwrap_or_else(|| ActivityDay {
            date: date.to_string(),
            ..ActivityDay::default()
        }));
    }
    let total_entries = summary.total_entries;
    let total_words = summary.total_words;
    let max_entry_count = days.iter().map(|day| day.entry_count).max().unwrap_or(0);
    Ok(MemoryCalendar {
        month: format!("{:04}-{:02}", month_start.year(), month_start.month()),
        year: month_start.year(),
        days_in_month: month_end.day(),
        active_days: buckets.len() as i64,
        total_entries,
        total_words,
        max_entry_count,
        days,
    })
}

/// Shared desktop-compatible writing calendar projection.  The desktop
/// command adapter and headless cap now call this same read-only aggregation
/// so day totals, mood normalization, and optional image counts cannot drift.
pub fn get_writing_calendar(year: Option<i32>) -> Result<WritingCalendarResponse> {
    get_writing_calendar_for_database(&db::resolve_database_path(), year)
}

pub fn get_writing_calendar_for_database(
    path: &Path,
    year: Option<i32>,
) -> Result<WritingCalendarResponse> {
    let year = year.unwrap_or_else(|| chrono::Local::now().year());
    let start = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| anyhow!("calendar year is out of range"))?;
    let end = NaiveDate::from_ymd_opt(year, 12, 31)
        .ok_or_else(|| anyhow!("calendar year is out of range"))?;
    let connection = db::open_read_only_connection(path)?;
    let query = MemoryQuery {
        since: Some(start),
        until: Some(end),
        ..MemoryQuery::default()
    };
    let (columns, where_sql, values) = memory_filter(&connection, &query)?;
    let text_sql = if columns.has_text_plain {
        "COALESCE(NULLIF(e.text_plain, ''), e.text, '')"
    } else {
        "COALESCE(e.text, '')"
    };
    let mood_sql = if columns.has_mood { "e.mood" } else { "NULL" };
    let sql = format!(
        "SELECT substr(e.created_at, 1, 10), {text_sql}, {mood_sql}
         FROM entries e {where_sql}
         ORDER BY datetime(e.created_at) ASC, e.id ASC"
    );
    let mood_scores = crate::mood_sentiment::scores_for_database(&connection)?;
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params_from_iter(values))?;
    let mut by_date = BTreeMap::<String, WritingCalendarDay>::new();
    while let Some(row) = rows.next()? {
        let date = row.get::<_, String>(0)?;
        let text = row.get::<_, String>(1)?;
        let day = by_date
            .entry(date.clone())
            .or_insert_with(|| WritingCalendarDay {
                date,
                entry_count: 0,
                word_count: 0,
                image_count: 0,
                moods: Vec::new(),
                average_mood_sentiment: None,
                mood_sentiment_count: 0,
            });
        day.entry_count = day.entry_count.saturating_add(1);
        day.word_count = day.word_count.saturating_add(count_words(&text));
        if let Some(mood) = row
            .get::<_, Option<String>>(2)?
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        {
            if let Some(score) = crate::mood_sentiment::score_from_catalog(&mood_scores, &mood) {
                let current_sum =
                    day.average_mood_sentiment.unwrap_or(0.0) * day.mood_sentiment_count as f64;
                day.mood_sentiment_count = day.mood_sentiment_count.saturating_add(1);
                day.average_mood_sentiment =
                    Some((current_sum + score) / day.mood_sentiment_count as f64);
            }
            if !day
                .moods
                .iter()
                .any(|item| item.eq_ignore_ascii_case(&mood))
            {
                day.moods.push(mood);
            }
        }
    }
    let image_counts = image_counts_by_date_for_calendar(&connection, &query)?;
    for (date, count) in image_counts {
        if let Some(day) = by_date.get_mut(&date) {
            day.image_count = count;
        }
    }
    let days = by_date.into_values().collect::<Vec<_>>();
    let max_entry_count = days.iter().map(|day| day.entry_count).max().unwrap_or(0);
    Ok(WritingCalendarResponse {
        year,
        total_days: (end - start).num_days() + 1,
        active_days: days.len() as i64,
        max_entry_count,
        days,
        warnings: Vec::new(),
    })
}

fn image_counts_by_date_for_calendar(
    connection: &Connection,
    query: &MemoryQuery,
) -> Result<BTreeMap<String, i64>> {
    if !table_exists(connection, "plugin_entry_media")? || !table_exists(connection, "entries")? {
        return Ok(BTreeMap::new());
    }
    let media_columns = table_columns(connection, "plugin_entry_media")?;
    let entry_columns = table_columns(connection, "entries")?;
    if !media_columns.contains("entry_uuid") || !media_columns.contains("id") {
        return Ok(BTreeMap::new());
    }
    if !entry_columns.contains("uuid") {
        return Ok(BTreeMap::new());
    }
    let hidden_condition = if entry_columns.contains("hidden") {
        "COALESCE(e.hidden, 0) = 0"
    } else {
        "1 = 1"
    };
    let mut conditions = vec![hidden_condition.to_string()];
    let mut values = Vec::<Value>::new();
    if let Some(since) = query.since {
        conditions.push("substr(e.created_at, 1, 10) >= ?".to_string());
        values.push(Value::Text(since.to_string()));
    }
    if let Some(until) = query.until {
        conditions.push("substr(e.created_at, 1, 10) <= ?".to_string());
        values.push(Value::Text(until.to_string()));
    }
    let asset_join =
        if media_columns.contains("media_id") && table_exists(connection, "plugin_media_assets")? {
            let asset_columns = table_columns(connection, "plugin_media_assets")?;
            if asset_columns.contains("deleted_at") {
                "JOIN plugin_media_assets ma ON ma.id = em.media_id AND ma.deleted_at IS NULL"
            } else {
                "JOIN plugin_media_assets ma ON ma.id = em.media_id"
            }
        } else {
            ""
        };
    let sql = format!(
        "SELECT substr(e.created_at, 1, 10), COUNT(em.id)
         FROM plugin_entry_media em
         JOIN entries e ON e.uuid = em.entry_uuid
         {asset_join}
         WHERE {}
         GROUP BY substr(e.created_at, 1, 10)",
        conditions.join(" AND ")
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
}

pub fn memory_stats_for_database(
    path: &Path,
    period: StatsPeriod,
    today: NaiveDate,
    include_hidden: bool,
) -> Result<MemoryStats> {
    let (from, to) = period_bounds(period, today)?;
    let summary = activity_summary_for_database(
        path,
        MemoryQuery {
            include_hidden,
            since: Some(from),
            until: Some(to),
            ..MemoryQuery::default()
        },
    )?;
    let buckets = summary.buckets;
    let days = (0..=(to - from).num_days())
        .filter_map(|offset| from.checked_add_signed(Duration::days(offset)))
        .map(|date| {
            buckets.get(&date).cloned().unwrap_or_else(|| ActivityDay {
                date: date.to_string(),
                ..ActivityDay::default()
            })
        })
        .collect::<Vec<_>>();
    let active_dates = buckets.keys().copied().collect::<Vec<_>>();
    // Current streak is an all-time property.  Query only local dates outside
    // the selected period so a month/year boundary cannot truncate it.
    let all_time_dates = active_dates_for_database(path, include_hidden)?;
    Ok(MemoryStats {
        period,
        from: from.to_string(),
        to: to.to_string(),
        total_entries: summary.total_entries,
        total_words: summary.total_words,
        active_days: active_dates.len() as i64,
        current_streak_days: current_streak(&all_time_dates, today),
        longest_streak_days: longest_streak(&active_dates),
        days,
    })
}

pub fn memory_garden_for_database(
    path: &Path,
    today: NaiveDate,
    include_hidden: bool,
) -> Result<MemoryGarden> {
    let from = today - Duration::days(6);
    let summary = activity_summary_for_database(
        path,
        MemoryQuery {
            include_hidden,
            since: Some(from),
            until: Some(today),
            ..MemoryQuery::default()
        },
    )?;
    let buckets = summary.buckets;
    let days = (0..7)
        .map(|offset| from + Duration::days(offset))
        .map(|date| {
            let activity = buckets.get(&date).cloned().unwrap_or_else(|| ActivityDay {
                date: date.to_string(),
                ..ActivityDay::default()
            });
            GardenDay {
                date: activity.date,
                entry_count: activity.entry_count,
                word_count: activity.word_count,
                growth: GardenGrowth::from_words(activity.word_count),
            }
        })
        .collect::<Vec<_>>();
    Ok(MemoryGarden {
        from: from.to_string(),
        to: today.to_string(),
        total_entries: summary.total_entries,
        total_words: summary.total_words,
        days,
    })
}

pub fn milestone_inputs_for_database(
    path: &Path,
    today: NaiveDate,
    include_hidden: bool,
) -> Result<MilestoneInputs> {
    let week_start = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    let summary = activity_summary_for_database(
        path,
        MemoryQuery {
            include_hidden,
            since: Some(week_start),
            until: Some(today),
            ..MemoryQuery::default()
        },
    )?;
    Ok(milestone_inputs_from_summary(&summary, today))
}

fn milestone_inputs_from_summary(summary: &ActivitySummary, today: NaiveDate) -> MilestoneInputs {
    let week_start = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    let today_key = today;
    let today_activity = summary.buckets.get(&today_key);
    MilestoneInputs {
        day: today.to_string(),
        week_start: week_start.to_string(),
        daily_entries: today_activity.map_or(0, |day| day.entry_count),
        daily_words: today_activity.map_or(0, |day| day.word_count),
        weekly_entries: summary.total_entries,
        weekly_words: summary.total_words,
        daily_threshold: DAILY_MILESTONE_WORDS,
        weekly_threshold: WEEKLY_MILESTONE_WORDS,
    }
}

/// Compute a true threshold crossing for one committed entry.  The database
/// is read once after commit and the supplied UUID's word contribution is
/// removed for the `before` totals.  A missing UUID is an explicit warning
/// condition for callers rather than an invitation to guess.
pub fn milestone_crossing_for_database(
    path: &Path,
    today: NaiveDate,
    committed_uuid: &str,
    include_hidden: bool,
) -> Result<MilestoneCrossing> {
    milestone_crossing_for_database_with_timeout(
        path,
        today,
        committed_uuid,
        include_hidden,
        StdDuration::from_millis(200),
    )
}

pub fn milestone_crossing_for_database_with_timeout(
    path: &Path,
    today: NaiveDate,
    committed_uuid: &str,
    include_hidden: bool,
    timeout: StdDuration,
) -> Result<MilestoneCrossing> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| anyhow!("milestone timeout is out of range"))?;
    ensure_deadline(deadline)?;
    let connection = open_read_only_connection_until(path, deadline)?;
    ensure_deadline(deadline)?;
    install_query_deadline(&connection, deadline);
    // Keep lock contention from sleeping past the monotonic deadline.  The
    // progress handler below covers CPU-bound SQLite work; this bound covers
    // the separate busy-handler wait path.
    connection.busy_timeout(timeout.min(StdDuration::from_millis(25)))?;
    milestone_crossing_from_connection(
        &connection,
        today,
        committed_uuid,
        include_hidden,
        deadline,
        || {},
    )
}

fn milestone_crossing_from_connection<F: FnOnce()>(
    connection: &Connection,
    today: NaiveDate,
    committed_uuid: &str,
    include_hidden: bool,
    deadline: Instant,
    after_summary: F,
) -> Result<MilestoneCrossing> {
    let query = MemoryQuery {
        include_hidden,
        since: Some(today - Duration::days(i64::from(today.weekday().num_days_from_monday()))),
        until: Some(today),
        ..MemoryQuery::default()
    };
    connection.execute_batch("BEGIN DEFERRED")?;
    let summary = activity_summary_from_connection(connection, query.clone(), Some(deadline))?;
    ensure_deadline(deadline)?;
    after_summary();
    ensure_deadline(deadline)?;
    let committed = committed_entry_from_connection(connection, &query, committed_uuid, deadline)?;
    let inputs = milestone_inputs_from_summary(&summary, today);
    let committed_words = count_words(&committed.text_plain);
    let committed_date = parse_local_date(&committed.created_at);
    let daily_before_words = if committed_date == Some(today) {
        inputs.daily_words.saturating_sub(committed_words)
    } else {
        inputs.daily_words
    };
    let weekly_before_words = inputs.weekly_words.saturating_sub(committed_words);
    let result = MilestoneCrossing {
        daily_crossed: daily_before_words < inputs.daily_threshold
            && inputs.daily_words >= inputs.daily_threshold,
        weekly_crossed: weekly_before_words < inputs.weekly_threshold
            && inputs.weekly_words >= inputs.weekly_threshold,
        inputs,
        committed_uuid: committed_uuid.to_owned(),
        committed_words,
        daily_before_words,
        weekly_before_words,
    };
    connection.execute_batch("ROLLBACK")?;
    Ok(result)
}

/// Return earlier-year entries for the same local month/day, newest year
/// first.  The current year's entries are intentionally excluded.
pub fn on_this_day_for_database(
    path: &Path,
    date: NaiveDate,
    include_hidden: bool,
) -> Result<Vec<MemoryEntry>> {
    Ok(on_this_day_page_for_database(path, date, include_hidden, u32::MAX, 0)?.entries)
}

pub fn on_this_day_page_for_database(
    path: &Path,
    date: NaiveDate,
    include_hidden: bool,
    limit: u32,
    offset: u64,
) -> Result<MemoryPage> {
    let connection = db::open_read_only_connection(path)?;
    let query = MemoryQuery {
        include_hidden,
        ..MemoryQuery::default()
    };
    let (columns, where_sql, values) = memory_filter(&connection, &query)?;
    let day_predicate = "date(substr(e.created_at, 1, 10)) IS NOT NULL
         AND strftime('%m-%d', substr(e.created_at, 1, 10)) = ?
         AND CAST(substr(e.created_at, 1, 4) AS INTEGER) < ?";
    let month_day = format!("{:02}-{:02}", date.month(), date.day());
    let mut count_values = values.clone();
    count_values.push(Value::Text(month_day.clone()));
    count_values.push(Value::Integer(i64::from(date.year())));
    let count_sql = format!(
        "SELECT COUNT(*) FROM entries e {where_sql}
         {and_where} {day_predicate}",
        and_where = if where_sql.is_empty() { "WHERE" } else { "AND" },
    );
    let total = connection.query_row(&count_sql, params_from_iter(count_values), |row| {
        row.get::<_, i64>(0)
    })?;
    let mut page_values = values;
    page_values.push(Value::Text(month_day));
    page_values.push(Value::Integer(i64::from(date.year())));
    page_values.push(Value::Integer(i64::from(limit)));
    page_values.push(Value::Integer(i64::try_from(offset).unwrap_or(i64::MAX)));
    let sql = format!(
        "SELECT e.id
         FROM entries e {where_sql}
         {and_where} {day_predicate}
         ORDER BY substr(e.created_at, 1, 10) DESC, e.created_at DESC, e.id DESC
         LIMIT ? OFFSET ?",
        and_where = if where_sql.is_empty() { "WHERE" } else { "AND" },
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params_from_iter(page_values))?;
    let mut page_ids = Vec::with_capacity(limit as usize);
    while let Some(row) = rows.next()? {
        page_ids.push(row.get::<_, i64>(0)?);
    }
    let mut page = fetch_entries_by_ids(&connection, &page_ids, columns)?;
    let mut by_id = page
        .drain(..)
        .map(|entry| (entry.id, entry))
        .collect::<BTreeMap<_, _>>();
    let entries = page_ids
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect::<Vec<_>>();
    Ok(MemoryPage {
        entries,
        total,
        limit,
        offset,
        has_more: offset.saturating_add(limit as u64) < total as u64,
    })
}

/// Deterministically choose a recall candidate.  A previous UUID is avoided
/// whenever at least one other candidate exists; with a single candidate the
/// honest behavior is to return it again.
pub fn choose_recall<'a>(
    candidates: &'a [MemoryEntry],
    previous_uuid: Option<&str>,
    seed: u64,
) -> Option<&'a MemoryEntry> {
    if candidates.is_empty() {
        return None;
    }
    let mut choices = candidates
        .iter()
        .filter(|entry| previous_uuid != entry.uuid.as_deref())
        .collect::<Vec<_>>();
    if choices.is_empty() {
        choices = candidates.iter().collect();
    }
    let index = (seed % choices.len() as u64) as usize;
    choices.get(index).copied()
}

pub fn period_bounds(period: StatsPeriod, today: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
    let from = match period {
        StatsPeriod::Week => {
            today - Duration::days(i64::from(today.weekday().num_days_from_monday()))
        }
        StatsPeriod::Month => NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .ok_or_else(|| anyhow!("invalid current month"))?,
        StatsPeriod::Year => NaiveDate::from_ymd_opt(today.year(), 1, 1)
            .ok_or_else(|| anyhow!("invalid current year"))?,
    };
    Ok((from, today))
}

pub fn parse_local_date(value: &str) -> Option<NaiveDate> {
    value
        .get(..10)
        .and_then(|prefix| NaiveDate::parse_from_str(prefix, "%Y-%m-%d").ok())
}

pub fn count_words(text: &str) -> i64 {
    text.split_whitespace()
        .filter(|word| !word.is_empty())
        .count() as i64
}

pub fn current_streak(active_dates: &[NaiveDate], today: NaiveDate) -> i64 {
    let active = active_dates
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let latest = active.iter().copied().max();
    let Some(latest) = latest else { return 0 };
    if latest < today - Duration::days(1) || latest > today {
        return 0;
    }
    let mut count = 0;
    let mut cursor = latest;
    while active.contains(&cursor) {
        count += 1;
        cursor -= Duration::days(1);
    }
    count
}

pub fn longest_streak(active_dates: &[NaiveDate]) -> i64 {
    if active_dates.is_empty() {
        return 0;
    }
    let mut dates = active_dates.to_vec();
    dates.sort_unstable();
    dates.dedup();
    let mut longest = 1_i64;
    let mut run = 1_i64;
    for pair in dates.windows(2) {
        if pair[1] == pair[0] + Duration::days(1) {
            run += 1;
        } else {
            longest = longest.max(run);
            run = 1;
        }
    }
    longest.max(run)
}

#[derive(Debug, Default)]
struct ActivitySummary {
    buckets: BTreeMap<NaiveDate, ActivityDay>,
    total_entries: i64,
    total_words: i64,
}

fn activity_summary_for_database(path: &Path, query: MemoryQuery) -> Result<ActivitySummary> {
    let connection = db::open_read_only_connection(path)?;
    activity_summary_from_connection(&connection, query, None)
}

/// Stream only the date and text projection needed by aggregates.  The
/// complete matching set is counted; no implicit result cap is applied.
fn activity_summary_from_connection(
    connection: &Connection,
    query: MemoryQuery,
    deadline: Option<Instant>,
) -> Result<ActivitySummary> {
    ensure_optional_deadline(deadline)?;
    let (columns, where_sql, values) = memory_filter(connection, &query)?;
    let text_sql = if columns.has_text_plain {
        "COALESCE(NULLIF(e.text_plain, ''), e.text, '')"
    } else {
        "COALESCE(e.text, '')"
    };
    let sql = format!(
        "SELECT substr(e.created_at, 1, 10), {text_sql}
         FROM entries e {where_sql}
         ORDER BY datetime(e.created_at) ASC, e.id ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params_from_iter(values))?;
    let mut summary = ActivitySummary::default();
    while let Some(row) = rows.next()? {
        ensure_optional_deadline(deadline)?;
        let date_text = row.get::<_, String>(0)?;
        let text = row.get::<_, String>(1)?;
        summary.total_entries = summary.total_entries.saturating_add(1);
        summary.total_words = summary.total_words.saturating_add(count_words(&text));
        let Some(date) = parse_local_date(&date_text) else {
            continue;
        };
        let day = summary.buckets.entry(date).or_insert_with(|| ActivityDay {
            date: date.to_string(),
            ..ActivityDay::default()
        });
        day.entry_count = day.entry_count.saturating_add(1);
        day.word_count = day.word_count.saturating_add(count_words(&text));
    }
    ensure_optional_deadline(deadline)?;
    Ok(summary)
}

fn committed_entry_from_connection(
    connection: &Connection,
    query: &MemoryQuery,
    committed_uuid: &str,
    deadline: Instant,
) -> Result<CommittedEntry> {
    ensure_deadline(deadline)?;
    let (columns, where_sql, mut values) = memory_filter(connection, query)?;
    if !columns.has_uuid {
        return Err(anyhow!(
            "database schema has no UUID column for committed milestone verification"
        ));
    }
    let text_sql = if columns.has_text_plain {
        "COALESCE(NULLIF(e.text_plain, ''), e.text, '')"
    } else {
        "COALESCE(e.text, '')"
    };
    let uuid_condition = if where_sql.is_empty() {
        "WHERE e.uuid = ?"
    } else {
        "AND e.uuid = ?"
    };
    values.push(Value::Text(committed_uuid.to_owned()));
    let sql = format!(
        "SELECT e.created_at, {text_sql}
         FROM entries e {where_sql} {uuid_condition}
         ORDER BY e.id ASC LIMIT 1"
    );
    let mut statement = connection.prepare(&sql)?;
    let committed = statement
        .query_row(params_from_iter(values), |row| {
            Ok(CommittedEntry {
                created_at: row.get(0)?,
                text_plain: row.get(1)?,
            })
        })
        .optional()?
        .ok_or_else(|| {
            anyhow!("committed entry `{committed_uuid}` was not visible in the milestone snapshot")
        })?;
    ensure_deadline(deadline)?;
    Ok(committed)
}

#[derive(Debug)]
struct CommittedEntry {
    created_at: String,
    text_plain: String,
}

fn install_query_deadline(connection: &Connection, deadline: Instant) {
    connection.progress_handler(1_000, Some(move || Instant::now() >= deadline));
}

fn ensure_optional_deadline(deadline: Option<Instant>) -> Result<()> {
    if let Some(deadline) = deadline {
        ensure_deadline(deadline)?;
    }
    Ok(())
}

fn ensure_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(anyhow!("metric query exceeded its deadline"))
    } else {
        Ok(())
    }
}

fn open_read_only_connection_until(path: &Path, deadline: Instant) -> Result<Connection> {
    ensure_deadline(deadline)?;
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    connection.busy_timeout(remaining.min(StdDuration::from_millis(25)))?;
    connection.pragma_update(None, "query_only", "ON")?;
    ensure_deadline(deadline)?;
    Ok(connection)
}

fn active_dates_for_database(path: &Path, include_hidden: bool) -> Result<Vec<NaiveDate>> {
    let connection = db::open_read_only_connection(path)?;
    let columns = table_columns(&connection, "entries")?;
    if !columns.contains("created_at") {
        return Err(anyhow!(
            "database schema is missing entries.created_at for streak metrics"
        ));
    }
    let hidden_sql = if columns.contains("hidden") {
        "COALESCE(hidden, 0) = 0"
    } else {
        "1 = 1"
    };
    let where_sql = if include_hidden { "" } else { hidden_sql };
    let sql = format!(
        "SELECT substr(created_at, 1, 10) FROM entries {} ORDER BY created_at ASC",
        if where_sql.is_empty() {
            String::new()
        } else {
            format!("WHERE {where_sql}")
        }
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut dates = rows
        .filter_map(|row| row.ok().and_then(|value| parse_local_date(&value)))
        .collect::<Vec<_>>();
    dates.sort_unstable();
    dates.dedup();
    Ok(dates)
}

fn shift_month(value: NaiveDate, delta: i32) -> Result<NaiveDate> {
    let index = value.year() * 12 + value.month0() as i32 + delta;
    let year = index.div_euclid(12);
    let month = index.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| anyhow!("month is out of range"))
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn table_columns(
    connection: &Connection,
    table: &str,
) -> Result<std::collections::HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<std::collections::HashSet<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().expect("fixture dir");
        let path = dir.path().join("capsule.db");
        let connection = Connection::open(&path).expect("db");
        connection
            .execute_batch(
                "CREATE TABLE entries (
                    id INTEGER PRIMARY KEY,
                    uuid TEXT,
                    created_at TEXT NOT NULL,
                    text TEXT NOT NULL,
                    text_plain TEXT NOT NULL DEFAULT '',
                    mood TEXT,
                    hidden INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                CREATE TABLE entry_tags (entry_id INTEGER NOT NULL, tag_id INTEGER NOT NULL);
                INSERT INTO entries (id, uuid, created_at, text, text_plain, mood, hidden) VALUES
                    (1, 'old', '2025-02-28 23:59:59', 'one two', 'one two', 'calm', 0),
                    (2, 'leap', '2024-02-29 00:00:00', 'leap day words', 'leap day words', 'happy', 0),
                    (3, 'today', '2026-02-28 00:00:01', 'today', 'today', NULL, 0),
                    (4, 'hidden', '2026-02-28 01:00:00', 'hidden hidden', 'hidden hidden', NULL, 1);
                INSERT INTO tags VALUES (1, 'Life');
                INSERT INTO entry_tags VALUES (1, 1);",
            )
            .expect("fixture schema");
        (dir, path)
    }

    #[test]
    fn visible_projection_and_tag_filter_are_read_only() {
        let (_dir, path) = fixture();
        let before = std::fs::read(&path).expect("before");
        let entries = memory_entries_for_database(&path, MemoryQuery::visible().with_tag("life"))
            .expect("entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid.as_deref(), Some("old"));
        assert!(!entries[0].hidden);
        assert_eq!(std::fs::read(&path).expect("after"), before);
    }

    #[test]
    fn calendar_and_garden_fill_empty_days_and_use_thresholds() {
        let (_dir, path) = fixture();
        let calendar = memory_calendar_for_database(
            &path,
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            false,
        )
        .expect("calendar");
        assert_eq!(calendar.days.len(), 28);
        assert_eq!(calendar.active_days, 1);
        assert_eq!(calendar.days[27].entry_count, 1);
        let garden =
            memory_garden_for_database(&path, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(), false)
                .expect("garden");
        assert_eq!(garden.days.len(), 7);
        assert_eq!(garden.days[6].growth, GardenGrowth::Seed);
        assert_eq!(GardenGrowth::from_words(0), GardenGrowth::Bare);
        assert_eq!(GardenGrowth::from_words(50), GardenGrowth::Sprout);
        assert_eq!(GardenGrowth::from_words(200), GardenGrowth::Leaf);
        assert_eq!(GardenGrowth::from_words(500), GardenGrowth::Bloom);
    }

    #[test]
    fn shared_desktop_calendar_uses_same_visible_projection() {
        let (_dir, path) = fixture();
        let response = get_writing_calendar_for_database(&path, Some(2026)).expect("calendar");
        assert_eq!(response.year, 2026);
        assert_eq!(response.total_days, 365);
        assert_eq!(response.active_days, 1);
        assert_eq!(response.days[0].date, "2026-02-28");
        assert_eq!(response.days[0].entry_count, 1);
        assert_eq!(response.days[0].word_count, 1);
    }

    #[test]
    fn streak_requires_today_or_yesterday_and_recall_avoids_previous() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 28).unwrap();
        assert_eq!(current_streak(&[today - Duration::days(2)], today), 0);
        assert_eq!(
            current_streak(
                &[today - Duration::days(2), today - Duration::days(1)],
                today
            ),
            2
        );
        assert_eq!(
            current_streak(&[today - Duration::days(1), today], today),
            2
        );
        let month_boundary = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(
            current_streak(
                &[month_boundary - Duration::days(1), month_boundary],
                month_boundary
            ),
            2
        );
        let entries = vec![
            MemoryEntry {
                id: 1,
                uuid: Some("one".to_string()),
                created_at: "2026-02-28 00:00".to_string(),
                date: "2026-02-28".to_string(),
                text: "one".to_string(),
                text_plain: "one".to_string(),
                mood: None,
                hidden: false,
            },
            MemoryEntry {
                id: 2,
                uuid: Some("two".to_string()),
                created_at: "2026-02-28 01:00".to_string(),
                date: "2026-02-28".to_string(),
                text: "two".to_string(),
                text_plain: "two".to_string(),
                mood: None,
                hidden: false,
            },
        ];
        assert_eq!(
            choose_recall(&entries, Some("one"), 0)
                .unwrap()
                .uuid
                .as_deref(),
            Some("two")
        );
        assert_eq!(
            choose_recall(&entries, Some("two"), 0)
                .unwrap()
                .uuid
                .as_deref(),
            Some("one")
        );
        assert_eq!(
            choose_recall(&entries[..1], Some("one"), 0).unwrap().uuid,
            Some("one".to_string())
        );
    }

    #[test]
    fn on_this_day_excludes_current_year_and_handles_leap_day() {
        let (_dir, path) = fixture();
        let page = on_this_day_page_for_database(
            &path,
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(),
            false,
            1,
            0,
        )
        .expect("page");
        assert_eq!(page.total, 1);
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].uuid.as_deref(), Some("old"));
        assert!(!page.has_more);
        let leap =
            on_this_day_for_database(&path, NaiveDate::from_ymd_opt(2028, 2, 29).unwrap(), false)
                .expect("on this day");
        // A Feb-29 request in a leap year sees the imported 2024 row; the
        // helper accepts an injected date so tests do not depend on wall time.
        assert_eq!(leap.len(), 1);
        assert_eq!(leap[0].uuid.as_deref(), Some("leap"));
    }

    #[test]
    fn milestone_crossing_subtracts_the_committed_entry() {
        let (_dir, path) = fixture();
        let connection = Connection::open(&path).unwrap();
        let words = std::iter::repeat_n("word", 50)
            .collect::<Vec<_>>()
            .join(" ");
        connection
            .execute(
                "INSERT INTO entries (id, uuid, created_at, text, text_plain, hidden)
                 VALUES (5, 'crossing', '2026-02-28 02:00:00', ?1, ?1, 0)",
                [&words],
            )
            .unwrap();
        drop(connection);
        let crossing = milestone_crossing_for_database(
            &path,
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(),
            "crossing",
            false,
        )
        .unwrap();
        assert_eq!(crossing.committed_words, 50);
        assert_eq!(crossing.daily_before_words, 1);
        assert!(crossing.daily_crossed);
        assert!(!crossing.weekly_crossed);

        let old_total = milestone_crossing_for_database(
            &path,
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(),
            "today",
            false,
        )
        .unwrap();
        assert!(!old_total.daily_crossed);
    }

    #[test]
    fn milestone_snapshot_survives_mutation_between_summary_and_committed_lookup() {
        let (_dir, path) = fixture();
        let words = std::iter::repeat_n("word", 50)
            .collect::<Vec<_>>()
            .join(" ");
        let seed = Connection::open(&path).unwrap();
        seed.execute(
            "INSERT INTO entries (id, uuid, created_at, text, text_plain, hidden)
             VALUES (5, 'snapshot', '2026-02-28 02:00:00', ?1, ?1, 0)",
            [&words],
        )
        .unwrap();
        seed.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        drop(seed);

        let reader = Connection::open(&path).unwrap();
        reader.busy_timeout(StdDuration::from_millis(100)).unwrap();
        let mutator = Connection::open(&path).unwrap();
        mutator.busy_timeout(StdDuration::from_millis(100)).unwrap();
        let deadline = Instant::now() + StdDuration::from_secs(1);
        let crossing = milestone_crossing_from_connection(
            &reader,
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(),
            "snapshot",
            false,
            deadline,
            || {
                mutator
                    .execute(
                        "UPDATE entries SET text = 'changed', text_plain = 'changed'
                         WHERE uuid = 'snapshot'",
                        [],
                    )
                    .unwrap();
            },
        )
        .expect("snapshot crossing");
        assert_eq!(crossing.committed_words, 50);
        assert!(crossing.daily_crossed);
    }
}
