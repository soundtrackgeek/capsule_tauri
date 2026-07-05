use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::Emitter;

use crate::{
    ai_config, ai_providers, backup, db, entries,
    models::{
        AiChatChunkEvent, AiChatCompleteEvent, AiChatContextEvent, AiChatContextPreviewEntry,
        AiChatContextPreviewRequest, AiChatContextPreviewResponse, AiChatErrorEvent,
        AiChatInterruptedEvent, AiChatRequest, AiChatRetryRequest, AiChatStartedEvent,
        AiChatStreamStartResponse, AiConversationDetail, AiConversationListResponse,
        AiConversationMessage, AiConversationSummary, DeleteAiConversationResponse, Entry,
        EntryFilters, EntrySort,
    },
    threads,
};

const EVENT_STARTED: &str = "ai-chat-started";
const EVENT_CONTEXT: &str = "ai-chat-context";
const EVENT_CHUNK: &str = "ai-chat-chunk";
const EVENT_COMPLETE: &str = "ai-chat-complete";
const EVENT_INTERRUPTED: &str = "ai-chat-interrupted";
const EVENT_ERROR: &str = "ai-chat-error";
const CONTEXT_PAGE_LIMIT: i64 = 200;
const LARGE_CONTEXT_WARNING: usize = 50;

static ACTIVE_STREAMS: OnceLock<Mutex<HashMap<String, ActiveStreamState>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct PreparedStream {
    stream_id: String,
    conversation_id: i64,
    assistant_message_id: i64,
    provider: String,
    model: String,
    context: AiChatContextPreviewResponse,
    provider_messages: Vec<ai_providers::ProviderChatMessage>,
    system_prompt: String,
    api_key: String,
}

#[derive(Clone)]
struct ActiveStreamState {
    cancel_flag: Arc<AtomicBool>,
    assistant_message_id: i64,
}

#[derive(Debug, Clone)]
struct ExistingConversation {
    cloud_provider: String,
    scope: String,
    scope_identifiers: Vec<String>,
    context_limit: Option<i64>,
    since: Option<String>,
    until: Option<String>,
}

struct ConversationTurnMetadata<'a> {
    provider: &'a str,
    model: &'a str,
    scope: &'a str,
    scope_identifiers: &'a [String],
    context_limit: Option<i64>,
    since: Option<&'a str>,
    until: Option<&'a str>,
}

pub fn preview_ai_chat_context(
    input: AiChatContextPreviewRequest,
) -> Result<AiChatContextPreviewResponse> {
    preview_ai_chat_context_for_database(&db::resolve_database_path(), input)
}

pub(crate) fn preview_ai_chat_context_for_database(
    db_path: &Path,
    input: AiChatContextPreviewRequest,
) -> Result<AiChatContextPreviewResponse> {
    let mut warnings = Vec::new();
    let scope = normalize_scope(&input.scope)?;
    let effective_limit = input.context_limit.or_else(|| {
        input
            .context_filters
            .as_ref()
            .and_then(|filters| filters.limit)
    });
    if matches!(effective_limit, Some(limit) if limit < 1) {
        return Err(anyhow!("Context limit must be a positive integer or all."));
    }
    let include_hidden = input
        .context_filters
        .as_ref()
        .and_then(|filters| filters.include_hidden)
        .unwrap_or(false);

    let entries = if let Some(uuids) = normalized_uuid_list(input.context_entry_uuids.clone()) {
        entries_by_uuid_contract(db_path, uuids, include_hidden, &mut warnings)?
    } else {
        match scope.as_str() {
            "search" => search_context_entries(db_path, &input, effective_limit)?,
            "entry" => explicit_context_entries(
                db_path,
                vec![first_identifier(
                    &input.scope_identifiers,
                    "Entry scope requires one entry.",
                )?],
                include_hidden,
                &mut warnings,
            )?,
            "entries" => explicit_context_entries(
                db_path,
                input.scope_identifiers.clone(),
                include_hidden,
                &mut warnings,
            )?,
            "thread" => thread_context_entries(db_path, &input, include_hidden, &mut warnings)?,
            _ => unreachable!(),
        }
    };

    let total = entries.len() as i64;
    if effective_limit.is_none() && entries.len() >= LARGE_CONTEXT_WARNING {
        warnings.push(format!(
            "Context limit is all; {total} entries will be sent if you continue."
        ));
    }

    Ok(AiChatContextPreviewResponse {
        scope,
        entries: entries.into_iter().map(preview_entry_from_entry).collect(),
        total,
        context_limit: effective_limit,
        warnings,
    })
}

pub fn list_ai_conversations() -> Result<AiConversationListResponse> {
    list_ai_conversations_for_database(&db::resolve_database_path())
}

pub(crate) fn list_ai_conversations_for_database(
    db_path: &Path,
) -> Result<AiConversationListResponse> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_ai_chat_schema(&connection)?;
    mark_stale_streaming_messages(&connection)?;
    let conversations = list_conversation_summaries(&connection, None)?;
    Ok(AiConversationListResponse {
        conversations,
        warnings: Vec::new(),
    })
}

pub fn get_ai_conversation(conversation_id: i64) -> Result<AiConversationDetail> {
    get_ai_conversation_for_database(&db::resolve_database_path(), conversation_id)
}

pub(crate) fn get_ai_conversation_for_database(
    db_path: &Path,
    conversation_id: i64,
) -> Result<AiConversationDetail> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_ai_chat_schema(&connection)?;
    mark_stale_streaming_messages(&connection)?;
    conversation_detail(&connection, conversation_id)
}

pub fn delete_ai_conversation(conversation_id: i64) -> Result<DeleteAiConversationResponse> {
    let db_path = db::resolve_database_path();
    let guarded =
        backup::with_database_backup_for_database(&db_path, "ai.chat.delete", move |path| {
            delete_ai_conversation_inner(path, conversation_id)
        })?;
    Ok(DeleteAiConversationResponse {
        conversation_id,
        conversation_uuid: guarded.value,
        audit: guarded.audit,
    })
}

pub fn start_ai_chat_stream<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    input: AiChatRequest,
) -> Result<AiChatStreamStartResponse> {
    let db_path = db::resolve_database_path();
    let prepared = prepare_start_stream(&db_path, input)?;
    launch_stream(app, db_path, prepared)
}

pub fn retry_ai_chat_stream<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    input: AiChatRetryRequest,
) -> Result<AiChatStreamStartResponse> {
    let db_path = db::resolve_database_path();
    let prepared = prepare_retry_stream(&db_path, input)?;
    launch_stream(app, db_path, prepared)
}

