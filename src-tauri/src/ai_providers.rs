use std::{
    io::{BufRead, BufReader},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use serde_json::{json, Value as JsonValue};

#[derive(Debug, Clone)]
pub(crate) struct ProviderChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderStreamRequest {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub system_prompt: String,
    pub messages: Vec<ProviderChatMessage>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderJsonSchema {
    pub name: String,
    pub schema: JsonValue,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderGenerateRequest {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub system_prompt: String,
    pub prompt: String,
    pub max_output_tokens: Option<u32>,
    pub json_schema: Option<ProviderJsonSchema>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderStreamOutcome {
    Complete,
    Cancelled,
}

pub(crate) fn stream_text(
    request: ProviderStreamRequest,
    cancelled: Arc<AtomicBool>,
    on_chunk: impl FnMut(&str) -> Result<()>,
) -> Result<ProviderStreamOutcome> {
    match request.provider.as_str() {
        "openai" => stream_openai(request, cancelled, on_chunk),
        "gemini" => stream_gemini(request, cancelled, on_chunk),
        "openrouter" => stream_openrouter(request, cancelled, on_chunk),
        _ => Err(anyhow!("Unsupported AI provider.")),
    }
}

pub(crate) fn generate_text(request: ProviderGenerateRequest) -> Result<String> {
    match request.provider.as_str() {
        "openai" => generate_openai(request),
        "gemini" => generate_gemini(request),
        "openrouter" => generate_openrouter(request),
        _ => Err(anyhow!("Unsupported AI provider.")),
    }
}

fn stream_openai(
    request: ProviderStreamRequest,
    cancelled: Arc<AtomicBool>,
    mut on_chunk: impl FnMut(&str) -> Result<()>,
) -> Result<ProviderStreamOutcome> {
    let input = request
        .messages
        .iter()
        .filter(|message| !message.content.trim().is_empty())
        .map(|message| {
            json!({
                "role": if message.role == "assistant" { "assistant" } else { "user" },
                "content": message.content,
            })
        })
        .collect::<Vec<_>>();
    let response = client()?
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(&request.api_key)
        .json(&json!({
            "model": request.model,
            "instructions": request.system_prompt,
            "input": input,
            "stream": true,
            "store": false,
        }))
        .send()
        .context("OpenAI request failed before streaming started.")?;
    let response = ensure_success(response, "OpenAI")?;
    stream_sse(response, cancelled, |event, data| {
        if let Some(chunk) = openai_text_delta(event, data)? {
            on_chunk(&chunk)?;
        }
        Ok(())
    })
}

fn generate_openai(request: ProviderGenerateRequest) -> Result<String> {
    let mut body = json!({
        "model": request.model,
        "instructions": request.system_prompt,
        "input": [{
            "role": "user",
            "content": request.prompt,
        }],
        "stream": false,
        "store": false,
    });
    if let Some(max_output_tokens) = request.max_output_tokens {
        body["max_output_tokens"] = json!(max_output_tokens);
    }
    if let Some(schema) = request.json_schema {
        body["text"] = json!({
            "format": {
                "type": "json_schema",
                "name": schema.name,
                "strict": true,
                "schema": schema.schema,
            }
        });
    }

    let response = client()?
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(&request.api_key)
        .json(&body)
        .send()
        .context("OpenAI request failed before a response was returned.")?;
    let value = read_json_response(response, "OpenAI")?;
    openai_response_text(&value)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| anyhow!("OpenAI completed without returning text."))
}

fn stream_gemini(
    request: ProviderStreamRequest,
    cancelled: Arc<AtomicBool>,
    mut on_chunk: impl FnMut(&str) -> Result<()>,
) -> Result<ProviderStreamOutcome> {
    let contents = request
        .messages
        .iter()
        .filter(|message| !message.content.trim().is_empty())
        .map(|message| {
            json!({
                "role": if message.role == "assistant" { "model" } else { "user" },
                "parts": [{ "text": message.content }],
            })
        })
        .collect::<Vec<_>>();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
        request.model
    );
    let response = client()?
        .post(url)
        .header("x-goog-api-key", &request.api_key)
        .json(&json!({
            "systemInstruction": {
                "parts": [{ "text": request.system_prompt }]
            },
            "contents": contents,
        }))
        .send()
        .context("Gemini request failed before streaming started.")?;
    let response = ensure_success(response, "Gemini")?;
    stream_sse(response, cancelled, |_event, data| {
        if let Some(chunk) = gemini_text_delta(data)? {
            on_chunk(&chunk)?;
        }
        Ok(())
    })
}

fn generate_gemini(request: ProviderGenerateRequest) -> Result<String> {
    let mut body = json!({
        "systemInstruction": {
            "parts": [{ "text": request.system_prompt }]
        },
        "contents": [{
            "role": "user",
            "parts": [{ "text": request.prompt }]
        }],
        "generationConfig": {
            "temperature": 0.2,
            "responseMimeType": if request.json_schema.is_some() { "application/json" } else { "text/plain" },
        }
    });
    if let Some(max_output_tokens) = request.max_output_tokens {
        body["generationConfig"]["maxOutputTokens"] = json!(max_output_tokens);
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        request.model
    );
    let response = client()?
        .post(url)
        .header("x-goog-api-key", &request.api_key)
        .json(&body)
        .send()
        .context("Gemini request failed before a response was returned.")?;
    let value = read_json_response(response, "Gemini")?;
    gemini_generated_text(&value)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| anyhow!("Gemini completed without returning text."))
}

fn stream_openrouter(
    request: ProviderStreamRequest,
    cancelled: Arc<AtomicBool>,
    mut on_chunk: impl FnMut(&str) -> Result<()>,
) -> Result<ProviderStreamOutcome> {
    let mut messages = vec![json!({
        "role": "system",
        "content": request.system_prompt,
    })];
    messages.extend(
        request
            .messages
            .iter()
            .filter(|message| !message.content.trim().is_empty())
            .map(|message| {
                json!({
                    "role": if message.role == "assistant" { "assistant" } else { "user" },
                    "content": message.content,
                })
            }),
    );
    let response = client()?
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(&request.api_key)
        .header("HTTP-Referer", "https://capsule.local")
        .header("X-OpenRouter-Title", "Capsule")
        .json(&json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        }))
        .send()
        .context("OpenRouter request failed before streaming started.")?;
    let response = ensure_success(response, "OpenRouter")?;
    stream_sse(response, cancelled, |_event, data| {
        if let Some(chunk) = openrouter_text_delta(data)? {
            on_chunk(&chunk)?;
        }
        Ok(())
    })
}

