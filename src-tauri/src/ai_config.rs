use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, Utc};
use keyring_core::{Entry, Error as KeyringError};
use serde_json::{Map, Value as JsonValue};

use crate::{
    db,
    models::{
        AiApiKeyMutationResponse, AiApiKeyUpdateRequest, AiProviderStatus, AiSettings,
        AiSettingsUpdateRequest, ConfigMutationResponse,
    },
    settings,
};

const KEYRING_SERVICE: &str = "capsule-tauri.ai";
const DEFAULT_PROVIDER: &str = "gemini";

const GEMINI_MODELS: &[&str] = &["gemini-3.5-flash", "gemini-3.1-flash-lite-preview"];
const OPENAI_MODELS: &[&str] = &["gpt-5.4-mini", "gpt-5.4-nano"];
const OPENROUTER_MODELS: &[&str] = &[
    "z-ai/glm-5.2",
    "moonshotai/kimi-k2.5",
    "qwen/qwen3.7-plus",
    "deepseek/deepseek-v4-flash",
    "xiaomi/mimo-v2.5",
    "minimax/minimax-m3",
];

const DEFAULT_GEMINI_MODEL: &str = "gemini-3.5-flash";
const DEFAULT_OPENAI_MODEL: &str = "gpt-5.4-mini";
const DEFAULT_OPENROUTER_MODEL: &str = "moonshotai/kimi-k2.5";

#[derive(Debug, Clone, Copy)]
struct ProviderDefinition {
    provider: &'static str,
    label: &'static str,
    env_key: &'static str,
    models: &'static [&'static str],
}

const PROVIDERS: &[ProviderDefinition] = &[
    ProviderDefinition {
        provider: "gemini",
        label: "Google Gemini",
        env_key: "GEMINI_API_KEY",
        models: GEMINI_MODELS,
    },
    ProviderDefinition {
        provider: "openai",
        label: "OpenAI",
        env_key: "OPENAI_API_KEY",
        models: OPENAI_MODELS,
    },
    ProviderDefinition {
        provider: "openrouter",
        label: "OpenRouter",
        env_key: "OPENROUTER_API_KEY",
        models: OPENROUTER_MODELS,
    },
];

#[derive(Debug, Clone)]
struct ApiKeyPresence {
    configured: bool,
    source: Option<String>,
}

pub fn get_ai_settings() -> Result<AiSettings> {
    get_ai_settings_for_database(&db::resolve_database_path())
}

pub(crate) fn get_ai_settings_for_database(db_path: &Path) -> Result<AiSettings> {
    let config = settings::get_capsule_config_for_database(db_path)?;
    Ok(ai_settings_from_config(&config.values))
}

pub fn get_ai_provider_status() -> Result<Vec<AiProviderStatus>> {
    let db_path = db::resolve_database_path();
    let settings = get_ai_settings_for_database(&db_path)?;
    Ok(provider_statuses_for_database(&db_path, &settings))
}

pub(crate) fn active_provider_and_model_for_database(
    db_path: &Path,
) -> Result<(String, String, bool)> {
    let settings = get_ai_settings_for_database(db_path)?;
    let selected_model = selected_model_for_provider(&settings, &settings.cloud_provider);
    let configured = provider_definition(&settings.cloud_provider)
        .map(|definition| lookup_api_key_presence(db_path, definition).configured)
        .unwrap_or(false);
    Ok((settings.cloud_provider, selected_model, configured))
}

pub fn update_ai_settings(input: AiSettingsUpdateRequest) -> Result<ConfigMutationResponse> {
    update_ai_settings_for_database(&db::resolve_database_path(), input)
}

