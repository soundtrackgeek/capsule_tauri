use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::{params_from_iter, types::Value, Connection};

use crate::{
    db,
    models::{
        AnalyticsBreakdownItem, AnalyticsDailyTrendPoint, AnalyticsHourPoint, AnalyticsOverview,
        AnalyticsPeriodRequest, AnalyticsResponse, AnalyticsTrendPoint, AnalyticsWeekdayPoint,
        AnalyticsWritingWindow, AnalyticsWritingWindowDay, AnalyticsWritingWindowLongestDay,
        AnalyticsWritingWindowSummary, WordCount, WritingCalendarDay, WritingCalendarResponse,
    },
    mood_sentiment,
};

const TOP_LIMIT: usize = 12;

#[derive(Debug, Clone)]
struct EntryStatsRow {
    created_at: String,
    date: String,
    text: String,
    mood: Option<String>,
}

pub fn get_analytics(input: Option<AnalyticsPeriodRequest>) -> Result<AnalyticsResponse> {
    get_analytics_for_database(&db::resolve_database_path(), input.unwrap_or_default())
}

pub(crate) fn get_analytics_for_database(
    db_path: &Path,
    input: AnalyticsPeriodRequest,
) -> Result<AnalyticsResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    if !table_exists(&connection, "entries")? {
        return Err(anyhow!(
            "The active database does not contain an entries table."
        ));
    }

    let rows = load_entry_rows(&connection, &input)?;
    let total_entries = rows.len() as i64;
    let total_words = rows
        .iter()
        .map(|row| word_count(&row.text) as i64)
        .sum::<i64>();
    let active_dates = rows
        .iter()
        .filter_map(|row| parse_date(&row.date))
        .collect::<HashSet<_>>();
    let (longest_streak_days, current_streak_days) = streaks(&active_dates);
    let (total_images, entries_with_images) = image_counts(&connection, &input)?;
    let entries_with_location = location_count(&connection, &input)?;
    let mood_sentiment_summary = mood_sentiment_summary(&rows);

    Ok(AnalyticsResponse {
        overview: AnalyticsOverview {
            total_entries,
            total_words,
            average_words: if total_entries == 0 {
                0.0
            } else {
                total_words as f64 / total_entries as f64
            },
            average_mood_sentiment: mood_sentiment_summary.average(),
            mood_sentiment_count: mood_sentiment_summary.count,
            total_images,
            entries_with_images,
            entries_with_location,
            longest_streak_days,
            current_streak_days,
        },
        monthly_trend: monthly_trend(&rows),
        daily_trend: daily_trend(&rows),
        hourly_trend: hourly_trend(&rows),
        weekday_trend: weekday_trend(&rows),
        writing_window: writing_window(&rows),
        location_activity: location_activity(&connection, &input)?,
        mood_breakdown: mood_breakdown(&rows),
        tag_breakdown: tag_breakdown(&connection, &input)?,
        location_breakdown: location_breakdown(&connection, &input)?,
        weather_breakdown: weather_breakdown(&connection, &input)?,
        top_words: top_words(&rows),
        warnings: Vec::new(),
    })
}

pub fn get_writing_calendar(year: Option<i32>) -> Result<WritingCalendarResponse> {
    get_writing_calendar_for_database(&db::resolve_database_path(), year)
}

