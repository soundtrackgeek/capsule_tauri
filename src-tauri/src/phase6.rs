use std::{
    collections::{HashMap, HashSet},
    env,
    path::Path,
};

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;

use crate::{
    backup, db, entries,
    models::{
        AiConversationSummary, AiMetadataSuggestionRequest, AiMetadataSuggestionResponse,
        AiOverviewResponse, AiTimeCapsuleSummary, EmbeddingModelSummary, GamificationBadge,
        GamificationOverviewResponse, GamificationProfileSummary, GamificationQuest,
        GamificationXpEvent, Phase6Capability, PluginInfo, PluginOverviewResponse,
        QuestClaimRequest, QuestClaimResponse, SyncHistoryItem, SyncOverviewResponse,
        SyncStatusSummary, SyncTombstoneCount,
    },
    settings,
};

#[cfg(test)]
use crate::models::{PluginMutationRequest, PluginMutationResponse};

const RECENT_LIMIT: i64 = 10;

#[derive(Debug, Clone)]
struct KnownPlugin {
    key: &'static str,
    label: &'static str,
    table_name: Option<&'static str>,
    implemented: bool,
}

pub fn get_ai_overview() -> Result<AiOverviewResponse> {
    get_ai_overview_for_database(&db::resolve_database_path())
}

pub(crate) fn get_ai_overview_for_database(db_path: &Path) -> Result<AiOverviewResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    let (provider, model) = ai_provider_and_model(db_path);
    let bridge_configured = python_bridge_configured();

    let conversation_count = count_if_table(&connection, &tables, "ai_conversations")?;
    let message_count = count_if_table(&connection, &tables, "ai_conversation_messages")?;
    let time_capsule_count = count_if_table(&connection, &tables, "ai_time_capsules")?;
    let embedded_entry_count = if tables.contains("embeddings") {
        connection.query_row(
            "SELECT COUNT(DISTINCT entry_id) FROM embeddings",
            [],
            |row| row.get::<_, i64>(0),
        )?
    } else {
        0
    };
    let embedding_models = list_embedding_models(&connection, &tables)?;
    let active_embedding_model = embedding_models.iter().any(|item| item.is_active);

    let mut warnings = Vec::new();
    if !bridge_configured {
        warnings.push(
            "Python bridge commands are not configured; AI chat, provider calls, and semantic search are read-only from Tauri."
                .to_string(),
        );
    }
    if provider.is_none() {
        warnings.push("No AI provider setting was found in Capsule config.".to_string());
    }

    Ok(AiOverviewResponse {
        provider: provider.clone(),
        model: model.clone(),
        capabilities: vec![
            capability(
                "metadata-suggestions",
                "Metadata suggestions",
                tables.contains("entries"),
                true,
                false,
                true,
                "Local title, summary, mood, and tag suggestions that do not send journal text to a cloud provider.",
            ),
            capability(
                "ai-search",
                "AI search",
                embedded_entry_count > 0,
                active_embedding_model && bridge_configured,
                true,
                true,
                if embedded_entry_count > 0 {
                    "Embedding rows are present; vector ranking remains delegated to the replaceable Python bridge."
                } else {
                    "No embedding rows were detected yet."
                },
            ),
            capability(
                "ai-chat-bridge",
                "AI chat bridge",
                tables.contains("ai_conversations") && tables.contains("ai_conversation_messages"),
                bridge_configured,
                true,
                true,
                "Persisted AI chats can be inspected; sending messages requires an explicitly configured provider bridge.",
            ),
            capability(
                "time-capsules",
                "AI Time Capsules",
                tables.contains("ai_time_capsules"),
                time_capsule_count > 0,
                true,
                true,
                "Reads existing AI Time Capsules without triggering new generation.",
            ),
        ],
        conversations: list_ai_conversations(&connection, &tables)?,
        time_capsules: list_time_capsules(&connection, &tables)?,
        embedding_models,
        conversation_count,
        message_count,
        time_capsule_count,
        embedded_entry_count,
        warnings,
    })
}

pub fn suggest_ai_metadata(
    input: AiMetadataSuggestionRequest,
) -> Result<AiMetadataSuggestionResponse> {
    suggest_ai_metadata_for_database(&db::resolve_database_path(), input)
}

pub(crate) fn suggest_ai_metadata_for_database(
    db_path: &Path,
    input: AiMetadataSuggestionRequest,
) -> Result<AiMetadataSuggestionResponse> {
    let entry = entries::get_entry_for_database(db_path, &input.identifier)?;
    let text = if entry.text_plain.trim().is_empty() {
        entry.text.clone()
    } else {
        entry.text_plain.clone()
    };
    let words = words(&text);
    let suggested_title = entry
        .title
        .clone()
        .or_else(|| title_from_text(&text).filter(|title| !title.is_empty()));
    let suggested_summary = entry
        .summary
        .clone()
        .or_else(|| summary_from_text(&words).filter(|summary| !summary.is_empty()));
    let suggested_mood = entry.mood.clone().or_else(|| mood_from_text(&words));
    let existing_tags = entry
        .tags
        .iter()
        .map(|tag| tag.name.to_lowercase())
        .collect::<HashSet<_>>();
    let suggested_tags = keywords_from_words(&words)
        .into_iter()
        .filter(|tag| !existing_tags.contains(tag))
        .take(5)
        .collect::<Vec<_>>();

    Ok(AiMetadataSuggestionResponse {
        entry_uuid: entry.uuid,
        source: "local-read-model".to_string(),
        suggested_title,
        suggested_summary,
        suggested_mood,
        suggested_tags,
        confidence: if words.len() >= 40 { 0.64 } else { 0.42 },
        warnings: vec![
            "No cloud request was made; these suggestions are local heuristics until an AI provider bridge is enabled."
                .to_string(),
        ],
    })
}