pub(crate) fn update_ai_settings_for_database(
    db_path: &Path,
    input: AiSettingsUpdateRequest,
) -> Result<ConfigMutationResponse> {
    let cloud_provider = validate_provider(&input.cloud_provider)?
        .provider
        .to_string();
    let gemini_model = validate_model("Gemini model", &input.gemini_model, GEMINI_MODELS)?;
    let openai_model = validate_model("OpenAI model", &input.openai_model, OPENAI_MODELS)?;
    let openrouter_model = validate_model(
        "OpenRouter model",
        &input.openrouter_model,
        OPENROUTER_MODELS,
    )?;
    let default_context_limit = match input.default_context_limit {
        Some(limit) if limit < 1 => {
            return Err(anyhow!(
                "Default context limit must be a positive integer or all."
            ));
        }
        other => other,
    };
    let default_since = normalize_date(input.default_since.as_deref(), "Default since")?;
    let default_until = normalize_date(input.default_until.as_deref(), "Default until")?;
    if let (Some(since), Some(until)) = (&default_since, &default_until) {
        if since > until {
            return Err(anyhow!("Default since cannot be after default until."));
        }
    }

    settings::mutate_capsule_config_for_database(db_path, "config.ai.set", |object| {
        object.insert(
            "cloud_provider".to_string(),
            JsonValue::String(cloud_provider),
        );
        object.insert("gemini_model".to_string(), JsonValue::String(gemini_model));
        object.insert("openai_model".to_string(), JsonValue::String(openai_model));
        object.insert(
            "openrouter_model".to_string(),
            JsonValue::String(openrouter_model),
        );
        object.insert(
            "ai_chat_context_limit".to_string(),
            JsonValue::String(
                default_context_limit
                    .map(|limit| limit.to_string())
                    .unwrap_or_else(|| "all".to_string()),
            ),
        );
        set_optional_config_string(object, "ai_chat_context_since", default_since);
        set_optional_config_string(object, "ai_chat_context_until", default_until);
        Ok(())
    })
}

pub fn set_ai_api_key(input: AiApiKeyUpdateRequest) -> Result<AiApiKeyMutationResponse> {
    let provider = validate_provider(&input.provider)?;
    let api_key = normalize_string(Some(&input.api_key))
        .ok_or_else(|| anyhow!("API key cannot be empty."))?;
    let entry = keyring_entry(provider.env_key)?;
    entry
        .set_password(&api_key)
        .with_context(|| format!("failed to save {} to OS credential store", provider.env_key))?;
    Ok(AiApiKeyMutationResponse {
        provider_status: provider_status_for_database(&db::resolve_database_path(), provider)?,
        completed_at: Utc::now().to_rfc3339(),
    })
}

pub fn clear_ai_api_key(provider: String) -> Result<AiApiKeyMutationResponse> {
    let provider = validate_provider(&provider)?;
    let entry = keyring_entry(provider.env_key)?;
    match entry.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to remove {} from OS credential store",
                    provider.env_key
                )
            });
        }
    }
    Ok(AiApiKeyMutationResponse {
        provider_status: provider_status_for_database(&db::resolve_database_path(), provider)?,
        completed_at: Utc::now().to_rfc3339(),
    })
}

fn provider_status_for_database(
    db_path: &Path,
    provider: ProviderDefinition,
) -> Result<AiProviderStatus> {
    let settings = get_ai_settings_for_database(db_path)?;
    let selected_model = selected_model_for_provider(&settings, provider.provider);
    Ok(build_provider_status(
        provider,
        selected_model,
        lookup_api_key_presence(db_path, provider),
    ))
}

fn provider_statuses_for_database(db_path: &Path, settings: &AiSettings) -> Vec<AiProviderStatus> {
    PROVIDERS
        .iter()
        .copied()
        .map(|provider| {
            build_provider_status(
                provider,
                selected_model_for_provider(settings, provider.provider),
                lookup_api_key_presence(db_path, provider),
            )
        })
        .collect()
}

fn build_provider_status(
    provider: ProviderDefinition,
    selected_model: String,
    presence: ApiKeyPresence,
) -> AiProviderStatus {
    AiProviderStatus {
        provider: provider.provider.to_string(),
        label: provider.label.to_string(),
        configured: presence.configured,
        selected_model,
        available_models: provider
            .models
            .iter()
            .map(|model| model.to_string())
            .collect(),
        missing_reason: (!presence.configured).then(|| {
            format!(
                "{} is not configured in the OS credential store, environment, or local .env.",
                provider.env_key
            )
        }),
        key_source: presence.source,
    }
}