pub(crate) fn get_writing_calendar_for_database(
    db_path: &Path,
    year: Option<i32>,
) -> Result<WritingCalendarResponse> {
    let year = year.unwrap_or_else(|| Local::now().year());
    let since = format!("{year}-01-01");
    let until = format!("{year}-12-31 23:59:59");
    let period = AnalyticsPeriodRequest {
        since: Some(since),
        until: Some(until),
    };

    let connection = db::open_read_only_connection(db_path)?;
    if !table_exists(&connection, "entries")? {
        return Err(anyhow!(
            "The active database does not contain an entries table."
        ));
    }
    let rows = load_entry_rows(&connection, &period)?;
    let image_counts = image_counts_by_date(&connection, &period)?;
    let mut by_date: BTreeMap<String, WritingCalendarDay> = BTreeMap::new();

    for row in rows {
        let date = row.date.get(0..10).unwrap_or(&row.date).to_string();
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
        day.entry_count += 1;
        day.word_count += word_count(&row.text) as i64;
        if let Some(mood) = row.mood.and_then(|value| normalize_string(Some(&value))) {
            if let Some(score) = mood_sentiment::score_for_mood(&mood) {
                let current_sum =
                    day.average_mood_sentiment.unwrap_or(0.0) * day.mood_sentiment_count as f64;
                day.mood_sentiment_count += 1;
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

    for (date, count) in image_counts {
        if let Some(day) = by_date.get_mut(&date) {
            day.image_count = count;
        }
    }

    let days = by_date.into_values().collect::<Vec<_>>();
    let max_entry_count = days.iter().map(|day| day.entry_count).max().unwrap_or(0);

    Ok(WritingCalendarResponse {
        year,
        total_days: days_in_year(year),
        active_days: days.len() as i64,
        max_entry_count,
        days,
        warnings: Vec::new(),
    })
}

fn load_entry_rows(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
) -> Result<Vec<EntryStatsRow>> {
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT e.created_at,
                substr(e.created_at, 1, 10) AS entry_date,
                COALESCE(NULLIF(e.text_plain, ''), e.text, '') AS text_value,
                e.mood
         FROM entries e
         {}
         ORDER BY datetime(e.created_at) ASC, e.id ASC",
        filter.where_sql
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(filter.params), |row| {
        Ok(EntryStatsRow {
            created_at: row.get(0)?,
            date: row.get(1)?,
            text: row.get(2)?,
            mood: row.get(3)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load analytics entry rows")
}

fn image_counts(connection: &Connection, period: &AnalyticsPeriodRequest) -> Result<(i64, i64)> {
    if !table_exists(connection, "plugin_entry_media")? {
        return Ok((0, 0));
    }
    let asset_join = if table_exists(connection, "plugin_media_assets")? {
        "JOIN plugin_media_assets ma ON ma.id = em.media_id AND ma.deleted_at IS NULL"
    } else {
        ""
    };
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT COUNT(em.id), COUNT(DISTINCT em.entry_uuid)
         FROM plugin_entry_media em
         JOIN entries e ON e.uuid = em.entry_uuid
         {asset_join}
         {}",
        filter.where_sql
    );
    connection
        .query_row(&sql, params_from_iter(filter.params), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .context("failed to count image attachments")
}

fn image_counts_by_date(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
) -> Result<HashMap<String, i64>> {
    if !table_exists(connection, "plugin_entry_media")? {
        return Ok(HashMap::new());
    }
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT substr(e.created_at, 1, 10), COUNT(em.id)
         FROM plugin_entry_media em
         JOIN entries e ON e.uuid = em.entry_uuid
         {}
         GROUP BY substr(e.created_at, 1, 10)",
        filter.where_sql
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(filter.params), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .collect())
}

fn location_count(connection: &Connection, period: &AnalyticsPeriodRequest) -> Result<i64> {
    if !table_exists(connection, "plugin_entry_locations")? {
        return Ok(0);
    }
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT COUNT(*)
         FROM plugin_entry_locations pel
         JOIN entries e ON e.uuid = pel.entry_uuid
         {}",
        filter.where_sql
    );
    connection
        .query_row(&sql, params_from_iter(filter.params), |row| row.get(0))
        .context("failed to count location rows")
}

fn location_breakdown(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
) -> Result<Vec<AnalyticsBreakdownItem>> {
    if !table_exists(connection, "plugin_entry_locations")? {
        return Ok(Vec::new());
    }
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT COALESCE(NULLIF(trim(pel.place_name), ''), 'Unknown location') AS label,
                COUNT(*) AS count_value
         FROM plugin_entry_locations pel
         JOIN entries e ON e.uuid = pel.entry_uuid
         {}
         GROUP BY label
         ORDER BY count_value DESC, lower(label) ASC
         LIMIT {}",
        filter.where_sql, TOP_LIMIT
    );
    breakdown_query(connection, &sql, filter.params)
}

fn weather_breakdown(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
) -> Result<Vec<AnalyticsBreakdownItem>> {
    if !table_exists(connection, "plugin_entry_locations")? {
        return Ok(Vec::new());
    }
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT COALESCE(NULLIF(trim(pel.weather_condition), ''), 'Unknown weather') AS label,
                COUNT(*) AS count_value
         FROM plugin_entry_locations pel
         JOIN entries e ON e.uuid = pel.entry_uuid
         {}
         GROUP BY label
         ORDER BY count_value DESC, lower(label) ASC
         LIMIT {}",
        filter.where_sql, TOP_LIMIT
    );
    breakdown_query(connection, &sql, filter.params)
}

fn tag_breakdown(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
) -> Result<Vec<AnalyticsBreakdownItem>> {
    if !table_exists(connection, "tags")? || !table_exists(connection, "entry_tags")? {
        return Ok(Vec::new());
    }
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT t.name AS label, COUNT(*) AS count_value
         FROM entry_tags et
         JOIN tags t ON t.id = et.tag_id
         JOIN entries e ON e.id = et.entry_id
         {}
         GROUP BY t.name
         ORDER BY count_value DESC, lower(t.name) ASC
         LIMIT {}",
        filter.where_sql, TOP_LIMIT
    );
    breakdown_query(connection, &sql, filter.params)
}

fn breakdown_query(
    connection: &Connection,
    sql: &str,
    params: Vec<Value>,
) -> Result<Vec<AnalyticsBreakdownItem>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params_from_iter(params), |row| {
        Ok(AnalyticsBreakdownItem {
            label: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load analytics breakdown")
}

fn monthly_trend(rows: &[EntryStatsRow]) -> Vec<AnalyticsTrendPoint> {
    let mut by_month: BTreeMap<String, TrendAccumulator> = BTreeMap::new();
    for row in rows {
        let period = row.date.get(0..7).unwrap_or(&row.date).to_string();
        let point = by_month.entry(period.clone()).or_insert(TrendAccumulator {
            period,
            entry_count: 0,
            word_count: 0,
            mood_sentiment_sum: 0.0,
            mood_sentiment_count: 0,
        });
        point.entry_count += 1;
        point.word_count += word_count(&row.text) as i64;
        if let Some(score) = row.mood.as_deref().and_then(mood_sentiment::score_for_mood) {
            point.mood_sentiment_sum += score;
            point.mood_sentiment_count += 1;
        }
    }
    by_month
        .into_values()
        .map(TrendAccumulator::into_point)
        .collect()
}

fn daily_trend(rows: &[EntryStatsRow]) -> Vec<AnalyticsDailyTrendPoint> {
    let mut by_date: BTreeMap<String, AnalyticsDailyTrendPoint> = BTreeMap::new();
    for row in rows {
        let point = by_date
            .entry(row.date.clone())
            .or_insert_with(|| AnalyticsDailyTrendPoint {
                date: row.date.clone(),
                entry_count: 0,
                word_count: 0,
            });
        point.entry_count += 1;
        point.word_count += word_count(&row.text) as i64;
    }
    by_date.into_values().collect()
}

fn hourly_trend(rows: &[EntryStatsRow]) -> Vec<AnalyticsHourPoint> {
    let mut hours = (0..24)
        .map(|hour| AnalyticsHourPoint {
            hour,
            label: format!("{hour:02}:00"),
            entry_count: 0,
            word_count: 0,
        })
        .collect::<Vec<_>>();

    for row in rows {
        if let Some(hour) = parse_hour(&row.created_at) {
            if let Some(point) = hours.get_mut(hour as usize) {
                point.entry_count += 1;
                point.word_count += word_count(&row.text) as i64;
            }
        }
    }

    hours
}

fn weekday_trend(rows: &[EntryStatsRow]) -> Vec<AnalyticsWeekdayPoint> {
    let mut days = weekday_labels()
        .iter()
        .map(|(day_num, label, short_label)| AnalyticsWeekdayPoint {
            day_num: *day_num,
            label: (*label).to_string(),
            short_label: (*short_label).to_string(),
            entry_count: 0,
            word_count: 0,
        })
        .collect::<Vec<_>>();

    for row in rows {
        if let Some(date) = parse_date(&row.date) {
            let day_num = weekday_day_num(date);
            if let Some(point) = days.iter_mut().find(|day| day.day_num == day_num) {
                point.entry_count += 1;
                point.word_count += word_count(&row.text) as i64;
            }
        }
    }

    days
}

fn writing_window(rows: &[EntryStatsRow]) -> AnalyticsWritingWindow {
    let mut by_date: BTreeMap<String, WritingWindowAccumulator> = BTreeMap::new();
    for row in rows {
        if let Some(minutes) = parse_minutes_since_midnight(&row.created_at) {
            let point =
                by_date
                    .entry(row.date.clone())
                    .or_insert_with(|| WritingWindowAccumulator {
                        date: row.date.clone(),
                        first_minutes: minutes,
                        last_minutes: minutes,
                        entry_count: 0,
                    });
            point.first_minutes = point.first_minutes.min(minutes);
            point.last_minutes = point.last_minutes.max(minutes);
            point.entry_count += 1;
        }
    }

    let days = by_date
        .into_values()
        .map(WritingWindowAccumulator::into_day)
        .collect::<Vec<_>>();
    let summary = writing_window_summary(&days);
    AnalyticsWritingWindow { days, summary }
}

fn location_activity(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
) -> Result<Vec<AnalyticsBreakdownItem>> {
    if !table_exists(connection, "plugin_entry_locations")? {
        return Ok(Vec::new());
    }

    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT COALESCE(
                    NULLIF(trim(pel.place_name), ''),
                    printf('%.4f, %.4f', pel.latitude, pel.longitude),
                    'Unknown location'
                ) AS location_name
         FROM plugin_entry_locations pel
         JOIN entries e ON e.uuid = pel.entry_uuid
         {}
         ORDER BY lower(location_name) ASC",
        filter.where_sql
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(filter.params), |row| {
        row.get::<_, String>(0)
    })?;

    let mut buckets: HashMap<String, LocationActivityBucket> = HashMap::new();
    for label in rows.collect::<rusqlite::Result<Vec<_>>>()? {
        let normalized = label.trim().to_lowercase();
        let bucket = buckets.entry(normalized).or_default();
        bucket.count += 1;
        *bucket.labels.entry(label).or_insert(0) += 1;
    }

    let mut values = buckets
        .into_values()
        .map(LocationActivityBucket::into_breakdown)
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
            .then_with(|| left.label.cmp(&right.label))
    });
    Ok(values)
}