pub fn cancel_ai_chat_stream(stream_id: String) -> Result<()> {
    let Some(cancelled) = active_streams()
        .lock()
        .ok()
        .and_then(|map| map.get(&stream_id).map(|state| state.cancel_flag.clone()))
    else {
        return Ok(());
    };
    cancelled.store(true, Ordering::Relaxed);
    Ok(())
}

fn launch_stream<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db_path: PathBuf,
    prepared: PreparedStream,
) -> Result<AiChatStreamStartResponse> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    active_streams()
        .lock()
        .map_err(|_| anyhow!("AI stream registry is unavailable."))?
        .insert(
            prepared.stream_id.clone(),
            ActiveStreamState {
                cancel_flag: cancel_flag.clone(),
                assistant_message_id: prepared.assistant_message_id,
            },
        );

    let response = AiChatStreamStartResponse {
        stream_id: prepared.stream_id.clone(),
        conversation_id: prepared.conversation_id,
        assistant_message_id: prepared.assistant_message_id,
        provider: prepared.provider.clone(),
        model: prepared.model.clone(),
    };
    let started = AiChatStartedEvent {
        stream_id: prepared.stream_id.clone(),
        conversation_id: prepared.conversation_id,
        assistant_message_id: prepared.assistant_message_id,
        provider: prepared.provider.clone(),
        model: prepared.model.clone(),
    };
    app.emit(EVENT_STARTED, &started)
        .context("failed to emit AI chat started event")?;
    app.emit(
        EVENT_CONTEXT,
        &AiChatContextEvent {
            stream_id: prepared.stream_id.clone(),
            conversation_id: prepared.conversation_id,
            context: prepared.context.clone(),
        },
    )
    .context("failed to emit AI chat context event")?;

    tauri::async_runtime::spawn_blocking(move || {
        run_stream_worker(app, db_path, prepared, cancel_flag);
    });

    Ok(response)
}

fn run_stream_worker<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db_path: PathBuf,
    prepared: PreparedStream,
    cancel_flag: Arc<AtomicBool>,
) {
    let mut content = String::new();
    let request = ai_providers::ProviderStreamRequest {
        provider: prepared.provider.clone(),
        model: prepared.model.clone(),
        api_key: prepared.api_key.clone(),
        system_prompt: prepared.system_prompt.clone(),
        messages: prepared.provider_messages.clone(),
    };
    let outcome = ai_providers::stream_text(request, cancel_flag.clone(), |chunk| {
        content.push_str(chunk);
        update_assistant_message(
            &db_path,
            prepared.assistant_message_id,
            &content,
            "streaming",
        )?;
        app.emit(
            EVENT_CHUNK,
            &AiChatChunkEvent {
                stream_id: prepared.stream_id.clone(),
                conversation_id: prepared.conversation_id,
                assistant_message_id: prepared.assistant_message_id,
                chunk: chunk.to_string(),
                content: content.clone(),
            },
        )
        .context("failed to emit AI chat chunk event")?;
        Ok(())
    });

    match outcome {
        Ok(ai_providers::ProviderStreamOutcome::Complete) => {
            let final_content = if content.trim().is_empty() {
                "The provider completed without returning text.".to_string()
            } else {
                content.clone()
            };
            let _ = update_assistant_message(
                &db_path,
                prepared.assistant_message_id,
                &final_content,
                "complete",
            );
            let _ = app.emit(
                EVENT_COMPLETE,
                &AiChatCompleteEvent {
                    stream_id: prepared.stream_id.clone(),
                    conversation_id: prepared.conversation_id,
                    assistant_message_id: prepared.assistant_message_id,
                    content: final_content,
                },
            );
        }
        Ok(ai_providers::ProviderStreamOutcome::Cancelled) => {
            let _ = update_assistant_message(
                &db_path,
                prepared.assistant_message_id,
                &content,
                "interrupted",
            );
            let _ = app.emit(
                EVENT_INTERRUPTED,
                &AiChatInterruptedEvent {
                    stream_id: prepared.stream_id.clone(),
                    conversation_id: prepared.conversation_id,
                    assistant_message_id: prepared.assistant_message_id,
                    content,
                    reason: "cancelled".to_string(),
                },
            );
        }
        Err(error) => {
            let safe_message = safe_provider_error(&error.to_string());
            let _ = update_assistant_message(
                &db_path,
                prepared.assistant_message_id,
                &content,
                "error",
            );
            let _ = app.emit(
                EVENT_ERROR,
                &AiChatErrorEvent {
                    stream_id: prepared.stream_id.clone(),
                    conversation_id: prepared.conversation_id,
                    assistant_message_id: prepared.assistant_message_id,
                    content,
                    message: safe_message,
                    detail: None,
                },
            );
        }
    }

    if let Ok(mut streams) = active_streams().lock() {
        streams.remove(&prepared.stream_id);
    }
}

fn prepare_start_stream(db_path: &Path, input: AiChatRequest) -> Result<PreparedStream> {
    let message = normalize_required(&input.message, "Message")?;
    let settings = ai_config::get_ai_settings_for_database(db_path)?;
    let provider = normalize_provider(input.cloud_provider.as_deref(), &settings.cloud_provider)?;
    let model = ai_config::model_for_provider(&settings, &provider, input.model.as_deref())?;
    let api_key = ai_config::api_key_for_provider(db_path, &provider)?;
    let preview_request = AiChatContextPreviewRequest {
        message: Some(message.clone()),
        scope: input.scope.clone(),
        scope_identifiers: input.scope_identifiers.clone(),
        context_filters: input.context_filters.clone(),
        context_limit: input.context_limit,
        since: input.since.clone(),
        until: input.until.clone(),
        context_entry_uuids: input.context_entry_uuids.clone(),
    };
    let context = preview_ai_chat_context_for_database(db_path, preview_request)?;
    let context_identifiers = preview_context_uuids(&context);
    let system_prompt = build_system_prompt_for_database(db_path, &context)?;
    let metadata = ConversationTurnMetadata {
        provider: &provider,
        model: &model,
        scope: &context.scope,
        scope_identifiers: &context_identifiers,
        context_limit: input.context_limit,
        since: input.since.as_deref(),
        until: input.until.as_deref(),
    };

    let guarded = backup::with_database_backup_for_database(db_path, "ai.chat.start", |path| {
        let connection = db::open_read_write_connection(path)?;
        ensure_ai_chat_schema(&connection)?;
        mark_stale_streaming_messages(&connection)?;
        let conversation_id = match input.conversation_id {
            Some(id) => {
                update_conversation_for_turn(&connection, id, &metadata)?;
                id
            }
            None => insert_conversation(&connection, &metadata, &message)?,
        };
        insert_message(
            &connection,
            conversation_id,
            "user",
            &message,
            "complete",
            1,
        )?;
        let assistant_message_id = insert_message(
            &connection,
            conversation_id,
            "assistant",
            "",
            "streaming",
            2,
        )?;
        refresh_conversation_summary(&connection, conversation_id)?;
        let provider_messages = provider_messages_for_conversation(&connection, conversation_id)?;
        Ok((conversation_id, assistant_message_id, provider_messages))
    })?;

    Ok(PreparedStream {
        stream_id: generate_runtime_id("stream"),
        conversation_id: guarded.value.0,
        assistant_message_id: guarded.value.1,
        provider,
        model,
        context,
        provider_messages: guarded.value.2,
        system_prompt,
        api_key: api_key.value,
    })
}

