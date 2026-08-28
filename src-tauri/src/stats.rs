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
        AnalyticsBreakdownItem, AnalyticsCaptureSourceTrendPoint, AnalyticsCaptureSources,
        AnalyticsDailyTrendPoint, AnalyticsHourPoint, AnalyticsMoodTiming,
        AnalyticsMoodTimingPoint, AnalyticsMoodTrendPoint, AnalyticsOverview,
        AnalyticsPeriodRequest, AnalyticsResponse, AnalyticsTrendPoint, AnalyticsWeather,
        AnalyticsWeatherConditionMood, AnalyticsWeatherConditionTags, AnalyticsWeatherOverview,
        AnalyticsWeatherTag, AnalyticsWeatherTemperatureBucket, AnalyticsWeatherTrendPoint,
        AnalyticsWeekdayPoint, AnalyticsWritingWindow, AnalyticsWritingWindowDay,
        AnalyticsWritingWindowLongestDay, AnalyticsWritingWindowSummary, DashboardBestDay,
        DashboardBestEntry, DashboardBestMonth, DashboardBestOf, WordCount, WrappedActivityPoint,
        WrappedBadge, WrappedBusiestDay, WrappedChartCountPoint, WrappedCharts, WrappedComparison,
        WrappedFunFact, WrappedHighlight, WrappedHighlights, WrappedInsight, WrappedLongestEntry,
        WrappedMetricComparison, WrappedMostTaggedEntry, WrappedNavigation, WrappedRange,
        WrappedRecords, WrappedRequest, WrappedResponse, WrappedSummary, WritingCalendarDay,
        WritingCalendarResponse,
    },
    mood_sentiment,
};

const TOP_LIMIT: usize = 12;

#[derive(Debug, Clone)]
struct EntryStatsRow {
    id: i64,
    uuid: String,
    created_at: String,
    date: String,
    text: String,
    mood: Option<String>,
}

#[derive(Debug, Clone)]
struct WeatherStatsRow {
    entry_id: i64,
    date: String,
    mood: Option<String>,
    condition: Option<String>,
    temp_c: Option<f64>,
    humidity: Option<f64>,
    wind_kph: Option<f64>,
}

pub fn get_analytics(input: Option<AnalyticsPeriodRequest>) -> Result<AnalyticsResponse> {
    get_analytics_for_database(&db::resolve_database_path(), input.unwrap_or_default())
}

pub fn get_dashboard_best_of() -> Result<DashboardBestOf> {
    get_dashboard_best_of_for_database(&db::resolve_database_path())
}

pub(crate) fn get_dashboard_best_of_for_database(db_path: &Path) -> Result<DashboardBestOf> {
    let connection = db::open_read_only_connection(db_path)?;
    if !table_exists(&connection, "entries")? {
        return Err(anyhow!(
            "The active database does not contain an entries table."
        ));
    }

    let period = AnalyticsPeriodRequest::default();
    let rows = load_entry_rows(&connection, &period)?;
    let tag_counts_by_entry = tag_counts_by_entry(&connection, &period)?;
    let total_words = load_total_word_count(&connection)?;
    Ok(dashboard_best_of(&rows, &tag_counts_by_entry, total_words))
}

fn dashboard_best_of(
    rows: &[EntryStatsRow],
    tag_counts_by_entry: &HashMap<i64, i64>,
    total_words: i64,
) -> DashboardBestOf {
    let mut days: BTreeMap<String, WrappedDayBucket> = BTreeMap::new();
    let mut months: BTreeMap<String, WrappedDayBucket> = BTreeMap::new();

    for row in rows {
        let words = word_count(&row.text) as i64;
        let day = days.entry(row.date.clone()).or_default();
        day.entries += 1;
        day.words += words;

        if let Some(month) = row.date.get(0..7) {
            let month = months.entry(month.to_string()).or_default();
            month.entries += 1;
            month.words += words;
        }
    }

    let most_entries_day = days
        .iter()
        .max_by(|(left_date, left), (right_date, right)| {
            left.entries
                .cmp(&right.entries)
                .then_with(|| left.words.cmp(&right.words))
                .then_with(|| left_date.cmp(right_date))
        })
        .map(|(date, bucket)| DashboardBestDay {
            date: date.clone(),
            entry_count: bucket.entries,
            word_count: bucket.words,
        });
    let most_words_day = days
        .iter()
        .max_by(|(left_date, left), (right_date, right)| {
            left.words
                .cmp(&right.words)
                .then_with(|| left.entries.cmp(&right.entries))
                .then_with(|| left_date.cmp(right_date))
        })
        .map(|(date, bucket)| DashboardBestDay {
            date: date.clone(),
            entry_count: bucket.entries,
            word_count: bucket.words,
        });
    let longest_entry = rows
        .iter()
        .max_by(|left, right| {
            word_count(&left.text)
                .cmp(&word_count(&right.text))
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|row| dashboard_best_entry(row, tag_counts_by_entry));
    let most_tagged_entry = rows
        .iter()
        .filter(|row| tag_counts_by_entry.get(&row.id).copied().unwrap_or(0) > 0)
        .max_by(|left, right| {
            tag_counts_by_entry
                .get(&left.id)
                .copied()
                .unwrap_or(0)
                .cmp(&tag_counts_by_entry.get(&right.id).copied().unwrap_or(0))
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|row| dashboard_best_entry(row, tag_counts_by_entry));
    let most_entries_month = months
        .iter()
        .max_by(|(left_period, left), (right_period, right)| {
            left.entries
                .cmp(&right.entries)
                .then_with(|| left.words.cmp(&right.words))
                .then_with(|| left_period.cmp(right_period))
        })
        .map(|(period, bucket)| DashboardBestMonth {
            period: period.clone(),
            entry_count: bucket.entries,
            word_count: bucket.words,
        });
    let most_words_month = months
        .iter()
        .max_by(|(left_period, left), (right_period, right)| {
            left.words
                .cmp(&right.words)
                .then_with(|| left.entries.cmp(&right.entries))
                .then_with(|| left_period.cmp(right_period))
        })
        .map(|(period, bucket)| DashboardBestMonth {
            period: period.clone(),
            entry_count: bucket.entries,
            word_count: bucket.words,
        });

    DashboardBestOf {
        total_words,
        most_entries_day,
        most_words_day,
        most_tagged_entry,
        longest_entry,
        most_entries_month,
        most_words_month,
    }
}

fn load_total_word_count(connection: &Connection) -> Result<i64> {
    let mut statement = connection.prepare(
        "SELECT COALESCE(NULLIF(text_plain, ''), text, '')
         FROM entries",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .try_fold(0_i64, |total, text| {
            total
                .checked_add(word_count(&text) as i64)
                .ok_or_else(|| anyhow!("Total word count exceeds the supported range"))
        })
}

fn dashboard_best_entry(
    row: &EntryStatsRow,
    tag_counts_by_entry: &HashMap<i64, i64>,
) -> DashboardBestEntry {
    DashboardBestEntry {
        entry_id: row.id,
        uuid: row.uuid.clone(),
        created_at: row.created_at.clone(),
        date: row.date.clone(),
        word_count: word_count(&row.text) as i64,
        tag_count: tag_counts_by_entry.get(&row.id).copied().unwrap_or(0),
    }
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
    let mood_sentiments = mood_sentiment::scores_for_database(&connection)?;
    let mood_sentiment_summary = mood_sentiment_summary(&rows, &mood_sentiments);

    let capture_sources = capture_sources(&connection, &input, &rows)?;
    let weather = weather_analytics(&connection, &input, total_entries, &mood_sentiments)?;

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
        monthly_trend: monthly_trend(&rows, &mood_sentiments),
        daily_trend: daily_trend(&rows),
        hourly_trend: hourly_trend(&rows),
        weekday_trend: weekday_trend(&rows),
        writing_window: writing_window(&rows),
        capture_sources,
        mood_trend: mood_trend(&rows, &mood_sentiments),
        mood_timing: mood_timing(&rows, &mood_sentiments),
        weather,
        location_activity: location_activity(&connection, &input)?,
        mood_breakdown: mood_breakdown(&rows),
        tag_breakdown: tag_breakdown(&connection, &input)?,
        location_breakdown: location_breakdown(&connection, &input)?,
        weather_breakdown: weather_breakdown(&connection, &input)?,
        top_words: top_words(&rows),
        warnings: Vec::new(),
    })
}