#[derive(Debug, Clone)]
struct TrendAccumulator {
    period: String,
    entry_count: i64,
    word_count: i64,
    mood_sentiment_sum: f64,
    mood_sentiment_count: i64,
}

impl TrendAccumulator {
    fn into_point(self) -> AnalyticsTrendPoint {
        AnalyticsTrendPoint {
            period: self.period,
            entry_count: self.entry_count,
            word_count: self.word_count,
            average_mood_sentiment: if self.mood_sentiment_count == 0 {
                None
            } else {
                Some(self.mood_sentiment_sum / self.mood_sentiment_count as f64)
            },
            mood_sentiment_count: self.mood_sentiment_count,
        }
    }
}

#[derive(Debug, Clone)]
struct WritingWindowAccumulator {
    date: String,
    first_minutes: i64,
    last_minutes: i64,
    entry_count: i64,
}

impl WritingWindowAccumulator {
    fn into_day(self) -> AnalyticsWritingWindowDay {
        AnalyticsWritingWindowDay {
            date: self.date,
            first_time: minutes_to_time(Some(self.first_minutes)).unwrap_or_default(),
            last_time: minutes_to_time(Some(self.last_minutes)).unwrap_or_default(),
            first_minutes: self.first_minutes,
            last_minutes: self.last_minutes,
            span_minutes: (self.last_minutes - self.first_minutes).max(0),
            entry_count: self.entry_count,
        }
    }
}