fn generate_openrouter(request: ProviderGenerateRequest) -> Result<String> {
    let mut body = json!({
        "model": request.model,
        "messages": [
            {
                "role": "system",
                "content": request.system_prompt,
            },
            {
                "role": "user",
                "content": request.prompt,
            }
        ],
        "temperature": 0.2,
    });
    if let Some(max_output_tokens) = request.max_output_tokens {
        body["max_tokens"] = json!(max_output_tokens);
    }
    if request.json_schema.is_some() {
        body["response_format"] = json!({ "type": "json_object" });
    }

    let response = client()?
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(&request.api_key)
        .header("HTTP-Referer", "https://capsule.local")
        .header("X-OpenRouter-Title", "Capsule")
        .json(&body)
        .send()
        .context("OpenRouter request failed before a response was returned.")?;
    let value = read_json_response(response, "OpenRouter")?;
    openrouter_generated_text(&value)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| anyhow!("OpenRouter completed without returning text."))
}

fn client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .context("failed to build AI provider HTTP client")
}

fn read_json_response(response: Response, provider: &str) -> Result<JsonValue> {
    ensure_success(response, provider)?
        .json::<JsonValue>()
        .with_context(|| format!("{provider} returned malformed JSON."))
}

fn ensure_success(response: Response, provider: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().unwrap_or_default();
    Err(provider_http_error(provider, status, &body))
}

fn provider_http_error(provider: &str, status: StatusCode, body: &str) -> anyhow::Error {
    let body_hint = provider_error_hint(body);
    let message = if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        format!("{provider} rejected the configured API key.")
    } else if status == StatusCode::TOO_MANY_REQUESTS {
        format!("{provider} rate limit reached. Try again later.")
    } else if status == StatusCode::BAD_REQUEST {
        format!("{provider} rejected the model or prompt request.")
    } else if status.is_server_error() {
        format!("{provider} is unavailable right now.")
    } else {
        format!("{provider} request failed with HTTP {status}.")
    };
    if body_hint.is_empty() {
        anyhow!(message)
    } else {
        anyhow!("{message} {body_hint}")
    }
}

