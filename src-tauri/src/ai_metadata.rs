use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Map, Value as JsonValue};

use crate::{
    ai_config, ai_providers, db,
    models::{AiEntryMetadataSuggestionRequest, AiEntryMetadataSuggestionResponse},
};

const MAX_TITLE_CHARS: usize = 120;
const MAX_SUMMARY_CHARS: usize = 320;

pub fn suggest_ai_entry_metadata(
    input: AiEntryMetadataSuggestionRequest,
) -> Result<AiEntryMetadataSuggestionResponse> {
    suggest_ai_entry_metadata_for_database(&db::resolve_database_path(), input)
}

pub(crate) fn suggest_ai_entry_metadata_for_database(
    db_path: &Path,
    input: AiEntryMetadataSuggestionRequest,
) -> Result<AiEntryMetadataSuggestionResponse> {
    let text = normalize_required_text(&input.text, "Entry text")?;
    let content_format = normalize_content_format(input.content_format.as_deref());
    let settings = ai_config::get_ai_settings_for_database(db_path)?;
    let provider = normalize_provider(input.cloud_provider.as_deref(), &settings.cloud_provider)?;
    let model = ai_config::model_for_provider(&settings, &provider, input.model.as_deref())?;
    let api_key = ai_config::api_key_for_provider(db_path, &provider)?;

    let raw = ai_providers::generate_text(ai_providers::ProviderGenerateRequest {
        provider: provider.clone(),
        model: model.clone(),
        api_key: api_key.value,
        system_prompt: metadata_system_prompt(),
        prompt: metadata_user_prompt(&text, &content_format),
        max_output_tokens: Some(700),
        json_schema: Some(ai_providers::ProviderJsonSchema {
            name: "capsule_entry_metadata".to_string(),
            schema: metadata_schema(),
        }),
    })?;
    let parsed = parse_metadata_response(&raw)?;

    Ok(AiEntryMetadataSuggestionResponse {
        title: parsed.title,
        summary: parsed.summary,
        cloud_provider: provider,
        model,
        warnings: parsed.warnings,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedMetadata {
    title: Option<String>,
    summary: Option<String>,
    warnings: Vec<String>,
}

fn metadata_system_prompt() -> String {
    [
        "You create concise metadata for private journal posts.",
        "Return only JSON with exactly two keys: title and summary.",
        "title must be a string or null, at most 120 characters.",
        "summary must be a string or null, at most 320 characters.",
        "Do not invent facts, people, places, tags, moods, or dates.",
    ]
    .join(" ")
}

fn metadata_user_prompt(text: &str, content_format: &str) -> String {
    format!(
        "Generate metadata for this {content_format} journal post.\n\nPost:\n```{content_format}\n{text}\n```"
    )
}

fn metadata_schema() -> JsonValue {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "title": {
                "anyOf": [
                    { "type": "string" },
                    { "type": "null" }
                ]
            },
            "summary": {
                "anyOf": [
                    { "type": "string" },
                    { "type": "null" }
                ]
            }
        },
        "required": ["title", "summary"]
    })
}

fn parse_metadata_response(raw: &str) -> Result<ParsedMetadata> {
    let json_text = strip_json_fence(raw)?;
    let value: JsonValue = serde_json::from_str(&json_text)
        .context("AI provider did not return valid JSON metadata.")?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("AI metadata response must be a JSON object."))?;
    let (title, title_truncated) = normalize_metadata_field(object, "title", MAX_TITLE_CHARS)?;
    let (summary, summary_truncated) =
        normalize_metadata_field(object, "summary", MAX_SUMMARY_CHARS)?;
    let mut warnings = Vec::new();
    if title_truncated {
        warnings.push(format!("Title was capped at {MAX_TITLE_CHARS} characters."));
    }
    if summary_truncated {
        warnings.push(format!(
            "Summary was capped at {MAX_SUMMARY_CHARS} characters."
        ));
    }

    Ok(ParsedMetadata {
        title,
        summary,
        warnings,
    })
}

fn strip_json_fence(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return Ok(trimmed.to_string());
    }

    let without_opening = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim_start();
    let Some(without_closing) = without_opening.strip_suffix("```") else {
        return Err(anyhow!("AI metadata JSON fence was not closed."));
    };
    Ok(without_closing.trim().to_string())
}

fn normalize_metadata_field(
    object: &Map<String, JsonValue>,
    field: &str,
    limit: usize,
) -> Result<(Option<String>, bool)> {
    let Some(value) = object.get(field) else {
        return Ok((None, false));
    };
    if value.is_null() {
        return Ok((None, false));
    }
    let Some(text) = value.as_str() else {
        return Err(anyhow!("AI metadata {field} must be a string or null."));
    };
    let Some(normalized) = normalize_optional(text) else {
        return Ok((None, false));
    };
    let char_count = normalized.chars().count();
    if char_count <= limit {
        return Ok((Some(normalized), false));
    }
    Ok((Some(normalized.chars().take(limit).collect()), true))
}

fn normalize_provider(value: Option<&str>, fallback: &str) -> Result<String> {
    match value
        .and_then(normalize_optional)
        .unwrap_or_else(|| fallback.to_string())
        .to_lowercase()
        .as_str()
    {
        "gemini" => Ok("gemini".to_string()),
        "openai" => Ok("openai".to_string()),
        "openrouter" => Ok("openrouter".to_string()),
        _ => Err(anyhow!(
            "Cloud provider must be gemini, openai, or openrouter."
        )),
    }
}

fn normalize_content_format(value: Option<&str>) -> String {
    match value
        .and_then(normalize_optional)
        .unwrap_or_else(|| "markdown".to_string())
        .to_lowercase()
        .as_str()
    {
        "plain" => "plain".to_string(),
        _ => "markdown".to_string(),
    }
}

fn normalize_required_text(value: &str, label: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("{label} is required."));
    }
    Ok(trimmed.to_string())
}

fn normalize_optional(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_metadata_json() {
        let parsed = parse_metadata_response(
            r#"```json
            {"title":"A quiet milestone","summary":"The work started to feel usable."}
            ```"#,
        )
        .expect("metadata");

        assert_eq!(parsed.title.as_deref(), Some("A quiet milestone"));
        assert_eq!(
            parsed.summary.as_deref(),
            Some("The work started to feel usable.")
        );
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn rejects_non_string_metadata_fields() {
        let parsed = parse_metadata_response(r#"{"title":["bad"],"summary":null}"#);

        assert!(parsed.is_err());
    }

    #[test]
    fn caps_metadata_fields() {
        let long_title = "t".repeat(MAX_TITLE_CHARS + 10);
        let long_summary = "s".repeat(MAX_SUMMARY_CHARS + 10);
        let parsed = parse_metadata_response(&format!(
            r#"{{"title":"{long_title}","summary":"{long_summary}"}}"#
        ))
        .expect("metadata");

        assert_eq!(parsed.title.unwrap().chars().count(), MAX_TITLE_CHARS);
        assert_eq!(parsed.summary.unwrap().chars().count(), MAX_SUMMARY_CHARS);
        assert_eq!(parsed.warnings.len(), 2);
    }
}