#[derive(Debug, Default)]
struct LocationActivityBucket {
    count: i64,
    labels: HashMap<String, i64>,
}

impl LocationActivityBucket {
    fn into_breakdown(self) -> AnalyticsBreakdownItem {
        let mut labels = self.labels.into_iter().collect::<Vec<_>>();
        labels.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.0.to_lowercase().cmp(&right.0.to_lowercase()))
                .then_with(|| left.0.cmp(&right.0))
        });
        AnalyticsBreakdownItem {
            label: labels
                .into_iter()
                .next()
                .map(|(label, _)| label)
                .unwrap_or_else(|| "Unknown location".to_string()),
            count: self.count,
        }
    }
}

fn writing_window_summary(days: &[AnalyticsWritingWindowDay]) -> AnalyticsWritingWindowSummary {
    if days.is_empty() {
        return AnalyticsWritingWindowSummary {
            active_days: 0,
            total_entries: 0,
            avg_first_time: None,
            avg_last_time: None,
            avg_span_minutes: 0,
            earliest_first_time: None,
            latest_last_time: None,
            longest_span_day: None,
        };
    }

    let first_values = days.iter().map(|day| day.first_minutes).collect::<Vec<_>>();
    let last_values = days.iter().map(|day| day.last_minutes).collect::<Vec<_>>();
    let span_values = days.iter().map(|day| day.span_minutes).collect::<Vec<_>>();
    let longest_span_day = days
        .iter()
        .max_by(|left, right| {
            left.span_minutes
                .cmp(&right.span_minutes)
                .then_with(|| right.date.cmp(&left.date))
        })
        .map(|day| AnalyticsWritingWindowLongestDay {
            date: day.date.clone(),
            span_minutes: day.span_minutes,
        });

    AnalyticsWritingWindowSummary {
        active_days: days.len() as i64,
        total_entries: days.iter().map(|day| day.entry_count).sum(),
        avg_first_time: minutes_to_time(rounded_average(&first_values)),
        avg_last_time: minutes_to_time(rounded_average(&last_values)),
        avg_span_minutes: rounded_average(&span_values).unwrap_or(0),
        earliest_first_time: minutes_to_time(first_values.iter().min().copied()),
        latest_last_time: minutes_to_time(last_values.iter().max().copied()),
        longest_span_day,
    }
}