fn prepare_retry_stream(db_path: &Path, input: AiChatRetryRequest) -> Result<PreparedStream> {
    let settings = ai_config::get_ai_settings_for_database(db_path)?;
    let existing = load_existing_conversation(db_path, input.conversation_id)?;
    let provider = normalize_provider(input.cloud_provider.as_deref(), &existing.cloud_provider)
        .or_else(|_| {
            normalize_provider(input.cloud_provider.as_deref(), &settings.cloud_provider)
        })?;
    let model = ai_config::model_for_provider(&settings, &provider, input.model.as_deref())?;
    let api_key = ai_config::api_key_for_provider(db_path, &provider)?;
    let last_user_message = last_user_message(db_path, input.conversation_id)?;
    let preview_request = AiChatContextPreviewRequest {
        message: Some(last_user_message),
        scope: existing.scope.clone(),
        scope_identifiers: existing.scope_identifiers.clone(),
        context_filters: None,
        context_limit: existing.context_limit,
        since: existing.since.clone(),
        until: existing.until.clone(),
        context_entry_uuids: input.context_entry_uuids.clone(),
    };
    let context = preview_ai_chat_context_for_database(db_path, preview_request)?;
    let system_prompt = build_system_prompt_for_database(db_path, &context)?;

    let guarded = backup::with_database_backup_for_database(db_path, "ai.chat.retry", |path| {
        let connection = db::open_read_write_connection(path)?;
        ensure_ai_chat_schema(&connection)?;
        mark_stale_streaming_messages(&connection)?;
        update_conversation_provider(&connection, input.conversation_id, &provider, &model)?;
        let assistant_message_id = insert_message(
            &connection,
            input.conversation_id,
            "assistant",
            "",
            "streaming",
            2,
        )?;
        refresh_conversation_summary(&connection, input.conversation_id)?;
        let provider_messages =
            provider_messages_for_conversation(&connection, input.conversation_id)?;
        Ok((assistant_message_id, provider_messages))
    })?;

    Ok(PreparedStream {
        stream_id: generate_runtime_id("stream"),
        conversation_id: input.conversation_id,
        assistant_message_id: guarded.value.0,
        provider,
        model,
        context,
        provider_messages: guarded.value.1,
        system_prompt,
        api_key: api_key.value,
    })
}

fn search_context_entries(
    db_path: &Path,
    input: &AiChatContextPreviewRequest,
    effective_limit: Option<i64>,
) -> Result<Vec<Entry>> {
    let filters = input.context_filters.clone();
    let query_text = filters
        .as_ref()
        .and_then(|filters| normalize_optional(filters.text.as_deref()))
        .or_else(|| normalize_optional(input.message.as_deref()));
    let mut entry_filters = EntryFilters {
        text: query_text,
        since: normalize_optional(input.since.as_deref()).or_else(|| {
            filters
                .as_ref()
                .and_then(|filters| normalize_optional(filters.since.as_deref()))
        }),
        until: normalize_optional(input.until.as_deref()).or_else(|| {
            filters
                .as_ref()
                .and_then(|filters| normalize_optional(filters.until.as_deref()))
        }),
        tags: filters
            .as_ref()
            .and_then(|filters| normalize_vec(filters.tags.clone())),
        exclude_tags: filters
            .as_ref()
            .and_then(|filters| normalize_vec(filters.exclude_tags.clone())),
        moods: filters
            .as_ref()
            .and_then(|filters| normalize_vec(filters.moods.clone())),
        exclude_moods: filters
            .as_ref()
            .and_then(|filters| normalize_vec(filters.exclude_moods.clone())),
        starred: filters.as_ref().and_then(|filters| filters.starred),
        pinned: filters.as_ref().and_then(|filters| filters.pinned),
        include_hidden: filters.as_ref().and_then(|filters| filters.include_hidden),
        has_images: filters.as_ref().and_then(|filters| filters.has_images),
        sort: filters
            .as_ref()
            .and_then(|filters| filters.sort.clone())
            .or(Some(EntrySort::Desc)),
        ..EntryFilters::default()
    };

    let mut entries = Vec::new();
    let mut offset = 0;
    loop {
        let remaining = effective_limit
            .map(|limit| limit.saturating_sub(entries.len() as i64))
            .unwrap_or(CONTEXT_PAGE_LIMIT);
        if remaining <= 0 {
            break;
        }
        entry_filters.limit = Some(remaining.min(CONTEXT_PAGE_LIMIT));
        entry_filters.offset = Some(offset);
        let response = entries::list_entries_for_database(db_path, entry_filters.clone())?;
        offset += response.limit;
        let batch_is_empty = response.entries.is_empty();
        let response_total = response.total;
        entries.extend(response.entries);
        if entries.len() as i64 >= response_total || batch_is_empty {
            break;
        }
        if effective_limit.is_none() {
            continue;
        }
    }
    Ok(entries)
}

fn explicit_context_entries(
    db_path: &Path,
    identifiers: Vec<String>,
    include_hidden: bool,
    warnings: &mut Vec<String>,
) -> Result<Vec<Entry>> {
    if identifiers.is_empty() {
        return Ok(Vec::new());
    }
    let mut uuids = Vec::new();
    for identifier in identifiers {
        let entry = entries::get_entry_for_database(db_path, &identifier)?;
        uuids.push(entry.uuid);
    }
    entries_by_uuid_contract(db_path, uuids, include_hidden, warnings)
}