fn provider_error_hint(body: &str) -> String {
    if body.trim().is_empty() {
        return String::new();
    }
    serde_json::from_str::<JsonValue>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.pointer("/message"))
                .and_then(JsonValue::as_str)
                .map(|value| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
        .map(|value| format!("Provider detail: {}", truncate(&value, 240)))
        .unwrap_or_default()
}

fn stream_sse(
    response: Response,
    cancelled: Arc<AtomicBool>,
    mut handle_event: impl FnMut(Option<&str>, &str) -> Result<()>,
) -> Result<ProviderStreamOutcome> {
    let mut reader = BufReader::new(response);
    let mut line = String::new();
    let mut event: Option<String> = None;
    let mut data_lines: Vec<String> = Vec::new();

    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(ProviderStreamOutcome::Cancelled);
        }
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .context("AI provider stream disconnected.")?;
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            dispatch_sse_event(&event, &data_lines, &mut handle_event)?;
            event = None;
            data_lines.clear();
            continue;
        }
        if trimmed.starts_with(':') {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("event:") {
            event = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("data:") {
            data_lines.push(value.trim_start().to_string());
        }
    }

    dispatch_sse_event(&event, &data_lines, &mut handle_event)?;
    if cancelled.load(Ordering::Relaxed) {
        Ok(ProviderStreamOutcome::Cancelled)
    } else {
        Ok(ProviderStreamOutcome::Complete)
    }
}

fn dispatch_sse_event(
    event: &Option<String>,
    data_lines: &[String],
    handle_event: &mut impl FnMut(Option<&str>, &str) -> Result<()>,
) -> Result<()> {
    if data_lines.is_empty() {
        return Ok(());
    }
    let data = data_lines.join("\n");
    if data.trim() == "[DONE]" {
        return Ok(());
    }
    handle_event(event.as_deref(), &data)
}

pub(crate) fn openai_text_delta(event: Option<&str>, data: &str) -> Result<Option<String>> {
    let value: JsonValue = serde_json::from_str(data).context("Malformed OpenAI stream event.")?;
    if value.get("type").and_then(JsonValue::as_str) == Some("error") {
        return Err(anyhow!(
            "{}",
            value
                .pointer("/error/message")
                .and_then(JsonValue::as_str)
                .unwrap_or("OpenAI returned a streaming error.")
        ));
    }
    if event == Some("response.output_text.delta")
        || value.get("type").and_then(JsonValue::as_str) == Some("response.output_text.delta")
    {
        return Ok(value
            .get("delta")
            .and_then(JsonValue::as_str)
            .map(str::to_string));
    }
    Ok(None)
}

pub(crate) fn gemini_text_delta(data: &str) -> Result<Option<String>> {
    let value: JsonValue = serde_json::from_str(data).context("Malformed Gemini stream event.")?;
    if let Some(message) = value.pointer("/error/message").and_then(JsonValue::as_str) {
        return Err(anyhow!("{message}"));
    }
    Ok(value
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(JsonValue::as_str)
        .map(str::to_string))
}

pub(crate) fn openrouter_text_delta(data: &str) -> Result<Option<String>> {
    let value: JsonValue =
        serde_json::from_str(data).context("Malformed OpenRouter stream event.")?;
    if let Some(message) = value.pointer("/error/message").and_then(JsonValue::as_str) {
        return Err(anyhow!("{message}"));
    }
    Ok(value
        .pointer("/choices/0/delta/content")
        .or_else(|| value.pointer("/choices/0/message/content"))
        .and_then(JsonValue::as_str)
        .map(str::to_string))
}