#[derive(Debug, Clone)]
struct WrappedWindow {
    period: String,
    anchor: String,
    start: NaiveDate,
    end: NaiveDate,
    previous_start: NaiveDate,
    previous_end: NaiveDate,
    label: String,
    navigation: WrappedNavigation,
}

#[derive(Debug, Clone)]
struct WrappedDataset {
    summary: WrappedSummary,
    highlights: WrappedHighlights,
    records: WrappedRecords,
    charts: WrappedCharts,
}

#[derive(Debug, Clone, Default)]
struct WrappedDayBucket {
    entries: i64,
    words: i64,
}

pub fn get_wrapped(input: WrappedRequest) -> Result<WrappedResponse> {
    get_wrapped_for_database(
        &db::resolve_database_path(),
        &input.period,
        input.anchor.as_deref(),
        Local::now().date_naive(),
    )
}

pub(crate) fn get_wrapped_for_database(
    db_path: &Path,
    period: &str,
    anchor: Option<&str>,
    today: NaiveDate,
) -> Result<WrappedResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    if !table_exists(&connection, "entries")? {
        return Err(anyhow!(
            "The active database does not contain an entries table."
        ));
    }

    let window = resolve_wrapped_window(period, anchor, today)?;
    let granularity = if window.period == "year" {
        "month"
    } else {
        "day"
    };
    let current = collect_wrapped_dataset(&connection, window.start, window.end, granularity)?;
    let previous = collect_wrapped_dataset(
        &connection,
        window.previous_start,
        window.previous_end,
        granularity,
    )?;
    let day_count = (window.end - window.start).num_days() + 1;
    let comparison = WrappedComparison {
        entries: wrapped_metric_comparison(current.summary.entries, previous.summary.entries),
        words: wrapped_metric_comparison(current.summary.words, previous.summary.words),
        active_days: wrapped_metric_comparison(
            current.summary.active_days,
            previous.summary.active_days,
        ),
        health_score: wrapped_metric_comparison(
            current.summary.health_score,
            previous.summary.health_score,
        ),
    };
    let personal_best_badges = wrapped_personal_best_badges(&connection, window.start, window.end)?;

    Ok(WrappedResponse {
        period: window.period.clone(),
        anchor: window.anchor,
        title: format!("Capsule Wrapped: {}", window.label),
        range: WrappedRange {
            from: window.start.to_string(),
            to: window.end.to_string(),
            day_count,
            label: window.label,
        },
        navigation: window.navigation,
        summary: current.summary.clone(),
        comparison,
        highlights: current.highlights.clone(),
        records: current.records.clone(),
        insights: wrapped_insights(&current, &previous, &window.period),
        fun_facts: wrapped_fun_facts(&current, day_count),
        charts: current.charts,
        personal_best_badges,
    })
}

fn resolve_wrapped_window(
    period: &str,
    anchor: Option<&str>,
    today: NaiveDate,
) -> Result<WrappedWindow> {
    let period = period.trim().to_lowercase();
    if !matches!(period.as_str(), "week" | "month" | "year") {
        return Err(anyhow!("period must be one of: week, month, year"));
    }

    let (start, end, latest_start, latest_end, previous_start, previous_end, next_start, next_end) =
        match period.as_str() {
            "week" => {
                let current_week_start =
                    today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
                let latest_start = current_week_start - Duration::days(7);
                let start = if let Some(value) = normalize_string(anchor) {
                    NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                        .context("Week anchor must be a Monday in YYYY-MM-DD format")?
                } else {
                    latest_start
                };
                if start.weekday().num_days_from_monday() != 0 {
                    return Err(anyhow!("Week anchor must be a Monday in YYYY-MM-DD format"));
                }
                if start > latest_start {
                    return Err(anyhow!("Week anchor must reference a completed week"));
                }
                (
                    start,
                    start + Duration::days(6),
                    latest_start,
                    latest_start + Duration::days(6),
                    start - Duration::days(7),
                    start - Duration::days(1),
                    start + Duration::days(7),
                    start + Duration::days(13),
                )
            }
            "month" => {
                let current_month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                    .expect("current month must be valid");
                let latest_start = shift_month_start(current_month_start, -1)?;
                let start = if let Some(value) = normalize_string(anchor) {
                    if value.len() != 7 {
                        return Err(anyhow!("Month anchor must be in YYYY-MM format"));
                    }
                    NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d")
                        .context("Month anchor must be in YYYY-MM format")?
                } else {
                    latest_start
                };
                if start > latest_start {
                    return Err(anyhow!("Month anchor must reference a completed month"));
                }
                let previous_start = shift_month_start(start, -1)?;
                let next_start = shift_month_start(start, 1)?;
                (
                    start,
                    last_day_of_month(start)?,
                    latest_start,
                    last_day_of_month(latest_start)?,
                    previous_start,
                    last_day_of_month(previous_start)?,
                    next_start,
                    last_day_of_month(next_start)?,
                )
            }
            _ => {
                let latest_start = NaiveDate::from_ymd_opt(today.year() - 1, 1, 1)
                    .ok_or_else(|| anyhow!("Unable to resolve the latest completed year"))?;
                let start = if let Some(value) = normalize_string(anchor) {
                    if value.len() != 4
                        || !value.chars().all(|character| character.is_ascii_digit())
                    {
                        return Err(anyhow!("Year anchor must be in YYYY format"));
                    }
                    let year = value
                        .parse::<i32>()
                        .context("Year anchor must be in YYYY format")?;
                    NaiveDate::from_ymd_opt(year, 1, 1)
                        .ok_or_else(|| anyhow!("Year anchor must be a positive year"))?
                } else {
                    latest_start
                };
                if start > latest_start {
                    return Err(anyhow!("Year anchor must reference a completed year"));
                }
                let previous_start = NaiveDate::from_ymd_opt(start.year() - 1, 1, 1)
                    .ok_or_else(|| anyhow!("Unable to resolve the previous year"))?;
                let next_start = NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)
                    .ok_or_else(|| anyhow!("Unable to resolve the next year"))?;
                (
                    start,
                    NaiveDate::from_ymd_opt(start.year(), 12, 31).expect("valid year end"),
                    latest_start,
                    NaiveDate::from_ymd_opt(latest_start.year(), 12, 31)
                        .expect("valid latest year end"),
                    previous_start,
                    NaiveDate::from_ymd_opt(previous_start.year(), 12, 31)
                        .expect("valid previous year end"),
                    next_start,
                    NaiveDate::from_ymd_opt(next_start.year(), 12, 31)
                        .expect("valid next year end"),
                )
            }
        };

    let anchor_value = wrapped_anchor(&period, start);
    let latest_anchor = wrapped_anchor(&period, latest_start);
    let is_latest = start == latest_start;
    let next_anchor = (!is_latest).then(|| wrapped_anchor(&period, next_start));
    let next_label = (!is_latest).then(|| wrapped_period_label(&period, next_start, next_end));

    Ok(WrappedWindow {
        period: period.clone(),
        anchor: anchor_value,
        start,
        end,
        previous_start,
        previous_end,
        label: wrapped_period_label(&period, start, end),
        navigation: WrappedNavigation {
            latest_anchor,
            latest_label: wrapped_period_label(&period, latest_start, latest_end),
            previous_anchor: wrapped_anchor(&period, previous_start),
            previous_label: wrapped_period_label(&period, previous_start, previous_end),
            next_anchor,
            next_label,
            is_latest,
        },
    })
}