fn thread_context_entries(
    db_path: &Path,
    input: &AiChatContextPreviewRequest,
    include_hidden: bool,
    warnings: &mut Vec<String>,
) -> Result<Vec<Entry>> {
    let anchor = first_identifier(
        &input.scope_identifiers,
        "Thread scope requires an anchor entry.",
    )?;
    let entry = entries::get_entry_for_database(db_path, &anchor)?;
    let root_uuid = entry
        .thread
        .as_ref()
        .map(|thread| thread.root_uuid.clone())
        .unwrap_or_else(|| entry.uuid.clone());
    let mut offset = 0;
    loop {
        let response = threads::list_threads_for_database(db_path, Some(200), Some(offset))?;
        if let Some(thread) = response
            .threads
            .into_iter()
            .find(|thread| thread.root_uuid == root_uuid)
        {
            let uuids = thread
                .entries
                .into_iter()
                .map(|entry| entry.uuid)
                .collect::<Vec<_>>();
            return entries_by_uuid_contract(db_path, uuids, include_hidden, warnings);
        }
        offset += response.limit;
        if offset >= response.total {
            break;
        }
    }
    entries_by_uuid_contract(db_path, vec![entry.uuid], include_hidden, warnings)
}

fn entries_by_uuid_contract(
    db_path: &Path,
    uuids: Vec<String>,
    include_hidden: bool,
    warnings: &mut Vec<String>,
) -> Result<Vec<Entry>> {
    let uuids = normalized_uuid_list(Some(uuids)).unwrap_or_default();
    let entries = entries::list_entries_by_uuids_for_database(db_path, &uuids)?;
    let mut filtered = Vec::new();
    for entry in entries {
        if entry.hidden && !include_hidden {
            warnings.push(format!(
                "Hidden entry {} was excluded from AI context.",
                entry.uuid
            ));
            continue;
        }
        filtered.push(entry);
    }
    Ok(filtered)
}

fn preview_entry_from_entry(entry: Entry) -> AiChatContextPreviewEntry {
    AiChatContextPreviewEntry {
        id: entry.id,
        uuid: entry.uuid,
        created_at: entry.created_at,
        title: entry.title,
        summary: entry.summary,
        mood: entry.mood,
        tags: entry.tags.into_iter().map(|tag| tag.name).collect(),
        hidden: entry.hidden,
        attachment_count: entry.attachment_count,
        thread_root_uuid: entry.thread.as_ref().map(|thread| thread.root_uuid.clone()),
        thread_title: entry
            .thread
            .as_ref()
            .and_then(|thread| thread.title.clone()),
        estimated_chars: entry.text_plain.chars().count(),
        text_preview: truncate_for_display(&entry.text_plain, 260),
    }
}

fn preview_context_uuids(context: &AiChatContextPreviewResponse) -> Vec<String> {
    context
        .entries
        .iter()
        .map(|entry| entry.uuid.clone())
        .collect()
}

fn build_system_prompt_for_database(
    db_path: &Path,
    context: &AiChatContextPreviewResponse,
) -> Result<String> {
    let uuids = preview_context_uuids(context);
    let entries = entries::list_entries_by_uuids_for_database(db_path, &uuids)?;
    let full_text_by_uuid = entries
        .into_iter()
        .map(|entry| (entry.uuid, entry.text_plain))
        .collect::<HashMap<_, _>>();
    Ok(build_system_prompt(context, &full_text_by_uuid))
}

fn build_system_prompt(
    context: &AiChatContextPreviewResponse,
    full_text_by_uuid: &HashMap<String, String>,
) -> String {
    let mut prompt = String::from(
        "You are Capsule's private journal chat assistant. Answer from the selected journal context only. Be careful, humane, and explicit when the context is insufficient.\n\nSelected context:\n",
    );
    if context.entries.is_empty() {
        prompt.push_str("No journal entries were selected for this turn.\n");
        return prompt;
    }
    for (index, entry) in context.entries.iter().enumerate() {
        prompt.push_str(&format!(
            "\n[Entry {}]\nUUID: {}\nCreated: {}\n",
            index + 1,
            entry.uuid,
            entry.created_at
        ));
        if let Some(title) = &entry.title {
            prompt.push_str(&format!("Title: {title}\n"));
        }
        if let Some(summary) = &entry.summary {
            prompt.push_str(&format!("Summary: {summary}\n"));
        }
        if let Some(mood) = &entry.mood {
            prompt.push_str(&format!("Mood: {mood}\n"));
        }
        if !entry.tags.is_empty() {
            prompt.push_str(&format!("Tags: {}\n", entry.tags.join(", ")));
        }
        if let Some(root) = &entry.thread_root_uuid {
            prompt.push_str(&format!("Thread root: {root}\n"));
        }
        if entry.attachment_count > 0 {
            prompt.push_str(&format!(
                "Image attachments: {} metadata-only attachment(s); image files are not included.\n",
                entry.attachment_count
            ));
        }
        prompt.push_str("Text:\n");
        prompt.push_str(
            full_text_by_uuid
                .get(&entry.uuid)
                .map(String::as_str)
                .unwrap_or(&entry.text_preview),
        );
        prompt.push('\n');
    }
    prompt
}

pub(crate) fn ensure_ai_chat_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS ai_conversations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE,
            title TEXT NOT NULL,
            preview TEXT NOT NULL DEFAULT '',
            cloud_provider TEXT NOT NULL DEFAULT 'gemini',
            model TEXT,
            scope TEXT NOT NULL DEFAULT 'search',
            scope_identifiers TEXT NOT NULL DEFAULT '[]',
            context_limit INTEGER,
            since TEXT,
            until TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_message_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS ai_conversation_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id INTEGER NOT NULL,
            uuid TEXT UNIQUE,
            role TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'complete',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            sort_key TEXT,
            FOREIGN KEY (conversation_id) REFERENCES ai_conversations(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS sync_ai_conversation_tombstones (
            conversation_uuid TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_ai_conversations_updated_at
            ON ai_conversations(updated_at);
        CREATE INDEX IF NOT EXISTS idx_ai_messages_conversation_sort
            ON ai_conversation_messages(conversation_id, sort_key, id);
        ",
    )?;
    add_missing_column(connection, "ai_conversations", "model", "TEXT")?;
    Ok(())
}