pub(crate) fn openai_response_text(value: &JsonValue) -> Option<String> {
    if let Some(text) = value.get("output_text").and_then(JsonValue::as_str) {
        return Some(text.to_string());
    }
    let output = value.get("output")?.as_array()?;
    let chunks = output
        .iter()
        .flat_map(|item| {
            item.get("content")
                .and_then(JsonValue::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|content| {
            content
                .get("text")
                .or_else(|| content.get("json"))
                .and_then(JsonValue::as_str)
        })
        .collect::<Vec<_>>();
    (!chunks.is_empty()).then(|| chunks.join(""))
}

pub(crate) fn gemini_generated_text(value: &JsonValue) -> Option<String> {
    value
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

pub(crate) fn openrouter_generated_text(value: &JsonValue) -> Option<String> {
    let content = value.pointer("/choices/0/message/content")?;
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }
    let chunks = content
        .as_array()?
        .iter()
        .filter_map(|item| {
            item.get("text")
                .or_else(|| item.get("content"))
                .and_then(JsonValue::as_str)
        })
        .collect::<Vec<_>>();
    (!chunks.is_empty()).then(|| chunks.join(""))
}

fn truncate(value: &str, limit: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= limit {
        return normalized;
    }
    format!(
        "{}...",
        normalized
            .chars()
            .take(limit.saturating_sub(3))
            .collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn parses_openai_text_delta() {
        let chunk = openai_text_delta(
            Some("response.output_text.delta"),
            r#"{"type":"response.output_text.delta","delta":"Hello"}"#,
        )
        .expect("delta");

        assert_eq!(chunk.as_deref(), Some("Hello"));
    }

    #[test]
    fn parses_gemini_text_delta() {
        let chunk = gemini_text_delta(r#"{"candidates":[{"content":{"parts":[{"text":"Hej"}]}}]}"#)
            .expect("delta");

        assert_eq!(chunk.as_deref(), Some("Hej"));
    }

    #[test]
    fn parses_openrouter_text_delta() {
        let chunk =
            openrouter_text_delta(r#"{"choices":[{"delta":{"content":"Yo"}}]}"#).expect("delta");

        assert_eq!(chunk.as_deref(), Some("Yo"));
    }

    #[test]
    fn extracts_openai_response_output_text() {
        let value = json!({
            "output": [{
                "type": "message",
                "content": [
                    { "type": "output_text", "text": "{\"title\":\"Hi\"}" }
                ]
            }]
        });

        assert_eq!(
            openai_response_text(&value).as_deref(),
            Some(r#"{"title":"Hi"}"#)
        );
    }

    #[test]
    fn extracts_non_streaming_provider_text() {
        let gemini = json!({
            "candidates": [{
                "content": { "parts": [{ "text": "{\"summary\":\"Hej\"}" }] }
            }]
        });
        let openrouter = json!({
            "choices": [{
                "message": { "content": "{\"summary\":\"Yo\"}" }
            }]
        });

        assert_eq!(
            gemini_generated_text(&gemini).as_deref(),
            Some(r#"{"summary":"Hej"}"#)
        );
        assert_eq!(
            openrouter_generated_text(&openrouter).as_deref(),
            Some(r#"{"summary":"Yo"}"#)
        );
    }

    #[test]
    #[ignore = "run with CAPSULE_AI_LIVE_SMOKE=1 to call live providers with synthetic context"]
    fn live_provider_smoke_uses_synthetic_context_only() {
        if std::env::var("CAPSULE_AI_LIVE_SMOKE").ok().as_deref() != Some("1") {
            return;
        }
        let temp = tempfile::tempdir().expect("tempdir");
        let providers = [
            ("gemini", "gemini-3.5-flash"),
            ("openai", "gpt-5.4-mini"),
            ("openrouter", "moonshotai/kimi-k2.5"),
        ];

        for (provider, model) in providers {
            let api_key =
                crate::ai_config::api_key_for_provider(temp.path(), provider).expect("api key");
            let mut output = String::new();
            stream_text(
                ProviderStreamRequest {
                    provider: provider.to_string(),
                    model: model.to_string(),
                    api_key: api_key.value,
                    system_prompt: "Synthetic Capsule AI smoke test. No journal entries are included. Reply with a short acknowledgement.".to_string(),
                    messages: vec![ProviderChatMessage {
                        role: "user".to_string(),
                        content: "Reply with OK and the provider name.".to_string(),
                    }],
                },
                Arc::new(AtomicBool::new(false)),
                |chunk| {
                    output.push_str(chunk);
                    Ok(())
                },
            )
            .unwrap_or_else(|error| panic!("{provider} live smoke failed: {error}"));
            assert!(
                !output.trim().is_empty(),
                "{provider} live smoke returned no text"
            );
        }
    }
}