fn ai_settings_from_config(values: &[crate::models::CapsuleConfigValue]) -> AiSettings {
    let values = values
        .iter()
        .map(|item| (item.key.to_lowercase(), item.value.clone()))
        .collect::<HashMap<_, _>>();
    let mut warnings = Vec::new();
    let cloud_provider = resolve_provider(values.get("cloud_provider").map(String::as_str))
        .unwrap_or_else(|| {
            warnings.push(format!(
                "Invalid cloud_provider; using default {DEFAULT_PROVIDER}."
            ));
            DEFAULT_PROVIDER.to_string()
        });
    let gemini_model = resolve_model(
        "gemini_model",
        values.get("gemini_model").map(String::as_str),
        GEMINI_MODELS,
        DEFAULT_GEMINI_MODEL,
        &mut warnings,
    );
    let openai_model = resolve_model(
        "openai_model",
        values.get("openai_model").map(String::as_str),
        OPENAI_MODELS,
        DEFAULT_OPENAI_MODEL,
        &mut warnings,
    );
    let openrouter_model = resolve_model(
        "openrouter_model",
        values.get("openrouter_model").map(String::as_str),
        OPENROUTER_MODELS,
        DEFAULT_OPENROUTER_MODEL,
        &mut warnings,
    );
    let default_context_limit = resolve_context_limit(
        values.get("ai_chat_context_limit").map(String::as_str),
        &mut warnings,
    );
    let default_since = resolve_config_date(
        "ai_chat_context_since",
        values.get("ai_chat_context_since").map(String::as_str),
        &mut warnings,
    );
    let default_until = resolve_config_date(
        "ai_chat_context_until",
        values.get("ai_chat_context_until").map(String::as_str),
        &mut warnings,
    );

    AiSettings {
        cloud_provider,
        openai_model,
        gemini_model,
        openrouter_model,
        default_context_limit,
        default_since,
        default_until,
        warnings,
    }
}

fn resolve_provider(value: Option<&str>) -> Option<String> {
    let normalized = normalize_string(value)?.to_lowercase();
    provider_definition(&normalized).map(|provider| provider.provider.to_string())
}

fn provider_definition(provider: &str) -> Option<ProviderDefinition> {
    PROVIDERS
        .iter()
        .copied()
        .find(|definition| definition.provider == provider)
}

fn validate_provider(provider: &str) -> Result<ProviderDefinition> {
    let normalized = normalize_string(Some(provider))
        .ok_or_else(|| anyhow!("Cloud provider is required."))?
        .to_lowercase();
    provider_definition(&normalized)
        .ok_or_else(|| anyhow!("Cloud provider must be one of: gemini, openai, openrouter."))
}

fn resolve_model(
    key: &str,
    value: Option<&str>,
    models: &[&str],
    default_model: &str,
    warnings: &mut Vec<String>,
) -> String {
    let Some(value) = normalize_string(value) else {
        return default_model.to_string();
    };
    let replacement = legacy_model_replacement(&value).unwrap_or(value.as_str());
    if replacement != value {
        warnings.push(format!("{key} was updated from {value} to {replacement}."));
    }
    if models.contains(&replacement) {
        replacement.to_string()
    } else {
        warnings.push(format!(
            "{key} value {replacement} is not supported; using {default_model}."
        ));
        default_model.to_string()
    }
}

fn validate_model(label: &str, value: &str, models: &[&str]) -> Result<String> {
    let value = normalize_string(Some(value)).ok_or_else(|| anyhow!("{label} is required."))?;
    let replacement = legacy_model_replacement(&value).unwrap_or(value.as_str());
    if models.contains(&replacement) {
        Ok(replacement.to_string())
    } else {
        Err(anyhow!("{label} must be one of: {}.", models.join(", ")))
    }
}

fn legacy_model_replacement(value: &str) -> Option<&'static str> {
    match value.trim() {
        "gemini-3-flash-preview" => Some("gemini-3.5-flash"),
        "z-ai/glm-5.1" => Some("z-ai/glm-5.2"),
        "qwen/qwen3.5-397b-a17b" => Some("qwen/qwen3.7-plus"),
        _ => None,
    }
}

fn resolve_context_limit(value: Option<&str>, warnings: &mut Vec<String>) -> Option<i64> {
    let value = normalize_string(value)?;
    let normalized = value.to_lowercase();
    if ["all", "none", "unlimited", "max"].contains(&normalized.as_str()) {
        return None;
    }
    match normalized.parse::<i64>() {
        Ok(limit) if limit > 0 => Some(limit),
        _ => {
            warnings.push("ai_chat_context_limit must be a positive integer or all.".to_string());
            None
        }
    }
}