fn insert_conversation(
    connection: &Connection,
    metadata: &ConversationTurnMetadata<'_>,
    message: &str,
) -> Result<i64> {
    let now = current_timestamp();
    let uuid = generate_unique_prefixed_id(connection, "ai_conversations", "uuid", "chat")?;
    connection.execute(
        "INSERT INTO ai_conversations (
            uuid, title, preview, cloud_provider, model, scope, scope_identifiers,
            context_limit, since, until, created_at, updated_at, last_message_at
         ) VALUES (?1, ?2, '', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?10)",
        params![
            uuid,
            truncate_for_display(message, 80),
            metadata.provider,
            metadata.model,
            metadata.scope,
            serde_json::to_string(metadata.scope_identifiers)?,
            metadata.context_limit,
            normalize_optional(metadata.since),
            normalize_optional(metadata.until),
            now,
        ],
    )?;
    Ok(connection.last_insert_rowid())
}

fn update_conversation_for_turn(
    connection: &Connection,
    conversation_id: i64,
    metadata: &ConversationTurnMetadata<'_>,
) -> Result<()> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM ai_conversations WHERE id = ?1 LIMIT 1",
            [conversation_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !exists {
        return Err(anyhow!("AI conversation not found."));
    }
    connection.execute(
        "UPDATE ai_conversations
         SET cloud_provider = ?1,
             model = ?2,
             scope = ?3,
             scope_identifiers = ?4,
             context_limit = ?5,
             since = ?6,
             until = ?7,
             updated_at = ?8
         WHERE id = ?9",
        params![
            metadata.provider,
            metadata.model,
            metadata.scope,
            serde_json::to_string(metadata.scope_identifiers)?,
            metadata.context_limit,
            normalize_optional(metadata.since),
            normalize_optional(metadata.until),
            current_timestamp(),
            conversation_id,
        ],
    )?;
    Ok(())
}

fn update_conversation_provider(
    connection: &Connection,
    conversation_id: i64,
    provider: &str,
    model: &str,
) -> Result<()> {
    connection.execute(
        "UPDATE ai_conversations
         SET cloud_provider = ?1, model = ?2, updated_at = ?3
         WHERE id = ?4",
        params![provider, model, current_timestamp(), conversation_id],
    )?;
    Ok(())
}

fn insert_message(
    connection: &Connection,
    conversation_id: i64,
    role: &str,
    content: &str,
    status: &str,
    ordinal: i64,
) -> Result<i64> {
    let now = current_timestamp();
    let uuid = generate_unique_prefixed_id(connection, "ai_conversation_messages", "uuid", "msg")?;
    let sort_key = format!("{}.{:03}", now.replace([' ', ':', '-'], ""), ordinal);
    connection.execute(
        "INSERT INTO ai_conversation_messages
            (conversation_id, uuid, role, content, status, created_at, updated_at, sort_key)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)",
        params![conversation_id, uuid, role, content, status, now, sort_key],
    )?;
    Ok(connection.last_insert_rowid())
}

fn update_assistant_message(
    db_path: &Path,
    message_id: i64,
    content: &str,
    status: &str,
) -> Result<()> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_ai_chat_schema(&connection)?;
    let now = current_timestamp();
    let conversation_id = connection
        .query_row(
            "SELECT conversation_id FROM ai_conversation_messages WHERE id = ?1",
            [message_id],
            |row| row.get::<_, i64>(0),
        )
        .context("AI assistant message not found.")?;
    connection.execute(
        "UPDATE ai_conversation_messages
         SET content = ?1, status = ?2, updated_at = ?3
         WHERE id = ?4",
        params![content, status, now, message_id],
    )?;
    refresh_conversation_summary(&connection, conversation_id)?;
    Ok(())
}

fn provider_messages_for_conversation(
    connection: &Connection,
    conversation_id: i64,
) -> Result<Vec<ai_providers::ProviderChatMessage>> {
    let mut statement = connection.prepare(
        "SELECT role, content, status
         FROM ai_conversation_messages
         WHERE conversation_id = ?1
         ORDER BY COALESCE(sort_key, created_at) ASC, id ASC",
    )?;
    let rows = statement
        .query_map([conversation_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, content, status)| !content.trim().is_empty() && status != "streaming")
        .filter(|(role, _, status)| role == "user" || (role == "assistant" && status == "complete"))
        .map(|(role, content, _)| ai_providers::ProviderChatMessage { role, content })
        .collect())
}