fn collect_wrapped_dataset(
    connection: &Connection,
    start: NaiveDate,
    end: NaiveDate,
    granularity: &str,
) -> Result<WrappedDataset> {
    let period = AnalyticsPeriodRequest {
        since: Some(start.to_string()),
        until: Some(format!("{end} 23:59:59")),
    };
    let rows = load_entry_rows(connection, &period)?;
    let total_entries = rows.len() as i64;
    let total_words = rows
        .iter()
        .map(|row| word_count(&row.text) as i64)
        .sum::<i64>();
    let active_dates = rows
        .iter()
        .filter_map(|row| parse_date(&row.date))
        .collect::<HashSet<_>>();
    let active_days = active_dates.len() as i64;
    let (longest_streak, _) = streaks(&active_dates);
    let tags = tag_breakdown(connection, &period)?;
    let moods = mood_breakdown(&rows);
    let locations = location_breakdown(connection, &period)?;
    let tag_counts_by_entry = tag_counts_by_entry(connection, &period)?;

    let top_tag = tags.first().map(|item| WrappedHighlight {
        label: item.label.clone(),
        count: item.count,
        share: Some(round_to(
            if total_entries == 0 {
                0.0
            } else {
                item.count as f64 / total_entries as f64
            },
            3,
        )),
        hour: None,
    });
    let mood_total = moods.iter().map(|item| item.count).sum::<i64>();
    let top_mood = moods.first().map(|item| WrappedHighlight {
        label: item.label.clone(),
        count: item.count,
        share: Some(round_to(
            if mood_total == 0 {
                0.0
            } else {
                item.count as f64 / mood_total as f64
            },
            3,
        )),
        hour: None,
    });

    let mut weekday_counts = [0_i64; 7];
    let mut hour_counts = [0_i64; 24];
    for row in &rows {
        if let Some(date) = parse_date(&row.date) {
            weekday_counts[date.weekday().num_days_from_monday() as usize] += 1;
        }
        if let Some(hour) = parse_hour(&row.created_at) {
            hour_counts[hour as usize] += 1;
        }
    }
    let top_weekday_index =
        (0..7)
            .filter(|index| weekday_counts[*index] > 0)
            .max_by(|left, right| {
                weekday_counts[*left]
                    .cmp(&weekday_counts[*right])
                    .then_with(|| right.cmp(left))
            });
    let top_hour_index = (0..24)
        .filter(|index| hour_counts[*index] > 0)
        .max_by(|left, right| {
            hour_counts[*left]
                .cmp(&hour_counts[*right])
                .then_with(|| right.cmp(left))
        });
    let weekday_names = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let top_weekday = top_weekday_index.map(|index| WrappedHighlight {
        label: weekday_names[index].to_string(),
        count: weekday_counts[index],
        share: None,
        hour: None,
    });
    let top_hour = top_hour_index.map(|index| WrappedHighlight {
        label: format!("{index:02}:00"),
        count: hour_counts[index],
        share: None,
        hour: Some(index as i64),
    });
    let top_location = locations.first().map(|item| WrappedHighlight {
        label: item.label.clone(),
        count: item.count,
        share: None,
        hour: None,
    });

    let mut by_day: BTreeMap<String, WrappedDayBucket> = BTreeMap::new();
    let mut activity_buckets: BTreeMap<String, WrappedDayBucket> = BTreeMap::new();
    for row in &rows {
        let entry_words = word_count(&row.text) as i64;
        let day = by_day.entry(row.date.clone()).or_default();
        day.entries += 1;
        day.words += entry_words;
        let activity_key = if granularity == "month" {
            row.date.get(0..7).unwrap_or(&row.date).to_string()
        } else {
            row.date.clone()
        };
        let activity = activity_buckets.entry(activity_key).or_default();
        activity.entries += 1;
        activity.words += entry_words;
    }

    let busiest_day = by_day
        .iter()
        .max_by(|(left_date, left), (right_date, right)| {
            left.entries
                .cmp(&right.entries)
                .then_with(|| left.words.cmp(&right.words))
                .then_with(|| left_date.cmp(right_date))
        })
        .map(|(date, bucket)| WrappedBusiestDay {
            date: date.clone(),
            entry_count: bucket.entries,
            word_count: bucket.words,
        });
    let longest_entry = rows
        .iter()
        .max_by(|left, right| {
            word_count(&left.text)
                .cmp(&word_count(&right.text))
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|row| WrappedLongestEntry {
            entry_id: row.id,
            uuid: row.uuid.clone(),
            created_at: row.created_at.clone(),
            date: row.date.clone(),
            word_count: word_count(&row.text) as i64,
        });
    let most_tagged_entry = rows
        .iter()
        .filter_map(|row| {
            let tag_count = tag_counts_by_entry.get(&row.id).copied().unwrap_or(0);
            (tag_count > 0).then_some((row, tag_count))
        })
        .max_by(|(left, left_count), (right, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|(row, tag_count)| WrappedMostTaggedEntry {
            entry_id: row.id,
            created_at: row.created_at.clone(),
            tag_count,
        });

    Ok(WrappedDataset {
        summary: WrappedSummary {
            entries: total_entries,
            words: total_words,
            active_days,
            avg_words_per_entry: round_to(
                if total_entries == 0 {
                    0.0
                } else {
                    total_words as f64 / total_entries as f64
                },
                2,
            ),
            avg_entries_per_active_day: round_to(
                if active_days == 0 {
                    0.0
                } else {
                    total_entries as f64 / active_days as f64
                },
                2,
            ),
            health_score: wrapped_health_score(total_entries, longest_streak, active_days),
            longest_streak,
        },
        highlights: WrappedHighlights {
            top_tag,
            top_mood,
            top_weekday,
            top_hour,
            top_location,
        },
        records: WrappedRecords {
            busiest_day,
            longest_entry,
            most_tagged_entry,
        },
        charts: WrappedCharts {
            activity_granularity: granularity.to_string(),
            activity: wrapped_activity_points(start, end, granularity, &activity_buckets)?,
            top_tags: tags
                .into_iter()
                .take(8)
                .map(|item| WrappedChartCountPoint {
                    label: item.label,
                    count: item.count,
                })
                .collect(),
            mood_distribution: moods
                .into_iter()
                .map(|item| WrappedChartCountPoint {
                    label: item.label,
                    count: item.count,
                })
                .collect(),
        },
    })
}

fn wrapped_activity_points(
    start: NaiveDate,
    end: NaiveDate,
    granularity: &str,
    buckets: &BTreeMap<String, WrappedDayBucket>,
) -> Result<Vec<WrappedActivityPoint>> {
    let mut points = Vec::new();
    if granularity == "month" {
        let mut cursor = NaiveDate::from_ymd_opt(start.year(), start.month(), 1)
            .expect("wrapped month start must be valid");
        while cursor <= end {
            let period = cursor.format("%Y-%m").to_string();
            let bucket = buckets.get(&period).cloned().unwrap_or_default();
            points.push(WrappedActivityPoint {
                period,
                entries: bucket.entries,
                words: bucket.words,
            });
            cursor = shift_month_start(cursor, 1)?;
        }
    } else {
        let mut cursor = start;
        while cursor <= end {
            let period = cursor.to_string();
            let bucket = buckets.get(&period).cloned().unwrap_or_default();
            points.push(WrappedActivityPoint {
                period,
                entries: bucket.entries,
                words: bucket.words,
            });
            cursor += Duration::days(1);
        }
    }
    Ok(points)
}

fn tag_counts_by_entry(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
) -> Result<HashMap<i64, i64>> {
    if !table_exists(connection, "tags")? || !table_exists(connection, "entry_tags")? {
        return Ok(HashMap::new());
    }
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT e.id, COUNT(*)
         FROM entry_tags et
         JOIN entries e ON e.id = et.entry_id
         {}
         GROUP BY e.id",
        filter.where_sql
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(filter.params), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .context("failed to count entry tags for wrapped")
}

fn wrapped_metric_comparison(current: i64, previous: i64) -> WrappedMetricComparison {
    let delta = current - previous;
    WrappedMetricComparison {
        current,
        previous,
        delta,
        pct_change: (previous != 0).then(|| round_to(delta as f64 / previous as f64 * 100.0, 1)),
        direction: if delta > 0 {
            "up"
        } else if delta < 0 {
            "down"
        } else {
            "flat"
        }
        .to_string(),
    }
}

fn wrapped_insights(
    current: &WrappedDataset,
    previous: &WrappedDataset,
    period: &str,
) -> Vec<WrappedInsight> {
    let mut insights = Vec::new();
    let current_entries = current.summary.entries;
    let previous_entries = previous.summary.entries;
    let current_words = current.summary.words;
    let previous_words = previous.summary.words;

    if current_entries > 0 {
        let body = if previous_entries > 0 && current_entries != previous_entries {
            let delta = current_entries - previous_entries;
            Some(format!(
                "You logged {} {} {} than the previous {} ({} vs {}).",
                delta.abs(),
                if delta > 0 { "more" } else { "fewer" },
                plural(delta.abs(), "entry"),
                period,
                current_entries,
                previous_entries
            ))
        } else if previous_entries == 0 {
            Some(format!(
                "After a quiet previous {period}, you filled this one with {current_entries} {} and {current_words} words.",
                plural(current_entries, "entry")
            ))
        } else if previous_words != current_words {
            let delta = current_words - previous_words;
            Some(format!(
                "Your writing volume landed at {current_words} words, {} {} than the previous {period}.",
                delta.abs(),
                if delta > 0 { "more" } else { "fewer" }
            ))
        } else {
            None
        };
        if let Some(body) = body {
            insights.push(WrappedInsight {
                kind: "momentum".to_string(),
                title: "Momentum".to_string(),
                body,
            });
        }
    }

    let routine_body = match (
        current.highlights.top_weekday.as_ref(),
        current.highlights.top_hour.as_ref(),
    ) {
        (Some(weekday), Some(hour)) => Some(format!(
            "Your rhythm leaned toward {} around {}, when you showed up most often.",
            weekday.label, hour.label
        )),
        (Some(weekday), None) => Some(format!(
            "{} was your most active writing day this {period}.",
            weekday.label
        )),
        (None, Some(hour)) => Some(format!(
            "{} was your peak writing hour this {period}.",
            hour.label
        )),
        (None, None) => None,
    };
    if let Some(body) = routine_body {
        insights.push(WrappedInsight {
            kind: "routine".to_string(),
            title: "Routine".to_string(),
            body,
        });
    }
    if let Some(mood) = current.highlights.top_mood.as_ref() {
        insights.push(WrappedInsight {
            kind: "mood".to_string(),
            title: "Mood".to_string(),
            body: format!(
                "'{}' led your mood check-ins, showing up on {} {}.",
                mood.label,
                mood.count,
                plural(mood.count, "entry")
            ),
        });
    }
    if let Some(tag) = current.highlights.top_tag.as_ref() {
        insights.push(WrappedInsight {
            kind: "topic".to_string(),
            title: "Topic".to_string(),
            body: format!(
                "The tag '{}' kept resurfacing, attached to {} {}.",
                tag.label,
                tag.count,
                plural(tag.count, "entry")
            ),
        });
    }
    insights
}

fn wrapped_fun_facts(dataset: &WrappedDataset, day_count: i64) -> Vec<WrappedFunFact> {
    let mut facts = Vec::new();
    if let Some(day) = dataset.records.busiest_day.as_ref() {
        facts.push(WrappedFunFact {
            kind: "busiest_day".to_string(),
            title: "Busiest Day".to_string(),
            body: format!(
                "{} packed in {} {} and {} words.",
                wrapped_display_date(&day.date),
                day.entry_count,
                plural(day.entry_count, "entry"),
                day.word_count
            ),
        });
    }
    if let Some(entry) = dataset.records.longest_entry.as_ref() {
        facts.push(WrappedFunFact {
            kind: "longest_entry".to_string(),
            title: "Longest Entry".to_string(),
            body: format!(
                "Your longest entry stretched to {} {} on {}.",
                entry.word_count,
                plural(entry.word_count, "word"),
                wrapped_display_date(&entry.date)
            ),
        });
    }
    if let Some(entry) = dataset.records.most_tagged_entry.as_ref() {
        facts.push(WrappedFunFact {
            kind: "most_tagged_entry".to_string(),
            title: "Most Tagged Entry".to_string(),
            body: format!(
                "One entry carried {} {} on {}.",
                entry.tag_count,
                plural(entry.tag_count, "tag"),
                wrapped_display_date(entry.created_at.get(0..10).unwrap_or(&entry.created_at))
            ),
        });
    }
    if dataset.summary.entries > 0 {
        let consistency = if day_count == 0 {
            0
        } else {
            (dataset.summary.active_days as f64 / day_count as f64 * 100.0).round() as i64
        };
        facts.push(WrappedFunFact {
            kind: "consistency".to_string(),
            title: "Consistency".to_string(),
            body: format!(
                "You showed up on {} of {} days ({}%) and built a best run of {} {}.",
                dataset.summary.active_days,
                day_count,
                consistency,
                dataset.summary.longest_streak,
                plural(dataset.summary.longest_streak, "day")
            ),
        });
    }
    facts
}

fn wrapped_personal_best_badges(
    connection: &Connection,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<WrappedBadge>> {
    let period = AnalyticsPeriodRequest::default();
    let rows = load_entry_rows(connection, &period)?;
    let tag_counts = tag_counts_by_entry(connection, &period)?;
    let mut by_day: BTreeMap<String, WrappedDayBucket> = BTreeMap::new();
    for row in &rows {
        let bucket = by_day.entry(row.date.clone()).or_default();
        bucket.entries += 1;
        bucket.words += word_count(&row.text) as i64;
    }
    let best_notes_day = by_day
        .iter()
        .max_by(|(left_date, left), (right_date, right)| {
            left.entries
                .cmp(&right.entries)
                .then_with(|| left.words.cmp(&right.words))
                .then_with(|| left_date.cmp(right_date))
        });
    let best_words_day = by_day
        .iter()
        .max_by(|(left_date, left), (right_date, right)| {
            left.words
                .cmp(&right.words)
                .then_with(|| left.entries.cmp(&right.entries))
                .then_with(|| left_date.cmp(right_date))
        });
    let most_tagged = rows
        .iter()
        .filter_map(|row| {
            let count = tag_counts.get(&row.id).copied().unwrap_or(0);
            (count > 0).then_some((row, count))
        })
        .max_by(|(left, left_count), (right, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        });
    let longest_entry = rows.iter().max_by(|left, right| {
        word_count(&left.text)
            .cmp(&word_count(&right.text))
            .then_with(|| left.created_at.cmp(&right.created_at))
            .then_with(|| left.id.cmp(&right.id))
    });
    let in_range = |value: &str| {
        parse_date(value)
            .map(|date| date >= start && date <= end)
            .unwrap_or(false)
    };
    let mut badges = Vec::new();
    if let Some((date, bucket)) = best_notes_day.filter(|(date, _)| in_range(date)) {
        badges.push(WrappedBadge {
            id: "best_notes_day".to_string(),
            title: "Lifetime Best Notes Day".to_string(),
            value: format!("{} {}", bucket.entries, plural(bucket.entries, "entry")),
            detail: wrapped_display_date(date),
        });
    }
    if let Some((date, bucket)) = best_words_day.filter(|(date, _)| in_range(date)) {
        badges.push(WrappedBadge {
            id: "best_words_day".to_string(),
            title: "Lifetime Best Words Day".to_string(),
            value: format!("{} {}", bucket.words, plural(bucket.words, "word")),
            detail: wrapped_display_date(date),
        });
    }
    if let Some((entry, count)) = most_tagged.filter(|(entry, _)| in_range(&entry.created_at)) {
        badges.push(WrappedBadge {
            id: "most_tags_in_post".to_string(),
            title: "Most Tags On One Entry".to_string(),
            value: format!("{count} {}", plural(count, "tag")),
            detail: wrapped_display_date(&entry.date),
        });
    }
    if let Some(entry) = longest_entry.filter(|entry| in_range(&entry.created_at)) {
        let count = word_count(&entry.text) as i64;
        badges.push(WrappedBadge {
            id: "longest_entry".to_string(),
            title: "Longest Entry Ever".to_string(),
            value: format!("{count} {}", plural(count, "word")),
            detail: wrapped_display_date(&entry.date),
        });
    }
    Ok(badges)
}

fn wrapped_health_score(total_entries: i64, longest_streak: i64, active_days: i64) -> i64 {
    if total_entries == 0 {
        return 0;
    }
    let entry_score = ((total_entries as f64 / 100.0) * 30.0) as i64;
    let streak_score = ((longest_streak as f64 / 30.0) * 40.0) as i64;
    let activity_score = ((active_days as f64 / 7.0) * 30.0) as i64;
    entry_score.min(30) + streak_score.min(40) + activity_score.min(30)
}

fn wrapped_anchor(period: &str, start: NaiveDate) -> String {
    match period {
        "week" => start.to_string(),
        "month" => start.format("%Y-%m").to_string(),
        _ => start.year().to_string(),
    }
}

fn wrapped_period_label(period: &str, start: NaiveDate, end: NaiveDate) -> String {
    match period {
        "week" => wrapped_date_range(start, end),
        "month" => start.format("%B %Y").to_string(),
        _ => start.year().to_string(),
    }
}

fn wrapped_date_range(start: NaiveDate, end: NaiveDate) -> String {
    if start.year() == end.year() {
        if start.month() == end.month() {
            format!(
                "{} {} - {}, {}",
                start.format("%b"),
                start.day(),
                end.day(),
                start.year()
            )
        } else {
            format!(
                "{} {} - {} {}, {}",
                start.format("%b"),
                start.day(),
                end.format("%b"),
                end.day(),
                start.year()
            )
        }
    } else {
        format!(
            "{} {}, {} - {} {}, {}",
            start.format("%b"),
            start.day(),
            start.year(),
            end.format("%b"),
            end.day(),
            end.year()
        )
    }
}

fn wrapped_display_date(value: &str) -> String {
    parse_date(value)
        .map(|date| date.format("%b %d, %Y").to_string())
        .unwrap_or_else(|| value.to_string())
}

fn shift_month_start(value: NaiveDate, months: i32) -> Result<NaiveDate> {
    let month_index = value.year() * 12 + value.month0() as i32 + months;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| anyhow!("Unable to resolve wrapped month"))
}

fn last_day_of_month(value: NaiveDate) -> Result<NaiveDate> {
    Ok(shift_month_start(value, 1)? - Duration::days(1))
}

fn round_to(value: f64, places: i32) -> f64 {
    let scale = 10_f64.powi(places);
    (value * scale).round() / scale
}

fn plural(value: i64, singular: &str) -> String {
    if value == 1 {
        singular.to_string()
    } else if let Some(stem) = singular.strip_suffix('y').filter(|stem| {
        stem.chars()
            .last()
            .is_some_and(|letter| !"aeiou".contains(letter))
    }) {
        format!("{stem}ies")
    } else {
        format!("{singular}s")
    }
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
    let mood_sentiments = mood_sentiment::scores_for_database(&connection)?;
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
            if let Some(score) = mood_sentiment::score_from_catalog(&mood_sentiments, &mood) {
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
        "SELECT e.id,
                e.uuid,
                e.created_at,
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
            id: row.get(0)?,
            uuid: row.get(1)?,
            created_at: row.get(2)?,
            date: row.get(3)?,
            text: row.get(4)?,
            mood: row.get(5)?,
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

fn monthly_trend(
    rows: &[EntryStatsRow],
    mood_sentiments: &HashMap<String, f64>,
) -> Vec<AnalyticsTrendPoint> {
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
        if let Some(score) = row
            .mood
            .as_deref()
            .and_then(|mood| mood_sentiment::score_from_catalog(mood_sentiments, mood))
        {
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

fn capture_sources(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
    rows: &[EntryStatsRow],
) -> Result<AnalyticsCaptureSources> {
    let mobile_uuids = if table_exists(connection, "plugin_entry_locations")?
        && table_columns(connection, "plugin_entry_locations")?.contains("source")
    {
        let filter = period_filter("e", period);
        let sql = format!(
            "SELECT DISTINCT e.uuid
             FROM entries e
             JOIN plugin_entry_locations pel ON pel.entry_uuid = e.uuid
             {}
             AND lower(COALESCE(pel.source, '')) = 'mobile'",
            filter.where_sql
        );
        let mut statement = connection.prepare(&sql)?;
        let values = statement
            .query_map(params_from_iter(filter.params), |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<HashSet<_>>>()?;
        values
    } else {
        HashSet::new()
    };

    let total_entries = rows.len() as i64;
    let mobile_entries = rows
        .iter()
        .filter(|row| mobile_uuids.contains(&row.uuid))
        .count() as i64;
    let desktop_entries = total_entries.saturating_sub(mobile_entries);
    let use_months = rows
        .first()
        .and_then(|first| parse_date(&first.date))
        .zip(rows.last().and_then(|last| parse_date(&last.date)))
        .is_some_and(|(first, last)| (last - first).num_days() > 90);
    let mut trend: BTreeMap<String, (i64, i64)> = BTreeMap::new();

    for row in rows {
        let period = if use_months {
            row.date.get(0..7).unwrap_or(&row.date).to_string()
        } else {
            row.date.clone()
        };
        let bucket = trend.entry(period).or_default();
        if mobile_uuids.contains(&row.uuid) {
            bucket.0 += 1;
        } else {
            bucket.1 += 1;
        }
    }

    Ok(AnalyticsCaptureSources {
        total_entries,
        mobile_entries,
        desktop_entries,
        mobile_percent: percent(mobile_entries, total_entries),
        desktop_percent: percent(desktop_entries, total_entries),
        trend: trend
            .into_iter()
            .map(|(period, (mobile_count, desktop_count))| {
                let total_count = mobile_count + desktop_count;
                AnalyticsCaptureSourceTrendPoint {
                    period,
                    mobile_count,
                    desktop_count,
                    total_count,
                    mobile_percent: percent(mobile_count, total_count),
                }
            })
            .collect(),
    })
}

fn mood_trend(
    rows: &[EntryStatsRow],
    mood_sentiments: &HashMap<String, f64>,
) -> Vec<AnalyticsMoodTrendPoint> {
    let mut values: BTreeMap<String, MoodBucket> = BTreeMap::new();
    for row in rows {
        let Some((mood, score)) = rated_mood(row, mood_sentiments) else {
            continue;
        };
        values.entry(row.date.clone()).or_default().add(mood, score);
    }
    values
        .into_iter()
        .map(|(date, bucket)| AnalyticsMoodTrendPoint {
            date,
            average_sentiment: bucket.average().unwrap_or_default(),
            mood_count: bucket.count,
        })
        .collect()
}

fn mood_timing(
    rows: &[EntryStatsRow],
    mood_sentiments: &HashMap<String, f64>,
) -> AnalyticsMoodTiming {
    const PHASES: [(&str, &str, &str); 5] = [
        ("morning", "Morning", "06:00-10:00"),
        ("midday", "Midday", "10:00-15:00"),
        ("late-afternoon", "Late Afternoon", "15:00-17:00"),
        ("evening", "Evening", "17:00-21:00"),
        ("late-night", "Late Night", "21:00-06:00"),
    ];
    let mut phase_buckets: HashMap<&str, MoodBucket> = HashMap::new();
    let mut weekday_buckets: HashMap<i64, MoodBucket> = HashMap::new();

    for row in rows {
        let Some((mood, score)) = rated_mood(row, mood_sentiments) else {
            continue;
        };
        if let Some(hour) = parse_hour(&row.created_at) {
            let phase = match hour {
                6..=9 => "morning",
                10..=14 => "midday",
                15..=16 => "late-afternoon",
                17..=20 => "evening",
                _ => "late-night",
            };
            phase_buckets.entry(phase).or_default().add(mood, score);
        }
        if let Some(date) = parse_date(&row.date) {
            weekday_buckets
                .entry(weekday_day_num(date))
                .or_default()
                .add(mood, score);
        }
    }

    let time_of_day = PHASES
        .into_iter()
        .map(|(key, label, detail)| {
            mood_timing_point(
                key,
                label,
                detail,
                phase_buckets.remove(key).unwrap_or_default(),
            )
        })
        .collect();
    let day_of_week = weekday_labels()
        .into_iter()
        .map(|(day_num, label, short_label)| {
            mood_timing_point(
                &day_num.to_string(),
                short_label,
                label,
                weekday_buckets.remove(&day_num).unwrap_or_default(),
            )
        })
        .collect();

    AnalyticsMoodTiming {
        time_of_day,
        day_of_week,
    }
}

fn mood_timing_point(
    key: &str,
    label: &str,
    detail: &str,
    bucket: MoodBucket,
) -> AnalyticsMoodTimingPoint {
    AnalyticsMoodTimingPoint {
        key: key.to_string(),
        label: label.to_string(),
        detail: detail.to_string(),
        average_sentiment: bucket.average(),
        mood_count: bucket.count,
        top_mood: bucket.top_mood(),
    }
}

fn rated_mood<'a>(
    row: &'a EntryStatsRow,
    mood_sentiments: &HashMap<String, f64>,
) -> Option<(&'a str, f64)> {
    let mood = row.mood.as_deref()?;
    mood_sentiment::score_from_catalog(mood_sentiments, mood).map(|score| (mood, score))
}

fn weather_analytics(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
    total_entries: i64,
    mood_sentiments: &HashMap<String, f64>,
) -> Result<AnalyticsWeather> {
    if !table_exists(connection, "plugin_entry_locations")? {
        return Ok(empty_weather_analytics(total_entries));
    }
    let columns = table_columns(connection, "plugin_entry_locations")?;
    let condition_expr = if columns.contains("weather_condition") {
        "pel.weather_condition"
    } else {
        "NULL"
    };
    let temp_expr = if columns.contains("weather_temp_c") {
        "pel.weather_temp_c"
    } else if columns.contains("weather_temp_f") {
        "((pel.weather_temp_f - 32.0) * 5.0 / 9.0)"
    } else {
        "NULL"
    };
    let humidity_expr = if columns.contains("weather_humidity") {
        "CAST(pel.weather_humidity AS REAL)"
    } else {
        "NULL"
    };
    let wind_expr = if columns.contains("weather_wind_kph") {
        "pel.weather_wind_kph"
    } else {
        "NULL"
    };
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT e.id,
                substr(e.created_at, 1, 10),
                e.mood,
                {condition_expr},
                {temp_expr},
                {humidity_expr},
                {wind_expr}
         FROM entries e
         JOIN plugin_entry_locations pel ON pel.entry_uuid = e.uuid
         {}
         ORDER BY datetime(e.created_at) ASC, e.id ASC",
        filter.where_sql
    );
    let mut statement = connection.prepare(&sql)?;
    let weather_rows = statement
        .query_map(params_from_iter(filter.params), |row| {
            Ok(WeatherStatsRow {
                entry_id: row.get(0)?,
                date: row.get(1)?,
                mood: row.get(2)?,
                condition: row.get(3)?,
                temp_c: row.get(4)?,
                humidity: row.get(5)?,
                wind_kph: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let rows = weather_rows
        .into_iter()
        .filter(|row| {
            normalize_string(row.condition.as_deref()).is_some()
                || row.temp_c.is_some()
                || row.humidity.is_some()
                || row.wind_kph.is_some()
        })
        .collect::<Vec<_>>();
    let entries_with_weather = rows
        .iter()
        .map(|row| row.entry_id)
        .collect::<HashSet<_>>()
        .len() as i64;
    let mut condition_counts: HashMap<String, i64> = HashMap::new();
    let mut temperatures = Vec::new();
    let mut temperature_mood_points = Vec::new();
    let mut mood_by_condition: HashMap<String, MoodBucket> = HashMap::new();
    let mut trend: BTreeMap<String, WeatherTrendAccumulator> = BTreeMap::new();
    let mut temperature_buckets = weather_temperature_buckets();

    for row in &rows {
        let condition = normalize_string(row.condition.as_deref());
        if let Some(condition) = condition.as_ref() {
            *condition_counts.entry(condition.clone()).or_insert(0) += 1;
        }
        if let Some(temp) = row.temp_c {
            temperatures.push(temp);
            if let Some(bucket) = temperature_buckets
                .iter_mut()
                .find(|bucket| weather_temperature_bucket_matches(&bucket.key, temp))
            {
                bucket.count += 1;
            }
        }
        let daily = trend.entry(row.date.clone()).or_default();
        daily.entry_count += 1;
        daily.add_temp(row.temp_c);
        daily.add_humidity(row.humidity);
        daily.add_wind(row.wind_kph);

        if let (Some(condition), Some(mood)) = (condition, row.mood.as_deref()) {
            if let Some(score) = mood_sentiment::score_from_catalog(mood_sentiments, mood) {
                mood_by_condition
                    .entry(condition)
                    .or_default()
                    .add(mood, score);
                if let Some(temp) = row.temp_c {
                    temperature_mood_points.push((temp, score));
                }
            }
        }
    }

    let mut condition_mood = mood_by_condition
        .into_iter()
        .filter_map(|(condition, bucket)| {
            Some(AnalyticsWeatherConditionMood {
                condition,
                average_sentiment: bucket.average()?,
                mood_count: bucket.count,
                top_mood: bucket.top_mood(),
            })
        })
        .collect::<Vec<_>>();
    condition_mood.sort_by(|left, right| {
        right
            .average_sentiment
            .total_cmp(&left.average_sentiment)
            .then_with(|| right.mood_count.cmp(&left.mood_count))
            .then_with(|| {
                left.condition
                    .to_lowercase()
                    .cmp(&right.condition.to_lowercase())
            })
    });

    let most_common_condition = condition_counts
        .iter()
        .max_by(|left, right| {
            left.1
                .cmp(right.1)
                .then_with(|| right.0.to_lowercase().cmp(&left.0.to_lowercase()))
        })
        .map(|(condition, _)| condition.clone());
    let average_temp_c = average_f64(&temperatures);
    let min_temp_c = temperatures.iter().copied().reduce(f64::min);
    let max_temp_c = temperatures.iter().copied().reduce(f64::max);
    let condition_tags = weather_condition_tags(connection, period, &condition_counts)?;

    Ok(AnalyticsWeather {
        overview: AnalyticsWeatherOverview {
            total_entries,
            entries_with_weather,
            coverage_percent: percent(entries_with_weather, total_entries),
            unique_conditions: condition_counts.len() as i64,
            average_temp_c,
            min_temp_c,
            max_temp_c,
            most_common_condition,
        },
        temperature_buckets,
        trend: trend
            .into_iter()
            .map(|(date, value)| AnalyticsWeatherTrendPoint {
                date,
                average_temp_c: value.average_temp(),
                average_humidity: value.average_humidity(),
                average_wind_kph: value.average_wind(),
                entry_count: value.entry_count,
            })
            .collect(),
        condition_mood,
        temperature_mood_correlation: pearson_correlation(&temperature_mood_points),
        condition_tags,
    })
}

fn weather_condition_tags(
    connection: &Connection,
    period: &AnalyticsPeriodRequest,
    condition_counts: &HashMap<String, i64>,
) -> Result<Vec<AnalyticsWeatherConditionTags>> {
    if condition_counts.is_empty()
        || !table_exists(connection, "entry_tags")?
        || !table_exists(connection, "tags")?
    {
        return Ok(Vec::new());
    }
    let filter = period_filter("e", period);
    let sql = format!(
        "SELECT trim(pel.weather_condition), t.name, COUNT(DISTINCT e.id)
         FROM entries e
         JOIN plugin_entry_locations pel ON pel.entry_uuid = e.uuid
         JOIN entry_tags et ON et.entry_id = e.id
         JOIN tags t ON t.id = et.tag_id
         {}
         AND NULLIF(trim(pel.weather_condition), '') IS NOT NULL
         GROUP BY trim(pel.weather_condition), t.name",
        filter.where_sql
    );
    let mut statement = connection.prepare(&sql)?;
    let values = statement
        .query_map(params_from_iter(filter.params), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut grouped: HashMap<String, Vec<AnalyticsWeatherTag>> = HashMap::new();
    for (condition, label, count) in values {
        let total = condition_counts
            .get(&condition)
            .copied()
            .unwrap_or_default();
        grouped
            .entry(condition)
            .or_default()
            .push(AnalyticsWeatherTag {
                label,
                count,
                percent: percent(count, total),
            });
    }
    let mut result = grouped
        .into_iter()
        .map(|(condition, mut top_tags)| {
            top_tags.sort_by(|left, right| {
                right
                    .count
                    .cmp(&left.count)
                    .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
            });
            top_tags.truncate(3);
            AnalyticsWeatherConditionTags {
                entry_count: condition_counts
                    .get(&condition)
                    .copied()
                    .unwrap_or_default(),
                condition,
                top_tags,
            }
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right.entry_count.cmp(&left.entry_count).then_with(|| {
            left.condition
                .to_lowercase()
                .cmp(&right.condition.to_lowercase())
        })
    });
    result.truncate(6);
    Ok(result)
}

fn empty_weather_analytics(total_entries: i64) -> AnalyticsWeather {
    AnalyticsWeather {
        overview: AnalyticsWeatherOverview {
            total_entries,
            entries_with_weather: 0,
            coverage_percent: 0.0,
            unique_conditions: 0,
            average_temp_c: None,
            min_temp_c: None,
            max_temp_c: None,
            most_common_condition: None,
        },
        temperature_buckets: weather_temperature_buckets(),
        trend: Vec::new(),
        condition_mood: Vec::new(),
        temperature_mood_correlation: None,
        condition_tags: Vec::new(),
    }
}

fn weather_temperature_buckets() -> Vec<AnalyticsWeatherTemperatureBucket> {
    [
        ("below-0", "Below 0 C"),
        ("0-5", "0-5 C"),
        ("5-10", "5-10 C"),
        ("10-15", "10-15 C"),
        ("15-20", "15-20 C"),
        ("20-25", "20-25 C"),
        ("25-30", "25-30 C"),
        ("30-plus", "30+ C"),
    ]
    .into_iter()
    .map(|(key, label)| AnalyticsWeatherTemperatureBucket {
        key: key.to_string(),
        label: label.to_string(),
        count: 0,
    })
    .collect()
}

fn weather_temperature_bucket_matches(key: &str, temp: f64) -> bool {
    match key {
        "below-0" => temp < 0.0,
        "0-5" => (0.0..5.0).contains(&temp),
        "5-10" => (5.0..10.0).contains(&temp),
        "10-15" => (10.0..15.0).contains(&temp),
        "15-20" => (15.0..20.0).contains(&temp),
        "20-25" => (20.0..25.0).contains(&temp),
        "25-30" => (25.0..30.0).contains(&temp),
        "30-plus" => temp >= 30.0,
        _ => false,
    }
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

#[derive(Debug, Clone, Default)]
struct MoodBucket {
    sum: f64,
    count: i64,
    moods: HashMap<String, i64>,
}

impl MoodBucket {
    fn add(&mut self, mood: &str, score: f64) {
        self.sum += score;
        self.count += 1;
        *self.moods.entry(mood.to_string()).or_insert(0) += 1;
    }

    fn average(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum / self.count as f64)
    }

    fn top_mood(&self) -> Option<String> {
        self.moods
            .iter()
            .max_by(|left, right| {
                left.1
                    .cmp(right.1)
                    .then_with(|| right.0.to_lowercase().cmp(&left.0.to_lowercase()))
            })
            .map(|(mood, _)| mood.clone())
    }
}

#[derive(Debug, Clone, Default)]
struct WeatherTrendAccumulator {
    entry_count: i64,
    temp_sum: f64,
    temp_count: i64,
    humidity_sum: f64,
    humidity_count: i64,
    wind_sum: f64,
    wind_count: i64,
}

impl WeatherTrendAccumulator {
    fn add_temp(&mut self, value: Option<f64>) {
        if let Some(value) = value {
            self.temp_sum += value;
            self.temp_count += 1;
        }
    }

    fn add_humidity(&mut self, value: Option<f64>) {
        if let Some(value) = value {
            self.humidity_sum += value;
            self.humidity_count += 1;
        }
    }

    fn add_wind(&mut self, value: Option<f64>) {
        if let Some(value) = value {
            self.wind_sum += value;
            self.wind_count += 1;
        }
    }

    fn average_temp(&self) -> Option<f64> {
        average_sum(self.temp_sum, self.temp_count)
    }

    fn average_humidity(&self) -> Option<f64> {
        average_sum(self.humidity_sum, self.humidity_count)
    }

    fn average_wind(&self) -> Option<f64> {
        average_sum(self.wind_sum, self.wind_count)
    }
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

fn mood_sentiment_summary(
    rows: &[EntryStatsRow],
    mood_sentiments: &HashMap<String, f64>,
) -> MoodSentimentSummary {
    rows.iter()
        .filter_map(|row| row.mood.as_deref())
        .filter_map(|mood| mood_sentiment::score_from_catalog(mood_sentiments, mood))
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

fn percent(value: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}

fn average_sum(sum: f64, count: i64) -> Option<f64> {
    (count > 0).then_some(sum / count as f64)
}

fn average_f64(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then_some(values.iter().sum::<f64>() / values.len() as f64)
}

fn pearson_correlation(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 3 {
        return None;
    }
    let count = points.len() as f64;
    let mean_x = points.iter().map(|point| point.0).sum::<f64>() / count;
    let mean_y = points.iter().map(|point| point.1).sum::<f64>() / count;
    let numerator = points
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let denominator_x = points
        .iter()
        .map(|(x, _)| (x - mean_x).powi(2))
        .sum::<f64>();
    let denominator_y = points
        .iter()
        .map(|(_, y)| (y - mean_y).powi(2))
        .sum::<f64>();
    let denominator = (denominator_x * denominator_y).sqrt();
    (denominator > f64::EPSILON).then_some(numerator / denominator)
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
        assert_eq!(response.capture_sources.mobile_entries, 1);
        assert_eq!(response.capture_sources.desktop_entries, 3);
        assert_eq!(response.mood_trend.len(), 3);
        assert_eq!(response.mood_timing.time_of_day[0].mood_count, 3);
        assert_eq!(response.weather.overview.entries_with_weather, 2);
        assert_eq!(
            response.weather.overview.most_common_condition.as_deref(),
            Some("Rain")
        );
        assert_eq!(
            response
                .weather
                .temperature_buckets
                .iter()
                .find(|bucket| bucket.key == "below-0")
                .expect("freezing bucket")
                .count,
            1
        );
        assert_eq!(response.weather.condition_tags[0].top_tags.len(), 1);
    }

    #[test]
    fn dashboard_best_of_uses_all_time_visible_entries() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_stats_fixture(temp_dir.path());

        let response = get_dashboard_best_of_for_database(&db_path).expect("dashboard best of");

        assert_eq!(response.total_words, 13);

        let most_entries_day = response.most_entries_day.expect("most entries day");
        assert_eq!(most_entries_day.date, "2026-01-02");
        assert_eq!(most_entries_day.entry_count, 2);
        assert_eq!(most_entries_day.word_count, 6);

        let most_words_day = response.most_words_day.expect("most words day");
        assert_eq!(most_words_day.date, "2026-01-02");
        assert_eq!(most_words_day.word_count, 6);

        let most_tagged_entry = response.most_tagged_entry.expect("most tagged entry");
        assert_eq!(most_tagged_entry.entry_id, 3);
        assert_eq!(most_tagged_entry.tag_count, 1);

        let longest_entry = response.longest_entry.expect("longest entry");
        assert_eq!(longest_entry.entry_id, 4);
        assert_eq!(longest_entry.word_count, 3);

        let most_entries_month = response.most_entries_month.expect("most entries month");
        assert_eq!(most_entries_month.period, "2026-01");
        assert_eq!(most_entries_month.entry_count, 3);

        let most_words_month = response.most_words_month.expect("most words month");
        assert_eq!(most_words_month.period, "2026-01");
        assert_eq!(most_words_month.word_count, 9);
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

    #[test]
    fn analytics_and_calendar_use_custom_mood_sentiments() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_stats_fixture(temp_dir.path());
        let connection = Connection::open(&db_path).expect("database");
        connection
            .execute_batch(
                "CREATE TABLE mood_catalog (
                    name TEXT PRIMARY KEY COLLATE NOCASE,
                    sentiment_score REAL NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                INSERT INTO mood_catalog (name, sentiment_score, created_at, updated_at)
                VALUES ('focused', 0.5, '2026-07-23 10:00:00', '2026-07-23 10:00:00');",
            )
            .expect("custom mood");
        drop(connection);

        let analytics = get_analytics_for_database(&db_path, AnalyticsPeriodRequest::default())
            .expect("analytics");
        assert_close(analytics.overview.average_mood_sentiment, 2.0 / 3.0);
        assert_close(analytics.monthly_trend[0].average_mood_sentiment, 0.75);

        let calendar = get_writing_calendar_for_database(&db_path, Some(2026)).expect("calendar");
        let focused_day = calendar
            .days
            .iter()
            .find(|day| day.date == "2026-01-01")
            .expect("focused day");
        assert_close(focused_day.average_mood_sentiment, 0.5);
    }

    #[test]
    fn wrapped_builds_completed_month_stats_and_lifetime_callouts() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_stats_fixture(temp_dir.path());

        let response = get_wrapped_for_database(
            &db_path,
            "month",
            Some("2026-01"),
            NaiveDate::from_ymd_opt(2026, 3, 15).expect("date"),
        )
        .expect("wrapped");

        assert_eq!(response.anchor, "2026-01");
        assert_eq!(response.range.label, "January 2026");
        assert_eq!(response.range.day_count, 31);
        assert_eq!(response.summary.entries, 3);
        assert_eq!(response.summary.words, 9);
        assert_eq!(response.summary.active_days, 2);
        assert_eq!(response.highlights.top_tag.expect("top tag").label, "work");
        assert_eq!(
            response.records.busiest_day.expect("busiest day").date,
            "2026-01-02"
        );
        assert_eq!(response.charts.activity.len(), 31);
        assert_eq!(
            response
                .charts
                .activity
                .iter()
                .find(|point| point.period == "2026-01-02")
                .expect("activity point")
                .entries,
            2
        );
        assert_eq!(response.navigation.next_anchor.as_deref(), Some("2026-02"));
        assert!(response
            .personal_best_badges
            .iter()
            .any(|badge| badge.id == "best_notes_day"));
        assert!(response
            .personal_best_badges
            .iter()
            .any(|badge| badge.id == "best_words_day"));
    }

    #[test]
    fn wrapped_uses_monthly_activity_for_completed_years() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_stats_fixture(temp_dir.path());

        let response = get_wrapped_for_database(
            &db_path,
            "year",
            None,
            NaiveDate::from_ymd_opt(2027, 4, 10).expect("date"),
        )
        .expect("wrapped");

        assert_eq!(response.anchor, "2026");
        assert_eq!(response.charts.activity_granularity, "month");
        assert_eq!(response.charts.activity.len(), 12);
        assert_eq!(
            response
                .charts
                .activity
                .iter()
                .find(|point| point.period == "2026-01")
                .expect("january")
                .entries,
            3
        );
        assert_eq!(
            response
                .charts
                .activity
                .iter()
                .find(|point| point.period == "2026-02")
                .expect("february")
                .entries,
            1
        );
        assert!(response.navigation.is_latest);
        assert!(response.navigation.next_anchor.is_none());
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
                    source TEXT,
                    weather_condition TEXT,
                    weather_temp_c REAL,
                    weather_temp_f REAL,
                    weather_humidity INTEGER,
                    weather_wind_kph REAL,
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
                    (entry_uuid, latitude, longitude, place_name, source, weather_condition, weather_temp_c, weather_temp_f, weather_humidity, weather_wind_kph, created_at)
                VALUES ('entry_one', 69.0, 18.0, 'Tromso', 'mobile', 'Snow', -2, 28, 82, 14, '2026-01-01 08:00'),
                       ('entry_two', 59.0, 10.0, 'Oslo', 'manual', 'Rain', 4, 39, 75, 9, '2026-01-02 08:00');
                ",
            )
            .expect("fixture");
        drop(connection);
        db_path
    }
}
