use std::{collections::HashMap, sync::OnceLock};

use serde::Deserialize;

const MOOD_SENTIMENT_JSON: &str = include_str!("../mood_sentiment.json");

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

pub fn score_for_mood(mood: &str) -> Option<f64> {
    lookup(mood).map(|sentiment| sentiment.sentiment_score)
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
}