fn conversation_detail(
    connection: &Connection,
    conversation_id: i64,
) -> Result<AiConversationDetail> {
    let summary = list_conversation_summaries(connection, Some(conversation_id))?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("AI conversation not found."))?;
    let mut metadata = connection.prepare(
        "SELECT scope_identifiers, context_limit, since, until
         FROM ai_conversations WHERE id = ?1",
    )?;
    let (scope_identifiers, context_limit, since, until): (
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
    ) = metadata.query_row([conversation_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;
    let mut message_statement = connection.prepare(
        "SELECT id, COALESCE(NULLIF(uuid, ''), 'msg_' || id), role, content, status, created_at, updated_at
         FROM ai_conversation_messages
         WHERE conversation_id = ?1
         ORDER BY COALESCE(sort_key, created_at) ASC, id ASC",
    )?;
    let messages = message_statement
        .query_map([conversation_id], |row| {
            Ok(AiConversationMessage {
                id: row.get(0)?,
                uuid: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(AiConversationDetail {
        id: summary.id,
        uuid: summary.uuid,
        title: summary.title,
        preview: summary.preview,
        cloud_provider: summary.cloud_provider,
        model: summary.model,
        scope: summary.scope,
        scope_identifiers: parse_scope_identifiers(&scope_identifiers),
        context_limit,
        since,
        until,
        message_count: summary.message_count,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
        last_message_at: summary.last_message_at,
        messages,
    })
}

fn list_conversation_summaries(
    connection: &Connection,
    only_id: Option<i64>,
) -> Result<Vec<AiConversationSummary>> {
    let where_clause = only_id.map(|_| "WHERE c.id = ?1").unwrap_or("");
    let sql = format!(
        "SELECT c.id,
                COALESCE(NULLIF(c.uuid, ''), 'chat_' || c.id),
                c.title,
                c.preview,
                c.cloud_provider,
                c.model,
                c.scope,
                (SELECT COUNT(*) FROM ai_conversation_messages m WHERE m.conversation_id = c.id),
                c.created_at,
                c.last_message_at,
                c.updated_at
         FROM ai_conversations c
         {where_clause}
         ORDER BY datetime(c.updated_at) DESC, c.id DESC"
    );
    let mut statement = connection.prepare(&sql)?;
    if let Some(id) = only_id {
        let rows = statement.query_map([id], conversation_summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    } else {
        let rows = statement.query_map([], conversation_summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn conversation_summary_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AiConversationSummary> {
    Ok(AiConversationSummary {
        id: row.get(0)?,
        uuid: row.get(1)?,
        title: row.get(2)?,
        preview: row.get(3)?,
        cloud_provider: row.get(4)?,
        model: row.get(5)?,
        scope: row.get(6)?,
        message_count: row.get(7)?,
        created_at: row.get(8)?,
        last_message_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn refresh_conversation_summary(connection: &Connection, conversation_id: i64) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT role, content, created_at, updated_at
         FROM ai_conversation_messages
         WHERE conversation_id = ?1
         ORDER BY COALESCE(sort_key, created_at) ASC, id ASC",
    )?;
    let rows = statement
        .query_map([conversation_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.is_empty() {
        return Ok(());
    }
    let title = rows
        .iter()
        .find(|(role, content, _, _)| role == "user" && !content.trim().is_empty())
        .map(|(_, content, _, _)| truncate_for_display(content, 80))
        .unwrap_or_else(|| "New chat".to_string());
    let preview = rows
        .iter()
        .rev()
        .find(|(_, content, _, _)| !content.trim().is_empty())
        .map(|(_, content, _, _)| truncate_for_display(content, 160))
        .unwrap_or_default();
    let last_message_at = rows
        .iter()
        .map(|(_, _, created_at, updated_at)| {
            if updated_at > created_at {
                updated_at.clone()
            } else {
                created_at.clone()
            }
        })
        .max()
        .unwrap_or_else(current_timestamp);
    connection.execute(
        "UPDATE ai_conversations
         SET title = ?1, preview = ?2, last_message_at = ?3, updated_at = ?3
         WHERE id = ?4",
        params![title, preview, last_message_at, conversation_id],
    )?;
    Ok(())
}

fn delete_ai_conversation_inner(db_path: &Path, conversation_id: i64) -> Result<String> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_ai_chat_schema(&connection)?;
    let conversation_uuid = connection
        .query_row(
            "SELECT COALESCE(NULLIF(uuid, ''), 'chat_' || id) FROM ai_conversations WHERE id = ?1",
            [conversation_id],
            |row| row.get::<_, String>(0),
        )
        .context("AI conversation not found.")?;
    connection.execute(
        "DELETE FROM ai_conversation_messages WHERE conversation_id = ?1",
        [conversation_id],
    )?;
    connection.execute(
        "DELETE FROM ai_conversations WHERE id = ?1",
        [conversation_id],
    )?;
    connection.execute(
        "INSERT INTO sync_ai_conversation_tombstones (conversation_uuid, deleted_at)
         VALUES (?1, ?2)
         ON CONFLICT(conversation_uuid)
         DO UPDATE SET deleted_at = excluded.deleted_at",
        params![conversation_uuid, current_timestamp()],
    )?;
    Ok(conversation_uuid)
}

fn load_existing_conversation(
    db_path: &Path,
    conversation_id: i64,
) -> Result<ExistingConversation> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_ai_chat_schema(&connection)?;
    let row = connection
        .query_row(
            "SELECT cloud_provider, scope, scope_identifiers, context_limit, since, until
             FROM ai_conversations WHERE id = ?1",
            [conversation_id],
            |row| {
                Ok(ExistingConversation {
                    cloud_provider: row.get(0)?,
                    scope: row.get(1)?,
                    scope_identifiers: parse_scope_identifiers(&row.get::<_, String>(2)?),
                    context_limit: row.get(3)?,
                    since: row.get(4)?,
                    until: row.get(5)?,
                })
            },
        )
        .context("AI conversation not found.")?;
    Ok(row)
}

fn last_user_message(db_path: &Path, conversation_id: i64) -> Result<String> {
    let connection = db::open_read_write_connection(db_path)?;
    ensure_ai_chat_schema(&connection)?;
    connection
        .query_row(
            "SELECT content
             FROM ai_conversation_messages
             WHERE conversation_id = ?1 AND role = 'user'
             ORDER BY COALESCE(sort_key, created_at) DESC, id DESC
             LIMIT 1",
            [conversation_id],
            |row| row.get::<_, String>(0),
        )
        .context("No user message is available to retry.")
}

fn mark_stale_streaming_messages(connection: &Connection) -> Result<()> {
    let active_message_ids = active_stream_assistant_message_ids();
    if active_message_ids.is_empty() {
        connection.execute(
            "UPDATE ai_conversation_messages
             SET status = 'interrupted', updated_at = ?1
             WHERE status = 'streaming'",
            [current_timestamp()],
        )?;
        return Ok(());
    }
    let excluded_ids = active_message_ids
        .into_iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    connection.execute(
        &format!(
            "UPDATE ai_conversation_messages
             SET status = ?1, updated_at = ?2
             WHERE status = 'streaming' AND id NOT IN ({excluded_ids})"
        ),
        params!["interrupted", current_timestamp()],
    )?;
    Ok(())
}

fn add_missing_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> Result<()> {
    if table_columns(connection, table_name)?.contains(column_name) {
        return Ok(());
    }
    connection.execute(
        &format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {definition}"),
        [],
    )?;
    Ok(())
}

fn table_columns(connection: &Connection, table_name: &str) -> Result<HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
}

fn generate_unique_prefixed_id(
    connection: &Connection,
    table: &str,
    column: &str,
    prefix: &str,
) -> Result<String> {
    for _ in 0..10_000 {
        let candidate = generate_runtime_id(prefix);
        let exists = connection
            .query_row(
                &format!("SELECT 1 FROM {table} WHERE {column} = ?1 LIMIT 1"),
                [candidate.as_str()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Ok(candidate);
        }
    }
    Err(anyhow!("Unable to generate unique AI identifier."))
}

fn generate_runtime_id(prefix: &str) -> String {
    let seed = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000) as u64;
    format!("{prefix}_{}", base36_12(seed))
}

fn base36_12(mut value: u64) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buffer = [b'0'; 12];
    for index in (0..12).rev() {
        buffer[index] = ALPHABET[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8_lossy(&buffer).to_string()
}

fn active_streams() -> &'static Mutex<HashMap<String, ActiveStreamState>> {
    ACTIVE_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn active_stream_assistant_message_ids() -> Vec<i64> {
    active_streams()
        .lock()
        .map(|streams| {
            streams
                .values()
                .map(|stream| stream.assistant_message_id)
                .collect()
        })
        .unwrap_or_default()
}

fn first_identifier(values: &[String], message: &str) -> Result<String> {
    let Some(value) = values
        .iter()
        .find_map(|value| normalize_optional(Some(value)))
    else {
        return Err(anyhow!(message.to_string()));
    };
    Ok(value)
}

fn normalized_uuid_list(values: Option<Vec<String>>) -> Option<Vec<String>> {
    let values = values?
        .into_iter()
        .filter_map(|value| normalize_optional(Some(&value)))
        .fold(Vec::new(), |mut acc, value| {
            if !acc.contains(&value) {
                acc.push(value);
            }
            acc
        });
    (!values.is_empty()).then_some(values)
}

fn normalize_scope(scope: &str) -> Result<String> {
    match scope.trim().to_lowercase().as_str() {
        "search" => Ok("search".to_string()),
        "entry" => Ok("entry".to_string()),
        "entries" => Ok("entries".to_string()),
        "thread" => Ok("thread".to_string()),
        _ => Err(anyhow!(
            "AI chat scope must be search, entry, entries, or thread."
        )),
    }
}

fn normalize_provider(value: Option<&str>, fallback: &str) -> Result<String> {
    match normalize_optional(value)
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

fn normalize_required(value: &str, label: &str) -> Result<String> {
    normalize_optional(Some(value)).ok_or_else(|| anyhow!("{label} is required."))
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_vec(values: Option<Vec<String>>) -> Option<Vec<String>> {
    let values = values?
        .into_iter()
        .filter_map(|value| normalize_optional(Some(&value)))
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(values)
}

fn parse_scope_identifiers(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn current_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn safe_provider_error(value: &str) -> String {
    if value.to_lowercase().contains("api key") {
        return "The selected provider key was rejected or is missing.".to_string();
    }
    truncate_for_display(value, 220)
}

fn truncate_for_display(value: &str, limit: usize) -> String {
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

    fn fixture_db() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("capsule.db");
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
                    text_plain TEXT,
                    content_format TEXT,
                    title TEXT,
                    summary TEXT,
                    mood TEXT,
                    starred INTEGER DEFAULT 0,
                    pinned INTEGER DEFAULT 0,
                    hidden INTEGER DEFAULT 0
                );
                CREATE TABLE tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE);
                CREATE TABLE entry_tags (entry_id INTEGER NOT NULL, tag_id INTEGER NOT NULL);
                CREATE TABLE plugin_entry_media (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    entry_uuid TEXT NOT NULL,
                    media_id INTEGER NOT NULL,
                    position INTEGER DEFAULT 0,
                    caption TEXT,
                    alt_text TEXT,
                    created_at TEXT
                );
                CREATE TABLE entry_continuations (
                    child_entry_uuid TEXT PRIMARY KEY,
                    parent_entry_uuid TEXT NOT NULL,
                    updated_at TEXT
                );
                CREATE TABLE entry_thread_titles (
                    thread_root_uuid TEXT PRIMARY KEY,
                    title TEXT,
                    updated_at TEXT
                );
                INSERT INTO entries
                    (uuid, created_at, updated_at, text, text_plain, content_format, title, summary, mood, hidden)
                VALUES
                    ('entry_root', '2026-07-01 09:00:00', '2026-07-01 09:00:00', 'Root text', 'Root text', 'markdown', 'Root', 'Root summary', 'focused', 0),
                    ('entry_child', '2026-07-02 09:00:00', '2026-07-02 09:00:00', 'Child text', 'Child text', 'markdown', 'Child', NULL, 'calm', 0),
                    ('entry_hidden', '2026-07-03 09:00:00', '2026-07-03 09:00:00', 'Hidden text', 'Hidden text', 'markdown', 'Hidden', NULL, NULL, 1);
                INSERT INTO tags (name) VALUES ('capsule');
                INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1);
                INSERT INTO plugin_entry_media (entry_uuid, media_id, position, created_at)
                VALUES ('entry_root', 1, 0, '2026-07-01 09:01:00');
                INSERT INTO entry_continuations (child_entry_uuid, parent_entry_uuid, updated_at)
                VALUES ('entry_child', 'entry_root', '2026-07-02 10:00:00');
                INSERT INTO entry_thread_titles (thread_root_uuid, title, updated_at)
                VALUES ('entry_root', 'Thread title', '2026-07-02 10:00:00');
                ",
            )
            .expect("fixture");
        drop(connection);
        (temp, db_path)
    }

    #[test]
    fn ensure_schema_adds_ai_chat_tables_and_model_column() {
        let (_temp, db_path) = fixture_db();
        let connection = db::open_read_write_connection(&db_path).expect("open");

        ensure_ai_chat_schema(&connection).expect("schema");

        let columns = table_columns(&connection, "ai_conversations").expect("columns");
        assert!(columns.contains("model"));
        assert!(table_columns(&connection, "ai_conversation_messages").is_ok());
    }

    #[test]
    fn preview_excludes_hidden_entries_by_default() {
        let (_temp, db_path) = fixture_db();

        let preview = preview_ai_chat_context_for_database(
            &db_path,
            AiChatContextPreviewRequest {
                message: Some("text".to_string()),
                scope: "entries".to_string(),
                scope_identifiers: vec!["entry_root".to_string(), "entry_hidden".to_string()],
                context_filters: None,
                context_limit: None,
                since: None,
                until: None,
                context_entry_uuids: None,
            },
        )
        .expect("preview");

        assert_eq!(preview.entries.len(), 1);
        assert_eq!(preview.entries[0].uuid, "entry_root");
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("Hidden entry")));
    }

    #[test]
    fn preview_thread_uses_visible_thread_entries() {
        let (_temp, db_path) = fixture_db();

        let preview = preview_ai_chat_context_for_database(
            &db_path,
            AiChatContextPreviewRequest {
                message: None,
                scope: "thread".to_string(),
                scope_identifiers: vec!["entry_child".to_string()],
                context_filters: None,
                context_limit: None,
                since: None,
                until: None,
                context_entry_uuids: None,
            },
        )
        .expect("preview");

        assert_eq!(
            preview
                .entries
                .iter()
                .map(|entry| entry.uuid.as_str())
                .collect::<Vec<_>>(),
            vec!["entry_root", "entry_child"]
        );
        assert_eq!(
            preview.entries[0].thread_title.as_deref(),
            Some("Thread title")
        );
    }

    #[test]
    fn preview_context_entry_uuids_keep_requested_order() {
        let (_temp, db_path) = fixture_db();

        let preview = preview_ai_chat_context_for_database(
            &db_path,
            AiChatContextPreviewRequest {
                message: None,
                scope: "entries".to_string(),
                scope_identifiers: vec![],
                context_filters: None,
                context_limit: None,
                since: None,
                until: None,
                context_entry_uuids: Some(vec![
                    "entry_child".to_string(),
                    "entry_root".to_string(),
                ]),
            },
        )
        .expect("preview");

        assert_eq!(
            preview
                .entries
                .iter()
                .map(|entry| entry.uuid.as_str())
                .collect::<Vec<_>>(),
            vec!["entry_child", "entry_root"]
        );
    }

    #[test]
    fn prompt_uses_full_text_from_previewed_entries() {
        let (_temp, db_path) = fixture_db();
        let connection = db::open_read_write_connection(&db_path).expect("open");
        let long_text = format!("{} final-full-text-marker", "alpha ".repeat(80));
        connection
            .execute(
                "UPDATE entries SET text = ?1, text_plain = ?1 WHERE uuid = 'entry_root'",
                [&long_text],
            )
            .expect("update");
        drop(connection);

        let preview = preview_ai_chat_context_for_database(
            &db_path,
            AiChatContextPreviewRequest {
                message: None,
                scope: "entry".to_string(),
                scope_identifiers: vec!["entry_root".to_string()],
                context_filters: None,
                context_limit: None,
                since: None,
                until: None,
                context_entry_uuids: None,
            },
        )
        .expect("preview");
        assert!(!preview.entries[0]
            .text_preview
            .contains("final-full-text-marker"));

        let prompt = build_system_prompt_for_database(&db_path, &preview).expect("prompt");
        assert!(prompt.contains("final-full-text-marker"));
        assert!(prompt.contains("Image attachments: 1"));
    }

    #[test]
    fn conversations_list_get_delete_and_tombstone() {
        let (_temp, db_path) = fixture_db();
        let connection = db::open_read_write_connection(&db_path).expect("open");
        ensure_ai_chat_schema(&connection).expect("schema");
        let identifiers = vec!["entry_child".to_string(), "entry_root".to_string()];
        let metadata = ConversationTurnMetadata {
            provider: "openrouter",
            model: "qwen/qwen3.7-plus",
            scope: "entries",
            scope_identifiers: &identifiers,
            context_limit: Some(2),
            since: Some("2026-07-01"),
            until: None,
        };
        let conversation_id =
            insert_conversation(&connection, &metadata, "hello").expect("conversation");
        insert_message(&connection, conversation_id, "user", "hello", "complete", 1).expect("user");
        insert_message(
            &connection,
            conversation_id,
            "assistant",
            "answer",
            "complete",
            2,
        )
        .expect("assistant");
        refresh_conversation_summary(&connection, conversation_id).expect("summary");
        drop(connection);

        let listed = list_ai_conversations_for_database(&db_path).expect("list");
        assert_eq!(listed.conversations.len(), 1);
        assert_eq!(
            listed.conversations[0].model.as_deref(),
            Some("qwen/qwen3.7-plus")
        );

        let detail = get_ai_conversation_for_database(&db_path, conversation_id).expect("detail");
        assert_eq!(detail.messages.len(), 2);
        assert_eq!(detail.scope_identifiers, vec!["entry_child", "entry_root"]);

        let deleted_uuid = delete_ai_conversation_inner(&db_path, conversation_id).expect("delete");
        let connection = db::open_read_write_connection(&db_path).expect("reopen");
        let tombstone: String = connection
            .query_row(
                "SELECT conversation_uuid FROM sync_ai_conversation_tombstones WHERE conversation_uuid = ?1",
                [&deleted_uuid],
                |row| row.get(0),
            )
            .expect("tombstone");
        assert_eq!(tombstone, deleted_uuid);
        let remaining = list_ai_conversations_for_database(&db_path).expect("list after delete");
        assert!(remaining.conversations.is_empty());
    }

    #[test]
    fn stale_streaming_messages_reopen_as_interrupted() {
        let (_temp, db_path) = fixture_db();
        let connection = db::open_read_write_connection(&db_path).expect("open");
        ensure_ai_chat_schema(&connection).expect("schema");
        let identifiers = Vec::new();
        let metadata = ConversationTurnMetadata {
            provider: "gemini",
            model: "gemini-3.5-flash",
            scope: "search",
            scope_identifiers: &identifiers,
            context_limit: Some(5),
            since: None,
            until: None,
        };
        let conversation_id =
            insert_conversation(&connection, &metadata, "hello").expect("conversation");
        insert_message(
            &connection,
            conversation_id,
            "assistant",
            "partial",
            "streaming",
            1,
        )
        .expect("message");

        mark_stale_streaming_messages(&connection).expect("mark stale");

        let status: String = connection
            .query_row(
                "SELECT status FROM ai_conversation_messages WHERE conversation_id = ?1",
                [conversation_id],
                |row| row.get(0),
            )
            .expect("status");
        assert_eq!(status, "interrupted");
    }

    #[test]
    fn active_streaming_messages_are_not_marked_stale() {
        let (_temp, db_path) = fixture_db();
        let connection = db::open_read_write_connection(&db_path).expect("open");
        ensure_ai_chat_schema(&connection).expect("schema");
        let identifiers = Vec::new();
        let metadata = ConversationTurnMetadata {
            provider: "gemini",
            model: "gemini-3.5-flash",
            scope: "search",
            scope_identifiers: &identifiers,
            context_limit: Some(5),
            since: None,
            until: None,
        };
        let conversation_id =
            insert_conversation(&connection, &metadata, "hello").expect("conversation");
        let message_id = insert_message(
            &connection,
            conversation_id,
            "assistant",
            "partial",
            "streaming",
            1,
        )
        .expect("message");
        let stream_id = generate_runtime_id("test_stream");
        active_streams().lock().expect("streams").insert(
            stream_id.clone(),
            ActiveStreamState {
                cancel_flag: Arc::new(AtomicBool::new(false)),
                assistant_message_id: message_id,
            },
        );

        mark_stale_streaming_messages(&connection).expect("mark stale");

        active_streams().lock().expect("streams").remove(&stream_id);
        let status: String = connection
            .query_row(
                "SELECT status FROM ai_conversation_messages WHERE id = ?1",
                [message_id],
                |row| row.get(0),
            )
            .expect("status");
        assert_eq!(status, "streaming");
    }
}