fn weekday_labels() -> [(i64, &'static str, &'static str); 7] {
    [
        (1, "Monday", "Mon"),
        (2, "Tuesday", "Tue"),
        (3, "Wednesday", "Wed"),
        (4, "Thursday", "Thu"),
        (5, "Friday", "Fri"),
        (6, "Saturday", "Sat"),
        (0, "Sunday", "Sun"),
    ]
}

fn weekday_day_num(date: NaiveDate) -> i64 {
    let day_num = date.weekday().number_from_monday();
    if day_num == 7 {
        0
    } else {
        day_num as i64
    }
}

fn parse_hour(value: &str) -> Option<i64> {
    parse_minutes_since_midnight(value).map(|minutes| minutes / 60)
}

fn parse_minutes_since_midnight(value: &str) -> Option<i64> {
    let time = value.get(11..16)?;
    let (hour, minute) = time.split_once(':')?;
    let hour = hour.parse::<i64>().ok()?;
    let minute = minute.parse::<i64>().ok()?;
    if (0..=23).contains(&hour) && (0..=59).contains(&minute) {
        Some(hour * 60 + minute)
    } else {
        None
    }
}

fn minutes_to_time(value: Option<i64>) -> Option<String> {
    value.map(|minutes| {
        let minutes = minutes.clamp(0, 1439);
        format!("{:02}:{:02}", minutes / 60, minutes % 60)
    })
}

fn rounded_average(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        None
    } else {
        Some((values.iter().sum::<i64>() + (values.len() as i64 / 2)) / values.len() as i64)
    }
}

#[derive(Debug, Clone, Copy)]
struct MoodSentimentSummary {
    sum: f64,
    count: i64,
}

impl MoodSentimentSummary {
    fn average(self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            Some(self.sum / self.count as f64)
        }
    }
}

fn mood_sentiment_summary(rows: &[EntryStatsRow]) -> MoodSentimentSummary {
    rows.iter()
        .filter_map(|row| row.mood.as_deref())
        .filter_map(mood_sentiment::score_for_mood)
        .fold(
            MoodSentimentSummary { sum: 0.0, count: 0 },
            |summary, score| MoodSentimentSummary {
                sum: summary.sum + score,
                count: summary.count + 1,
            },
        )
}