pub fn get_sync_overview() -> Result<SyncOverviewResponse> {
    get_sync_overview_for_database(&db::resolve_database_path())
}

pub(crate) fn get_sync_overview_for_database(db_path: &Path) -> Result<SyncOverviewResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    let local_settings = db::read_local_path_settings();
    let configured_sync_path = env::var("CAPSULE_SYNC_PATH")
        .ok()
        .and_then(|value| normalized_string(Some(&value)))
        .or_else(|| local_settings.sync_path.clone());
    let github_gist_id = env::var("CAPSULE_GITHUB_GIST_ID")
        .ok()
        .and_then(|value| normalized_string(Some(&value)))
        .or_else(|| local_settings.github_gist_id.clone());
    let github_gist_token_configured = env::var("CAPSULE_GITHUB_GIST_TOKEN")
        .ok()
        .and_then(|value| normalized_string(Some(&value)))
        .or_else(|| local_settings.github_gist_token.clone())
        .is_some();
    let effective_sync_path = configured_sync_path.clone().or_else(|| {
        github_gist_id
            .as_ref()
            .map(|_| db::path_to_string(&db::local_github_gist_sync_cache_path()))
    });
    let configured = effective_sync_path.is_some();
    let sync_file_path = effective_sync_path
        .as_ref()
        .map(|path| db::path_to_string(&Path::new(path).join("capsule_sync.json")));
    let auto_sync_enabled = configured && local_settings.auto_sync_enabled.unwrap_or(false);
    let auto_sync_interval_minutes = local_settings
        .auto_sync_interval_minutes
        .unwrap_or(15)
        .clamp(1, 24 * 60);
    let status = if tables.contains("sync_status") {
        connection
            .query_row(
                "SELECT last_successful_sync_at, last_sync_file_path, last_sync_file_size_bytes,
                        last_sync_imported, last_sync_updated, last_sync_deleted,
                        last_sync_total, last_sync_summary, last_conflict_count,
                        last_conflict_summary, last_sync_error
                 FROM sync_status
                 WHERE id = 1",
                [],
                sync_status_from_row,
            )
            .optional()?
    } else {
        None
    };
    let recent_history = if tables.contains("sync_history") {
        let mut statement = connection.prepare(
            "SELECT id, timestamp, status, sync_file_path, imported_count, updated_count,
                    deleted_count, exported_count, conflict_count, summary, error
             FROM sync_history
             ORDER BY timestamp DESC, id DESC
             LIMIT ?1",
        )?;
        let rows = statement
            .query_map([RECENT_LIMIT], sync_history_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        Vec::new()
    };
    let tombstones = sync_tombstone_tables()
        .into_iter()
        .filter(|table| tables.contains(*table))
        .map(|table| {
            Ok(SyncTombstoneCount {
                table: table.to_string(),
                count: count_table_rows(&connection, table)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut warnings = Vec::new();
    if !configured {
        warnings.push("No shared-folder sync path is configured in Settings.".to_string());
    }

    Ok(SyncOverviewResponse {
        configured,
        sync_path: effective_sync_path,
        sync_file_path,
        github_gist_id: github_gist_id.clone(),
        github_gist_token_configured,
        auto_sync_enabled,
        auto_sync_interval_minutes,
        status,
        recent_history,
        tombstones,
        capabilities: vec![
            capability(
                "shared-folder-sync",
                "Shared-folder sync",
                true,
                configured_sync_path.is_some(),
                false,
                false,
                "Runs Capsule-compatible shared-folder sync directly from Tauri.",
            ),
            capability(
                "github-gist-import",
                "GitHub Gist import",
                true,
                github_gist_id.is_some(),
                true,
                !github_gist_token_configured,
                if github_gist_token_configured {
                    "Pulls Capsule sync files before merge and pushes merged files back to GitHub Gist."
                } else {
                    "Pulls Capsule sync files before merge; add a Gist token to push merged files back."
                },
            ),
            capability(
                "sync-history",
                "Sync history",
                tables.contains("sync_history"),
                true,
                false,
                true,
                "Reads previous sync/import outcomes and conflict counts.",
            ),
        ],
        warnings,
    })
}

pub fn get_plugin_overview() -> Result<PluginOverviewResponse> {
    get_plugin_overview_for_database(&db::resolve_database_path())
}

pub(crate) fn get_plugin_overview_for_database(db_path: &Path) -> Result<PluginOverviewResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    let plugins = list_plugins(&connection, &tables)?;

    Ok(PluginOverviewResponse {
        capabilities: vec![
            capability(
                "plugin-state",
                "Plugin state",
                tables.contains("plugin_state"),
                tables.contains("plugin_state"),
                false,
                true,
                "Reads legacy Capsule plugin activation state for compatibility; enable/disable writes are not exposed.",
            ),
            capability(
                "plugin-screens",
                "Plugin screens",
                false,
                false,
                false,
                true,
                "Plugin registry screens are hidden while legacy plugin-prefixed tables remain readable.",
            ),
        ],
        plugins,
        warnings: vec![
            "Plugin management UI is disabled; media and location compatibility tables remain supported."
                .to_string(),
        ],
    })
}

pub fn get_gamification_overview() -> Result<GamificationOverviewResponse> {
    get_gamification_overview_for_database(&db::resolve_database_path())
}

pub(crate) fn get_gamification_overview_for_database(
    db_path: &Path,
) -> Result<GamificationOverviewResponse> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    let total_xp = if tables.contains("gamification_xp_events") {
        connection.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM gamification_xp_events",
            [],
            |row| row.get::<_, i64>(0),
        )?
    } else {
        0
    };
    let event_count = count_if_table(&connection, &tables, "gamification_xp_events")?;
    let (level, xp_to_next_level) = level_from_xp(total_xp);

    Ok(GamificationOverviewResponse {
        profile: load_gamification_profile(&connection, &tables)?,
        total_xp,
        level,
        xp_to_next_level,
        event_count,
        recent_events: load_xp_events(&connection, &tables)?,
        quests: load_quests(&connection, &tables, 12)?,
        badges: load_badges(&connection, &tables)?,
        capabilities: vec![
            capability(
                "xp-events",
                "XP events",
                tables.contains("gamification_xp_events"),
                true,
                false,
                true,
                "Reads existing XP events and derives a desktop profile level.",
            ),
            capability(
                "quest-claim",
                "Quest claiming",
                tables.contains("gamification_quest_state")
                    && tables.contains("gamification_xp_events"),
                true,
                false,
                false,
                "Claims completed quests with a backup and a compatible XP event.",
            ),
            capability(
                "profile",
                "Profile",
                tables.contains("gamification_profile"),
                true,
                false,
                true,
                "Displays the selected hero profile if the table exists.",
            ),
        ],
        warnings: Vec::new(),
    })
}

pub fn claim_quest(input: QuestClaimRequest) -> Result<QuestClaimResponse> {
    let instance_id = normalize_key(&input.instance_id, "Quest instance")?;
    let guarded = backup::with_database_backup("gamification.quest.claim", move |db_path| {
        claim_quest_inner(db_path, &instance_id)
    })?;

    Ok(QuestClaimResponse {
        quest: guarded.value.0,
        total_xp: guarded.value.1,
        level: guarded.value.2,
        xp_to_next_level: guarded.value.3,
        audit: guarded.audit,
    })
}

#[cfg(test)]
fn set_plugin_enabled_for_database(
    db_path: &Path,
    input: PluginMutationRequest,
) -> Result<PluginMutationResponse> {
    let operation = if input.enabled {
        "plugin.enable"
    } else {
        "plugin.disable"
    };
    let plugin_name = normalize_key(&input.plugin_name, "Plugin name")?;
    let enabled = input.enabled;
    let guarded = backup::with_database_backup_for_database(db_path, operation, move |path| {
        set_plugin_enabled_inner(path, &plugin_name, enabled)?;
        let connection = db::open_read_only_connection(path)?;
        let tables = detected_tables(&connection)?;
        let plugins = list_plugins(&connection, &tables)?;
        let plugin = plugins
            .iter()
            .find(|plugin| plugin.key == plugin_name)
            .cloned()
            .ok_or_else(|| anyhow!("Plugin '{plugin_name}' was not found after update."))?;
        Ok((plugin, plugins))
    })?;

    Ok(PluginMutationResponse {
        plugin: guarded.value.0,
        plugins: guarded.value.1,
        audit: guarded.audit,
    })
}

#[cfg(test)]
fn claim_quest_for_database(db_path: &Path, instance_id: &str) -> Result<QuestClaimResponse> {
    let instance_id = normalize_key(instance_id, "Quest instance")?;
    let guarded =
        backup::with_database_backup_for_database(db_path, "gamification.quest.claim", |path| {
            claim_quest_inner(path, &instance_id)
        })?;
    Ok(QuestClaimResponse {
        quest: guarded.value.0,
        total_xp: guarded.value.1,
        level: guarded.value.2,
        xp_to_next_level: guarded.value.3,
        audit: guarded.audit,
    })
}

fn list_ai_conversations(
    connection: &Connection,
    tables: &HashSet<String>,
) -> Result<Vec<AiConversationSummary>> {
    if !tables.contains("ai_conversations") {
        return Ok(Vec::new());
    }

    let message_count_sql = if tables.contains("ai_conversation_messages") {
        "(SELECT COUNT(*) FROM ai_conversation_messages m WHERE m.conversation_id = c.id)"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT c.id, c.uuid, c.title, c.preview, c.cloud_provider, c.scope,
                {message_count_sql} AS message_count, c.last_message_at, c.updated_at
         FROM ai_conversations c
         ORDER BY datetime(c.updated_at) DESC, c.id DESC
         LIMIT ?1"
    );
    let mut statement = connection.prepare(&sql)?;
    let conversations = statement
        .query_map([RECENT_LIMIT], |row| {
            Ok(AiConversationSummary {
                id: row.get(0)?,
                uuid: row.get(1)?,
                title: row.get(2)?,
                preview: row.get(3)?,
                cloud_provider: row.get(4)?,
                scope: row.get(5)?,
                message_count: row.get(6)?,
                last_message_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(conversations)
}

fn list_time_capsules(
    connection: &Connection,
    tables: &HashSet<String>,
) -> Result<Vec<AiTimeCapsuleSummary>> {
    if !tables.contains("ai_time_capsules") {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        "SELECT id, trigger_label, due_date, status, source_entry_count,
                cloud_provider, llm_model, read_at, dismissed_at, error_message
         FROM ai_time_capsules
         ORDER BY datetime(due_date) DESC, id DESC
         LIMIT ?1",
    )?;
    let capsules = statement
        .query_map([RECENT_LIMIT], |row| {
            Ok(AiTimeCapsuleSummary {
                id: row.get(0)?,
                trigger_label: row.get(1)?,
                due_date: row.get(2)?,
                status: row.get(3)?,
                source_entry_count: row.get(4)?,
                cloud_provider: row.get(5)?,
                llm_model: row.get(6)?,
                read_at: row.get(7)?,
                dismissed_at: row.get(8)?,
                error_message: row.get(9)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(capsules)
}

fn list_embedding_models(
    connection: &Connection,
    tables: &HashSet<String>,
) -> Result<Vec<EmbeddingModelSummary>> {
    if !tables.contains("embedding_models") {
        return Ok(Vec::new());
    }
    let entry_count_sql = if tables.contains("embeddings") {
        "(SELECT COUNT(*) FROM embeddings e WHERE e.model_id = m.id)"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT m.id, m.name, m.dimensions, m.provider, COALESCE(m.is_active, 0),
                {entry_count_sql} AS entry_count
         FROM embedding_models m
         ORDER BY COALESCE(m.is_active, 0) DESC, lower(m.name) ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let models = statement
        .query_map([], |row| {
            Ok(EmbeddingModelSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                dimensions: row.get(2)?,
                provider: row.get(3)?,
                is_active: row.get::<_, i64>(4)? != 0,
                entry_count: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(models)
}

fn sync_status_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncStatusSummary> {
    Ok(SyncStatusSummary {
        last_successful_sync_at: row.get(0)?,
        last_sync_file_path: row.get(1)?,
        last_sync_file_size_bytes: row.get(2)?,
        last_sync_imported: row.get(3)?,
        last_sync_updated: row.get(4)?,
        last_sync_deleted: row.get(5)?,
        last_sync_total: row.get(6)?,
        last_sync_summary: row.get(7)?,
        last_conflict_count: row.get(8)?,
        last_conflict_summary: row.get(9)?,
        last_sync_error: row.get(10)?,
    })
}

fn sync_history_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncHistoryItem> {
    Ok(SyncHistoryItem {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        status: row.get(2)?,
        sync_file_path: row.get(3)?,
        imported_count: row.get(4)?,
        updated_count: row.get(5)?,
        deleted_count: row.get(6)?,
        exported_count: row.get(7)?,
        conflict_count: row.get(8)?,
        summary: row.get(9)?,
        error: row.get(10)?,
    })
}

fn list_plugins(connection: &Connection, tables: &HashSet<String>) -> Result<Vec<PluginInfo>> {
    let state = load_plugin_state(connection, tables)?;
    let mut plugins = Vec::new();
    let known = known_plugins();
    let known_keys = known
        .iter()
        .map(|plugin| plugin.key)
        .collect::<HashSet<_>>();

    for plugin in known {
        let state = state.get(plugin.key);
        let row_count = plugin
            .table_name
            .filter(|table| tables.contains(*table))
            .map(|table| count_table_rows(connection, table))
            .transpose()?
            .unwrap_or(0);
        plugins.push(PluginInfo {
            key: plugin.key.to_string(),
            label: plugin.label.to_string(),
            enabled: state.map(|item| item.enabled).unwrap_or(false),
            installed_version: state.and_then(|item| item.installed_version.clone()),
            source: state
                .map(|item| item.source.clone())
                .unwrap_or_else(|| "not-installed".to_string()),
            updated_at: state.and_then(|item| item.updated_at.clone()),
            implemented: plugin.implemented,
            table_name: plugin.table_name.map(str::to_string),
            row_count,
        });
    }

    for (key, state) in state {
        if known_keys.contains(key.as_str()) {
            continue;
        }
        plugins.push(PluginInfo {
            key: key.clone(),
            label: labelize(&key),
            enabled: state.enabled,
            installed_version: state.installed_version,
            source: state.source,
            updated_at: state.updated_at,
            implemented: false,
            table_name: None,
            row_count: 0,
        });
    }

    plugins.sort_by_key(|plugin| plugin.label.to_lowercase());
    Ok(plugins)
}

#[derive(Debug, Clone)]
struct PluginStateRow {
    enabled: bool,
    installed_version: Option<String>,
    source: String,
    updated_at: Option<String>,
}

fn load_plugin_state(
    connection: &Connection,
    tables: &HashSet<String>,
) -> Result<HashMap<String, PluginStateRow>> {
    if !tables.contains("plugin_state") {
        return Ok(HashMap::new());
    }

    let mut statement = connection.prepare(
        "SELECT plugin_name, enabled, installed_version, source, updated_at
         FROM plugin_state",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            PluginStateRow {
                enabled: row.get::<_, i64>(1)? != 0,
                installed_version: row.get(2)?,
                source: row.get(3)?,
                updated_at: row.get(4)?,
            },
        ))
    })?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .collect())
}

#[cfg(test)]
fn set_plugin_enabled_inner(db_path: &Path, plugin_name: &str, enabled: bool) -> Result<()> {
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_plugin_schema(&tx)?;
    tx.execute(
        "INSERT INTO plugin_state (plugin_name, enabled, installed_version, source, updated_at)
         VALUES (?1, ?2, NULL, 'tauri', ?3)
         ON CONFLICT(plugin_name)
         DO UPDATE SET enabled = excluded.enabled,
                       updated_at = excluded.updated_at",
        params![
            plugin_name,
            bool_to_int(enabled),
            current_timestamp_seconds()
        ],
    )?;
    tx.commit()?;
    Ok(())
}

fn load_gamification_profile(
    connection: &Connection,
    tables: &HashSet<String>,
) -> Result<Option<GamificationProfileSummary>> {
    if !tables.contains("gamification_profile") {
        return Ok(None);
    }

    connection
        .query_row(
            "SELECT hero_sprite_path, updated_at FROM gamification_profile WHERE id = 1",
            [],
            |row| {
                Ok(GamificationProfileSummary {
                    hero_sprite_path: row.get(0)?,
                    updated_at: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_xp_events(
    connection: &Connection,
    tables: &HashSet<String>,
) -> Result<Vec<GamificationXpEvent>> {
    if !tables.contains("gamification_xp_events") {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        "SELECT id, source_type, source_key, amount, reason, created_at
         FROM gamification_xp_events
         ORDER BY datetime(created_at) DESC, id DESC
         LIMIT ?1",
    )?;
    let events = statement
        .query_map([RECENT_LIMIT], |row| {
            Ok(GamificationXpEvent {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_key: row.get(2)?,
                amount: row.get(3)?,
                reason: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(events)
}

fn load_quests(
    connection: &Connection,
    tables: &HashSet<String>,
    limit: i64,
) -> Result<Vec<GamificationQuest>> {
    if !tables.contains("gamification_quest_state") {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        "SELECT instance_id, quest_key, kind, title, description, enemy_sprite_path,
                target_value, progress_value, reward_xp, status, period_key, starts_at,
                expires_at, completed_at, claimed_at, updated_at
         FROM gamification_quest_state
         ORDER BY CASE WHEN claimed_at IS NULL THEN 0 ELSE 1 END ASC,
                  datetime(updated_at) DESC,
                  instance_id ASC
         LIMIT ?1",
    )?;
    let quests = statement
        .query_map([limit], quest_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(quests)
}

fn load_badges(
    connection: &Connection,
    tables: &HashSet<String>,
) -> Result<Vec<GamificationBadge>> {
    if !tables.contains("gamification_badge_unlocks") {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        "SELECT badge_key, unlocked_at, updated_at
         FROM gamification_badge_unlocks
         ORDER BY datetime(unlocked_at) DESC, badge_key ASC
         LIMIT ?1",
    )?;
    let badges = statement
        .query_map([RECENT_LIMIT], |row| {
            Ok(GamificationBadge {
                badge_key: row.get(0)?,
                unlocked_at: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(badges)
}

fn quest_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GamificationQuest> {
    Ok(GamificationQuest {
        instance_id: row.get(0)?,
        quest_key: row.get(1)?,
        kind: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        enemy_sprite_path: row.get(5)?,
        target_value: row.get(6)?,
        progress_value: row.get(7)?,
        reward_xp: row.get(8)?,
        status: row.get(9)?,
        period_key: row.get(10)?,
        starts_at: row.get(11)?,
        expires_at: row.get(12)?,
        completed_at: row.get(13)?,
        claimed_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn claim_quest_inner(
    db_path: &Path,
    instance_id: &str,
) -> Result<(GamificationQuest, i64, i64, i64)> {
    let mut connection = db::open_read_write_connection(db_path)?;
    let tx = connection.transaction()?;
    ensure_gamification_schema(&tx)?;
    let quest = tx
        .query_row(
            "SELECT instance_id, quest_key, kind, title, description, enemy_sprite_path,
                    target_value, progress_value, reward_xp, status, period_key, starts_at,
                    expires_at, completed_at, claimed_at, updated_at
             FROM gamification_quest_state
             WHERE instance_id = ?1",
            [instance_id],
            quest_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow!("Quest '{instance_id}' was not found."))?;

    if quest.claimed_at.is_some() || quest.status.eq_ignore_ascii_case("claimed") {
        return Err(anyhow!("Quest '{}' has already been claimed.", quest.title));
    }
    if quest.progress_value < quest.target_value {
        return Err(anyhow!("Quest '{}' is not complete yet.", quest.title));
    }

    let now = current_timestamp_seconds();
    tx.execute(
        "UPDATE gamification_quest_state
         SET status = 'claimed',
             completed_at = COALESCE(completed_at, ?2),
             claimed_at = ?2,
             updated_at = ?2
         WHERE instance_id = ?1",
        params![instance_id, now],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO gamification_xp_events
            (source_type, source_key, amount, reason, metadata_json, created_at)
         VALUES ('quest', ?1, ?2, ?3, ?4, ?5)",
        params![
            instance_id,
            quest.reward_xp,
            format!("Quest claimed: {}", quest.title),
            json!({ "questKey": quest.quest_key, "kind": quest.kind }).to_string(),
            now,
        ],
    )?;
    let updated = tx.query_row(
        "SELECT instance_id, quest_key, kind, title, description, enemy_sprite_path,
                target_value, progress_value, reward_xp, status, period_key, starts_at,
                expires_at, completed_at, claimed_at, updated_at
         FROM gamification_quest_state
         WHERE instance_id = ?1",
        [instance_id],
        quest_from_row,
    )?;
    let total_xp = tx.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM gamification_xp_events",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    tx.commit()?;
    let (level, xp_to_next_level) = level_from_xp(total_xp);
    Ok((updated, total_xp, level, xp_to_next_level))
}

fn ai_provider_and_model(db_path: &Path) -> (Option<String>, Option<String>) {
    let config = settings::get_capsule_config_for_database(db_path).ok();
    let Some(config) = config else {
        return (None, None);
    };
    let values = config
        .values
        .into_iter()
        .map(|item| (item.key.to_lowercase(), item.value))
        .collect::<HashMap<_, _>>();
    let provider = [
        "ai_provider",
        "llm_provider",
        "cloud_provider",
        "default_ai_provider",
    ]
    .into_iter()
    .find_map(|key| normalized_string(values.get(key).map(String::as_str)));
    let model = ["ai_model", "llm_model", "gemini_model", "openai_model"]
        .into_iter()
        .find_map(|key| normalized_string(values.get(key).map(String::as_str)));
    (provider, model)
}

#[cfg(test)]
fn ensure_plugin_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS plugin_state (
            plugin_name TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
            installed_version TEXT,
            source TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

fn ensure_gamification_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS gamification_xp_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_type TEXT NOT NULL,
            source_key TEXT NOT NULL,
            amount INTEGER NOT NULL,
            reason TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            UNIQUE (source_type, source_key)
        );
        CREATE TABLE IF NOT EXISTS gamification_quest_state (
            instance_id TEXT PRIMARY KEY,
            quest_key TEXT NOT NULL,
            kind TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            enemy_sprite_path TEXT,
            target_value INTEGER NOT NULL,
            progress_value INTEGER NOT NULL DEFAULT 0,
            reward_xp INTEGER NOT NULL,
            status TEXT NOT NULL,
            period_key TEXT NOT NULL,
            starts_at TEXT NOT NULL,
            expires_at TEXT,
            completed_at TEXT,
            claimed_at TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_gamification_xp_events_created_at
            ON gamification_xp_events(created_at DESC, id DESC);
        CREATE INDEX IF NOT EXISTS idx_gamification_quest_kind_period
            ON gamification_quest_state(kind, period_key, status);
        ",
    )?;
    Ok(())
}

fn detected_tables(connection: &Connection) -> Result<HashSet<String>> {
    Ok(db::inspect_schema(connection)?
        .detected_tables
        .into_iter()
        .collect())
}

fn count_if_table(
    connection: &Connection,
    tables: &HashSet<String>,
    table_name: &str,
) -> Result<i64> {
    if tables.contains(table_name) {
        count_table_rows(connection, table_name)
    } else {
        Ok(0)
    }
}

fn count_table_rows(connection: &Connection, table_name: &str) -> Result<i64> {
    let safe_table = safe_table_name(table_name)?;
    let sql = format!("SELECT COUNT(*) FROM {safe_table}");
    connection
        .query_row(&sql, [], |row| row.get::<_, i64>(0))
        .with_context(|| format!("failed to count rows in {safe_table}"))
}

fn safe_table_name(table_name: &str) -> Result<&str> {
    let allowed = known_plugins()
        .into_iter()
        .filter_map(|plugin| plugin.table_name)
        .chain(sync_tombstone_tables())
        .chain([
            "ai_conversations",
            "ai_conversation_messages",
            "ai_time_capsules",
            "embedding_models",
            "embeddings",
            "gamification_xp_events",
            "gamification_quest_state",
            "gamification_badge_unlocks",
            "plugin_state",
            "sync_status",
            "sync_history",
        ])
        .collect::<HashSet<_>>();
    if allowed.contains(table_name) {
        Ok(table_name)
    } else {
        Err(anyhow!("unsupported table count: {table_name}"))
    }
}

fn known_plugins() -> Vec<KnownPlugin> {
    vec![
        KnownPlugin {
            key: "dream_log",
            label: "Dream Log",
            table_name: Some("plugin_dreams"),
            implemented: true,
        },
        KnownPlugin {
            key: "coding_ideas",
            label: "Coding Ideas",
            table_name: Some("plugin_coding_ideas"),
            implemented: true,
        },
        KnownPlugin {
            key: "writing_ideas",
            label: "Writing Ideas",
            table_name: Some("plugin_writing_ideas"),
            implemented: true,
        },
        KnownPlugin {
            key: "post_ideas",
            label: "Post Ideas",
            table_name: Some("plugin_post_ideas"),
            implemented: true,
        },
        KnownPlugin {
            key: "images",
            label: "Images",
            table_name: Some("plugin_media_assets"),
            implemented: true,
        },
        KnownPlugin {
            key: "location",
            label: "Location",
            table_name: Some("plugin_entry_locations"),
            implemented: true,
        },
    ]
}

fn sync_tombstone_tables() -> Vec<&'static str> {
    vec![
        "sync_tombstones",
        "sync_entry_continuation_tombstones",
        "sync_entry_thread_title_tombstones",
        "sync_entry_thread_summary_tombstones",
        "sync_image_tombstones",
        "sync_location_tombstones",
        "sync_template_tombstones",
        "sync_prompt_tombstones",
        "sync_ai_conversation_tombstones",
    ]
}

fn capability(
    key: &str,
    label: &str,
    available: bool,
    configured: bool,
    requires_cloud: bool,
    read_only: bool,
    detail: &str,
) -> Phase6Capability {
    Phase6Capability {
        key: key.to_string(),
        label: label.to_string(),
        available,
        configured,
        requires_cloud,
        read_only,
        detail: detail.to_string(),
    }
}

fn python_bridge_configured() -> bool {
    ["CAPSULE_PYTHON_BRIDGE_COMMAND", "CAPSULE_PYTHON_BRIDGE"]
        .into_iter()
        .any(|name| {
            env::var(name)
                .ok()
                .and_then(|value| normalized_string(Some(&value)))
                .is_some()
        })
}

fn title_from_text(text: &str) -> Option<String> {
    let sentence = text
        .split(['.', '!', '?', '\n'])
        .map(str::trim)
        .find(|value| !value.is_empty())?;
    let words = sentence.split_whitespace().take(9).collect::<Vec<_>>();
    (!words.is_empty()).then(|| words.join(" "))
}

fn summary_from_text(words: &[String]) -> Option<String> {
    if words.is_empty() {
        return None;
    }
    let limit = words.len().min(28);
    Some(words[..limit].join(" "))
}

fn mood_from_text(words: &[String]) -> Option<String> {
    let joined = words.join(" ");
    let candidates = [
        (
            "grateful",
            ["thankful", "grateful", "appreciate", "kind"].as_slice(),
        ),
        (
            "focused",
            ["focus", "work", "build", "debug", "ship"].as_slice(),
        ),
        (
            "excited",
            ["excited", "spark", "momentum", "delight"].as_slice(),
        ),
        ("tired", ["tired", "exhausted", "sleep", "heavy"].as_slice()),
        ("calm", ["calm", "quiet", "steady", "peace"].as_slice()),
    ];
    candidates
        .into_iter()
        .find(|(_, needles)| needles.iter().any(|needle| joined.contains(needle)))
        .map(|(mood, _)| mood.to_string())
}

fn keywords_from_words(words: &[String]) -> Vec<String> {
    let stopwords = stopwords();
    let mut counts = HashMap::<String, i64>::new();
    for word in words {
        if word.len() < 4 || stopwords.contains(word.as_str()) {
            continue;
        }
        *counts.entry(word.clone()).or_insert(0) += 1;
    }
    let mut values = counts.into_iter().collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values.into_iter().map(|(word, _)| word).collect()
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

fn level_from_xp(total_xp: i64) -> (i64, i64) {
    let level_size = 500_i64;
    let level = (total_xp / level_size) + 1;
    let next_threshold = level * level_size;
    (level, (next_threshold - total_xp).max(0))
}

fn normalize_key(value: &str, label: &str) -> Result<String> {
    let value = normalized_string(Some(value)).ok_or_else(|| anyhow!("{label} is required."))?;
    if value.len() > 128 {
        return Err(anyhow!("{label} must be 128 characters or fewer."));
    }
    Ok(value)
}

fn normalized_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn labelize(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
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

fn current_timestamp_seconds() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
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
        "entry",
        "journal",
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::{fs, path::PathBuf};

    #[test]
    fn ai_overview_counts_models_and_local_metadata_suggestions() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_phase6_fixture(temp_dir.path());

        let overview = get_ai_overview_for_database(&db_path).expect("overview");
        assert_eq!(overview.conversation_count, 1);
        assert_eq!(overview.message_count, 2);
        assert_eq!(overview.embedded_entry_count, 1);
        assert_eq!(overview.embedding_models[0].name, "text-embedding-test");

        let suggestion = suggest_ai_metadata_for_database(
            &db_path,
            AiMetadataSuggestionRequest {
                identifier: "entry_one".to_string(),
            },
        )
        .expect("suggestion");
        assert_eq!(suggestion.entry_uuid, "entry_one");
        assert_eq!(suggestion.source, "local-read-model");
        assert!(suggestion.suggested_title.is_some());
        assert!(!suggestion.warnings.is_empty());
    }

    #[test]
    fn plugin_toggle_upserts_state_with_backup() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_phase6_fixture(temp_dir.path());

        let response = set_plugin_enabled_for_database(
            &db_path,
            PluginMutationRequest {
                plugin_name: "dream_log".to_string(),
                enabled: true,
            },
        )
        .expect("toggle");

        assert!(response.plugin.enabled);
        assert_eq!(response.audit.operation, "plugin.enable");
        assert!(PathBuf::from(response.audit.backup_path).exists());
        let overview = get_plugin_overview_for_database(&db_path).expect("plugins");
        assert!(overview
            .plugins
            .iter()
            .any(|plugin| plugin.key == "dream_log" && plugin.enabled));
    }

    #[test]
    fn quest_claim_awards_xp_once_with_backup() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = create_phase6_fixture(temp_dir.path());

        let response = claim_quest_for_database(&db_path, "daily:focus:2026-06-29").expect("claim");

        assert_eq!(response.quest.status, "claimed");
        assert_eq!(response.total_xp, 140);
        assert_eq!(response.audit.operation, "gamification.quest.claim");
        assert!(PathBuf::from(response.audit.backup_path).exists());

        let second = claim_quest_for_database(&db_path, "daily:focus:2026-06-29");
        assert!(second.is_err());
    }

    fn create_phase6_fixture(path: &Path) -> PathBuf {
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
                    (uuid, created_at, updated_at, text, text_plain, content_format, title, mood)
                VALUES
                    ('entry_one', '2026-06-29 09:00', '2026-06-29 09:00',
                     'Focused work on the Capsule desktop AI bridge made the sync and plugin plan feel clear.',
                     'Focused work on the Capsule desktop AI bridge made the sync and plugin plan feel clear.',
                     'markdown', NULL, NULL);
                INSERT INTO tags (name) VALUES ('capsule');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1);

                CREATE TABLE ai_conversations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    uuid TEXT,
                    title TEXT NOT NULL,
                    preview TEXT NOT NULL DEFAULT '',
                    cloud_provider TEXT NOT NULL DEFAULT 'gemini',
                    scope TEXT NOT NULL DEFAULT 'search',
                    scope_identifiers TEXT NOT NULL DEFAULT '[]',
                    context_limit INTEGER,
                    since TEXT,
                    until TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    last_message_at TEXT NOT NULL
                );
                CREATE TABLE ai_conversation_messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    conversation_id INTEGER NOT NULL,
                    uuid TEXT,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL DEFAULT '',
                    status TEXT NOT NULL DEFAULT 'complete',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    sort_key TEXT
                );
                INSERT INTO ai_conversations
                    (uuid, title, preview, cloud_provider, scope, created_at, updated_at, last_message_at)
                VALUES ('chat_one', 'A chat', 'Preview', 'gemini', 'search',
                        '2026-06-29 09:00', '2026-06-29 09:05', '2026-06-29 09:05');
                INSERT INTO ai_conversation_messages
                    (conversation_id, uuid, role, content, created_at, updated_at)
                VALUES (1, 'msg_one', 'user', 'Hello', '2026-06-29 09:00', '2026-06-29 09:00'),
                       (1, 'msg_two', 'assistant', 'Hi', '2026-06-29 09:01', '2026-06-29 09:01');
                CREATE TABLE embedding_models (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    dimensions INTEGER NOT NULL,
                    provider TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    is_active INTEGER DEFAULT 0
                );
                CREATE TABLE embeddings (
                    entry_id INTEGER NOT NULL,
                    model_id INTEGER NOT NULL,
                    embedding BLOB NOT NULL,
                    created_at TEXT NOT NULL,
                    PRIMARY KEY (entry_id, model_id)
                );
                INSERT INTO embedding_models (name, dimensions, provider, created_at, is_active)
                VALUES ('text-embedding-test', 3, 'local', '2026-06-29 09:00', 1);
                INSERT INTO embeddings (entry_id, model_id, embedding, created_at)
                VALUES (1, 1, X'000102', '2026-06-29 09:01');

                CREATE TABLE sync_status (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    last_successful_sync_at TEXT,
                    last_sync_file_path TEXT,
                    last_sync_file_size_bytes INTEGER,
                    last_sync_imported INTEGER NOT NULL DEFAULT 0,
                    last_sync_updated INTEGER NOT NULL DEFAULT 0,
                    last_sync_deleted INTEGER NOT NULL DEFAULT 0,
                    last_sync_total INTEGER NOT NULL DEFAULT 0,
                    last_sync_summary TEXT,
                    last_conflict_count INTEGER NOT NULL DEFAULT 0,
                    last_conflict_summary TEXT,
                    last_sync_error TEXT
                );
                INSERT INTO sync_status
                    (id, last_successful_sync_at, last_sync_file_path, last_sync_file_size_bytes,
                     last_sync_imported, last_sync_updated, last_sync_deleted, last_sync_total,
                     last_sync_summary, last_conflict_count)
                VALUES (1, '2026-06-29 09:00', 'sync.json', 42, 1, 2, 0, 3, 'ok', 0);

                CREATE TABLE plugin_state (
                    plugin_name TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
                    installed_version TEXT,
                    source TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE plugin_dreams (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    entry_uuid TEXT NOT NULL
                );
                INSERT INTO plugin_state
                    (plugin_name, enabled, installed_version, source, updated_at)
                VALUES ('dream_log', 0, '1.0.0', 'catalog', '2026-06-29 09:00');
                INSERT INTO plugin_dreams (entry_uuid) VALUES ('entry_one');

                CREATE TABLE gamification_xp_events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    source_type TEXT NOT NULL,
                    source_key TEXT NOT NULL,
                    amount INTEGER NOT NULL,
                    reason TEXT NOT NULL,
                    metadata_json TEXT NOT NULL DEFAULT '{}',
                    created_at TEXT NOT NULL,
                    UNIQUE (source_type, source_key)
                );
                CREATE TABLE gamification_quest_state (
                    instance_id TEXT PRIMARY KEY,
                    quest_key TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    title TEXT NOT NULL,
                    description TEXT NOT NULL,
                    enemy_sprite_path TEXT,
                    target_value INTEGER NOT NULL,
                    progress_value INTEGER NOT NULL DEFAULT 0,
                    reward_xp INTEGER NOT NULL,
                    status TEXT NOT NULL,
                    period_key TEXT NOT NULL,
                    starts_at TEXT NOT NULL,
                    expires_at TEXT,
                    completed_at TEXT,
                    claimed_at TEXT,
                    updated_at TEXT NOT NULL
                );
                INSERT INTO gamification_xp_events
                    (source_type, source_key, amount, reason, created_at)
                VALUES ('entry', 'entry_one', 100, 'Entry created', '2026-06-29 09:00');
                INSERT INTO gamification_quest_state
                    (instance_id, quest_key, kind, title, description, target_value,
                     progress_value, reward_xp, status, period_key, starts_at, updated_at)
                VALUES ('daily:focus:2026-06-29', 'focus', 'daily', 'Focus',
                        'Write with focus', 1, 1, 40, 'complete', '2026-06-29',
                        '2026-06-29 00:00', '2026-06-29 09:00');
                ",
            )
            .expect("fixture");
        drop(connection);

        fs::write(
            path.join("config.json"),
            r#"{"ai_provider":"local","ai_model":"test"}"#,
        )
        .expect("config");
        db_path
    }
}
