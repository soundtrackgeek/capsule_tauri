//! Shared mood sentiment lookup used by desktop settings and analytics.
//!
//! The catalog is bundled with the Capsule repository so a headless client
//! observes the same defaults as the desktop application.  User-defined
//! `mood_catalog` rows remain authoritative when a database connection is
//! supplied.

use std::{collections::HashMap, sync::OnceLock};

use anyhow::Result;
use rusqlite::Connection;
use serde::Deserialize;

// Keep one source of truth while the extracted core and desktop application
// are developed in the same repository.  The path is part of the Git
// dependency checkout and does not access a user's live configuration.
const MOOD_SENTIMENT_JSON: &str = include_str!("../../../src-tauri/mood_sentiment.json");

static MOOD_SENTIMENT: OnceLock<HashMap<String, MoodSentiment>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct MoodSentiment {
    pub sentiment_score: f64,
    #[allow(dead_code)]
    pub category: String,
    #[allow(dead_code)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct MoodSentimentFile {
    moods: HashMap<String, MoodSentiment>,
}

/// Return the bundled score for one mood, matching desktop case/whitespace
/// normalization.  Unknown or blank moods intentionally return `None`.
pub fn score_for_mood(mood: &str) -> Option<f64> {
    lookup(mood).map(|sentiment| sentiment.sentiment_score)
}

/// Load bundled scores and apply any user-defined catalog overrides.  This is
/// read-only; callers may use it on a query-only connection.
pub fn scores_for_database(connection: &Connection) -> Result<HashMap<String, f64>> {
    let mut scores = sentiments()
        .iter()
        .map(|(name, sentiment)| (name.clone(), sentiment.sentiment_score))
        .collect::<HashMap<_, _>>();
    let has_catalog = connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'mood_catalog'
        )",
        [],
        |row| row.get::<_, i64>(0),
    )? != 0;
    if !has_catalog {
        return Ok(scores);
    }

    let mut statement = connection.prepare("SELECT name, sentiment_score FROM mood_catalog")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;
    for row in rows {
        let (name, score) = row?;
        let name = name.trim().to_lowercase();
        if !name.is_empty() {
            scores.insert(name, score);
        }
    }
    Ok(scores)
}

/// Resolve a normalized mood against a previously loaded score map.
pub fn score_from_catalog(scores: &HashMap<String, f64>, mood: &str) -> Option<f64> {
    let normalized = mood.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    scores.get(&normalized).copied()
}

fn lookup(mood: &str) -> Option<&'static MoodSentiment> {
    let normalized = mood.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }

    sentiments().get(&normalized)
}

fn sentiments() -> &'static HashMap<String, MoodSentiment> {
    MOOD_SENTIMENT.get_or_init(|| {
        serde_json::from_str::<MoodSentimentFile>(MOOD_SENTIMENT_JSON)
            .expect("bundled mood sentiment data must be valid JSON")
            .moods
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_known_moods_case_insensitively() {
        assert_eq!(score_for_mood("Happy"), Some(1.0));
        assert_eq!(score_for_mood(" sad "), Some(-1.0));
        assert_eq!(score_for_mood("not-in-catalog"), None);
    }

    #[test]
    fn database_scores_override_bundled_values() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                "CREATE TABLE mood_catalog (
                    name TEXT PRIMARY KEY COLLATE NOCASE,
                    sentiment_score REAL NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                INSERT INTO mood_catalog (name, sentiment_score, created_at, updated_at)
                VALUES ('happy', 0.25, '2026-07-23 10:00:00', '2026-07-23 10:00:00');",
            )
            .expect("mood catalog");

        let scores = scores_for_database(&connection).expect("scores");
        assert_eq!(score_from_catalog(&scores, "Happy"), Some(0.25));
        assert_eq!(score_from_catalog(&scores, "sad"), Some(-1.0));
    }
}