fn mood_breakdown(rows: &[EntryStatsRow]) -> Vec<AnalyticsBreakdownItem> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for mood in rows
        .iter()
        .filter_map(|row| row.mood.as_deref())
        .filter_map(|mood| normalize_string(Some(mood)))
    {
        *counts.entry(mood).or_insert(0) += 1;
    }
    let mut values = counts
        .into_iter()
        .map(|(label, count)| AnalyticsBreakdownItem { label, count })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    values.truncate(TOP_LIMIT);
    values
}

fn top_words(rows: &[EntryStatsRow]) -> Vec<WordCount> {
    let stopwords = stopwords();
    let mut counts: HashMap<String, i64> = HashMap::new();
    for word in rows.iter().flat_map(|row| words(&row.text)) {
        if word.len() < 3 || stopwords.contains(word.as_str()) {
            continue;
        }
        *counts.entry(word).or_insert(0) += 1;
    }
    let mut values = counts
        .into_iter()
        .map(|(word, count)| WordCount { word, count })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.word.cmp(&right.word))
    });
    values.truncate(TOP_LIMIT);
    values
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|ch| ch.is_alphanumeric() || *ch == '\'')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

fn word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|value| !value.is_empty())
        .count()
}

fn streaks(active_dates: &HashSet<NaiveDate>) -> (i64, i64) {
    if active_dates.is_empty() {
        return (0, 0);
    }
    let mut dates = active_dates.iter().copied().collect::<Vec<_>>();
    dates.sort();

    let mut longest = 1_i64;
    let mut current_run = 1_i64;
    for pair in dates.windows(2) {
        if pair[1] == pair[0] + Duration::days(1) {
            current_run += 1;
        } else {
            longest = longest.max(current_run);
            current_run = 1;
        }
    }
    longest = longest.max(current_run);

    let mut current = 0_i64;
    let mut cursor = *dates.last().unwrap();
    while active_dates.contains(&cursor) {
        current += 1;
        cursor -= Duration::days(1);
    }
    (longest, current)
}

fn days_in_year(year: i32) -> i64 {
    let start = NaiveDate::from_ymd_opt(year, 1, 1).expect("valid year");
    let next = NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid next year");
    (next - start).num_days()
}

struct SqlFilter {
    where_sql: String,
    params: Vec<Value>,
}

fn period_filter(alias: &str, period: &AnalyticsPeriodRequest) -> SqlFilter {
    let mut conditions = vec![format!("COALESCE({alias}.hidden, 0) = 0")];
    let mut params = Vec::new();
    if let Some(since) = normalize_string(period.since.as_deref()) {
        conditions.push(format!("datetime({alias}.created_at) >= datetime(?)"));
        params.push(Value::Text(since));
    }
    if let Some(until) = normalize_string(period.until.as_deref()) {
        conditions.push(format!("datetime({alias}.created_at) <= datetime(?)"));
        params.push(Value::Text(until));
    }
    SqlFilter {
        where_sql: format!("WHERE {}", conditions.join(" AND ")),
        params,
    }
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.get(0..10).unwrap_or(value), "%Y-%m-%d").ok()
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