fn resolve_config_date(
    key: &str,
    value: Option<&str>,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let value = normalize_string(value)?;
    match NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
        Ok(_) => Some(value),
        Err(_) => {
            warnings.push(format!("{key} must use YYYY-MM-DD format."));
            None
        }
    }
}

fn normalize_date(value: Option<&str>, label: &str) -> Result<Option<String>> {
    let Some(value) = normalize_string(value) else {
        return Ok(None);
    };
    NaiveDate::parse_from_str(&value, "%Y-%m-%d")
        .with_context(|| format!("{label} must use YYYY-MM-DD format."))?;
    Ok(Some(value))
}

fn selected_model_for_provider(settings: &AiSettings, provider: &str) -> String {
    match provider {
        "openai" => settings.openai_model.clone(),
        "openrouter" => settings.openrouter_model.clone(),
        _ => settings.gemini_model.clone(),
    }
}

fn lookup_api_key_presence(db_path: &Path, provider: ProviderDefinition) -> ApiKeyPresence {
    if credential_store_has_key(provider.env_key).unwrap_or(false) {
        return ApiKeyPresence {
            configured: true,
            source: Some("OS credential store".to_string()),
        };
    }

    if process_env_has_key(provider.env_key) {
        return ApiKeyPresence {
            configured: true,
            source: Some("Environment".to_string()),
        };
    }

    if dotenv_has_key(db_path, provider.env_key) {
        return ApiKeyPresence {
            configured: true,
            source: Some("Local .env".to_string()),
        };
    }

    ApiKeyPresence {
        configured: false,
        source: None,
    }
}

fn credential_store_has_key(env_key: &str) -> Result<bool> {
    let entry = keyring_entry(env_key)?;
    match entry.get_password() {
        Ok(value) => Ok(!value.trim().is_empty()),
        Err(KeyringError::NoEntry) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn keyring_entry(env_key: &str) -> Result<Entry> {
    ensure_credential_store()?;
    Entry::new(KEYRING_SERVICE, env_key)
        .with_context(|| format!("failed to open OS credential store for {env_key}"))
}

fn ensure_credential_store() -> Result<()> {
    if keyring_core::get_default_store().is_some() {
        return Ok(());
    }

    #[cfg(windows)]
    {
        keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);
        Ok(())
    }

    #[cfg(not(windows))]
    {
        Err(anyhow!(
            "OS credential store is not configured for this platform."
        ))
    }
}

fn process_env_has_key(env_key: &str) -> bool {
    env::var(env_key)
        .ok()
        .and_then(|value| normalize_string(Some(&value)))
        .is_some()
}

fn dotenv_has_key(db_path: &Path, env_key: &str) -> bool {
    dotenv_candidates(db_path)
        .into_iter()
        .any(|path| dotenv_file_has_key(&path, env_key))
}

fn dotenv_candidates(db_path: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = env::var("CAPSULE_ENV_PATH")
        .ok()
        .and_then(|value| normalize_string(Some(&value)))
    {
        candidates.push(PathBuf::from(path));
    }
    if let Some(directory) = settings::config_path_for_database(db_path).parent() {
        candidates.push(directory.join(".env"));
    }
    if let Ok(directory) = env::current_dir() {
        candidates.push(directory.join(".env"));
    }

    let mut unique = Vec::new();
    for path in candidates {
        if !unique.iter().any(|existing: &PathBuf| existing == &path) {
            unique.push(path);
        }
    }
    unique
}

fn dotenv_file_has_key(path: &Path, env_key: &str) -> bool {
    if !path.exists()
        || !fs::metadata(path)
            .map(|item| item.is_file())
            .unwrap_or(false)
    {
        return false;
    }
    let Ok(iter) = dotenvy::from_path_iter(path) else {
        return false;
    };
    iter.filter_map(|item| item.ok())
        .any(|(key, value)| key == env_key && !value.trim().is_empty())
}

fn set_optional_config_string(
    object: &mut Map<String, JsonValue>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        object.insert(key.to_string(), JsonValue::String(value));
    } else {
        object.remove(key);
    }
}