trait OptionalRowExt<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalRowExt<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn normalize_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn stopwords() -> HashSet<&'static str> {
    [
        "the",
        "and",
        "for",
        "that",
        "with",
        "this",
        "from",
        "have",
        "but",
        "not",
        "you",
        "are",
        "was",
        "were",
        "about",
        "into",
        "just",
        "like",
        "they",
        "there",
        "then",
        "when",
        "what",
        "would",
        "could",
        "should",
        "really",
        "still",
        "been",
        "will",
        "over",
        "after",
        "before",
        "because",
        "through",
        "today",
        "tomorrow",
        "yesterday",
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::path::PathBuf;

    #[test]
    fn analytics_counts_entries_images_locations_and_breakdowns() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_stats_fixture(temp_dir.path());

        let response = get_analytics_for_database(
            &db_path,
            AnalyticsPeriodRequest {
                since: Some("2026-01-01".to_string()),
                until: Some("2026-12-31".to_string()),
            },
        )
        .expect("analytics");

        assert_eq!(response.overview.total_entries, 4);
        assert_eq!(response.overview.total_images, 2);
        assert_eq!(response.overview.entries_with_images, 1);
        assert_eq!(response.overview.entries_with_location, 2);
        assert_eq!(response.overview.mood_sentiment_count, 3);
        assert_close(response.overview.average_mood_sentiment, 1.0 / 3.0);
        assert_eq!(response.mood_breakdown[0].label, "focused");
        assert_eq!(response.tag_breakdown[0].label, "work");
        assert_eq!(response.daily_trend[1].date, "2026-01-02");
        assert_eq!(response.daily_trend[1].entry_count, 2);
        assert_eq!(response.hourly_trend[8].entry_count, 3);
        assert_eq!(response.hourly_trend[18].entry_count, 1);
        assert_eq!(
            response
                .weekday_trend
                .iter()
                .find(|day| day.short_label == "Fri")
                .expect("friday")
                .entry_count,
            2
        );
        assert_eq!(response.writing_window.days[1].first_time, "08:00");
        assert_eq!(response.writing_window.days[1].last_time, "18:30");
        assert_eq!(response.writing_window.summary.avg_span_minutes, 210);
        assert_eq!(response.location_activity[0].label, "Oslo");
        assert_eq!(response.monthly_trend[0].period, "2026-01");
        assert_eq!(response.monthly_trend[0].mood_sentiment_count, 2);
        assert_close(response.monthly_trend[0].average_mood_sentiment, 0.5);
    }

    #[test]
    fn writing_calendar_groups_active_days() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_stats_fixture(temp_dir.path());

        let response = get_writing_calendar_for_database(&db_path, Some(2026)).expect("calendar");

        assert_eq!(response.year, 2026);
        assert_eq!(response.active_days, 3);
        assert_eq!(response.max_entry_count, 2);
        assert!(response.days.iter().any(|day| day.date == "2026-01-02"));
        let happy_day = response
            .days
            .iter()
            .find(|day| day.date == "2026-01-02")
            .expect("happy day");
        assert_eq!(happy_day.mood_sentiment_count, 1);
        assert_close(happy_day.average_mood_sentiment, 1.0);
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("sentiment average");
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    fn create_stats_fixture(path: &Path) -> PathBuf {
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
                    weather_condition TEXT,
                    weather_temp_c REAL,
                    weather_temp_f REAL,
                    created_at TEXT NOT NULL
                );
                INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format, title, mood, hidden)
                VALUES
                    ('entry_one', '2026-01-01 08:00', '2026-01-01 08:00', 'Rust work starts', 'Rust work starts', 'markdown', 'One', 'focused', 0),
                    ('entry_two', '2026-01-02 08:00', '2026-01-02 08:00', 'Location weather note', 'Location weather note', 'plain', 'Two', 'happy', 0),
                    ('entry_two_evening', '2026-01-02 18:30', '2026-01-02 18:30', 'Evening followup words', 'Evening followup words', 'plain', 'Two evening', NULL, 0),
                    ('entry_three', '2026-02-01 08:00', '2026-02-01 08:00', 'More rust work', 'More rust work', 'plain', 'Three', 'focused', 0),
                    ('entry_hidden', '2026-02-02 08:00', '2026-02-02 08:00', 'Hidden', 'Hidden', 'plain', 'Hidden', 'quiet', 1);
                INSERT INTO tags (name) VALUES ('work'), ('personal');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1), (2, 2), (3, 1);
                INSERT INTO plugin_media_assets
                    (hash, mime_type, bytes, width, height, storage_backend, storage_key, created_at)
                VALUES ('hash', 'image/jpeg', 100, 10, 10, 'local_fs', 'ha/hash.jpg', '2026-01-01 08:00');
                INSERT INTO plugin_entry_media (entry_uuid, media_id, position, created_at)
                VALUES ('entry_one', 1, 0, '2026-01-01 08:00'),
                       ('entry_one', 1, 1, '2026-01-01 08:00');
                INSERT INTO plugin_entry_locations
                    (entry_uuid, latitude, longitude, place_name, weather_condition, weather_temp_c, weather_temp_f, created_at)
                VALUES ('entry_one', 69.0, 18.0, 'Tromso', 'Snow', -2, 28, '2026-01-01 08:00'),
                       ('entry_two', 59.0, 10.0, 'Oslo', 'Rain', 4, 39, '2026-01-02 08:00');
                ",
            )
            .expect("fixture");
        drop(connection);
        db_path
    }
}