fn normalize_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn ai_settings_replace_legacy_model_ids() {
        let values = vec![
            crate::models::CapsuleConfigValue {
                key: "cloud_provider".to_string(),
                value: "openrouter".to_string(),
            },
            crate::models::CapsuleConfigValue {
                key: "gemini_model".to_string(),
                value: "gemini-3-flash-preview".to_string(),
            },
            crate::models::CapsuleConfigValue {
                key: "openrouter_model".to_string(),
                value: "qwen/qwen3.5-397b-a17b".to_string(),
            },
        ];

        let settings = ai_settings_from_config(&values);

        assert_eq!(settings.cloud_provider, "openrouter");
        assert_eq!(settings.gemini_model, "gemini-3.5-flash");
        assert_eq!(settings.openrouter_model, "qwen/qwen3.7-plus");
        assert!(settings
            .warnings
            .iter()
            .any(|warning| warning.contains("gemini_model was updated")));
    }

    #[test]
    fn update_ai_settings_writes_one_config_backup() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");
        fs::write(temp_dir.path().join("config.json"), r#"{"theme":"dark"}"#).expect("config");

        let response = update_ai_settings_for_database(
            &db_path,
            AiSettingsUpdateRequest {
                cloud_provider: "openrouter".to_string(),
                gemini_model: "gemini-3.5-flash".to_string(),
                openai_model: "gpt-5.4-mini".to_string(),
                openrouter_model: "z-ai/glm-5.2".to_string(),
                default_context_limit: Some(25),
                default_since: Some("2026-01-01".to_string()),
                default_until: None,
            },
        )
        .expect("update ai settings");

        assert_eq!(response.operation, "config.ai.set");
        assert!(response
            .backup_path
            .as_ref()
            .map(|path| PathBuf::from(path).exists())
            .unwrap_or(false));
        let raw = fs::read_to_string(temp_dir.path().join("config.json")).expect("read config");
        let value: JsonValue = serde_json::from_str(&raw).expect("json");
        assert_eq!(value["cloud_provider"], "openrouter");
        assert_eq!(value["openrouter_model"], "z-ai/glm-5.2");
        assert_eq!(value["ai_chat_context_limit"], "25");
        assert_eq!(value["ai_chat_context_since"], "2026-01-01");
        assert!(value.get("ai_chat_context_until").is_none());
    }

    #[test]
    fn update_ai_settings_validates_model_limit_and_dates() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("capsule.db");

        let invalid_model = update_ai_settings_for_database(
            &db_path,
            AiSettingsUpdateRequest {
                cloud_provider: "gemini".to_string(),
                gemini_model: "gemini-not-real".to_string(),
                openai_model: "gpt-5.4-mini".to_string(),
                openrouter_model: DEFAULT_OPENROUTER_MODEL.to_string(),
                default_context_limit: None,
                default_since: None,
                default_until: None,
            },
        );
        assert!(invalid_model.is_err());

        let invalid_dates = update_ai_settings_for_database(
            &db_path,
            AiSettingsUpdateRequest {
                cloud_provider: "gemini".to_string(),
                gemini_model: DEFAULT_GEMINI_MODEL.to_string(),
                openai_model: DEFAULT_OPENAI_MODEL.to_string(),
                openrouter_model: DEFAULT_OPENROUTER_MODEL.to_string(),
                default_context_limit: Some(0),
                default_since: Some("2026-02-01".to_string()),
                default_until: Some("2026-01-01".to_string()),
            },
        );
        assert!(invalid_dates.is_err());
    }

    #[test]
    fn provider_status_is_redacted() {
        let status = build_provider_status(
            PROVIDERS[1],
            DEFAULT_OPENAI_MODEL.to_string(),
            ApiKeyPresence {
                configured: true,
                source: Some("Environment".to_string()),
            },
        );

        assert!(status.configured);
        assert_eq!(status.key_source.as_deref(), Some("Environment"));
        assert_eq!(status.selected_model, DEFAULT_OPENAI_MODEL);
        assert!(status.missing_reason.is_none());
    }

    #[test]
    fn dotenv_detection_reports_presence_without_returning_value() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let env_path = temp_dir.path().join(".env");
        fs::write(&env_path, "OPENROUTER_API_KEY=secret-value\n").expect("env");

        assert!(dotenv_file_has_key(&env_path, "OPENROUTER_API_KEY"));
        assert!(!dotenv_file_has_key(&env_path, "OPENAI_API_KEY"));
    }
}
