# Capsule Tauri App Specification

Date: 2026-07-05
Status: Draft for upcoming build
Target repo: `C:\_code\capsule_tauri`
Reference repos inspected:

- `C:\_code\music_backup_v5`
- `C:\_code\capsule_exp_ai`
- `C:\_code\capsule_exp_ai\capsule-web`

## 1. Product Goal

Build a clean, local-first Capsule desktop app using Tauri 2, React, TypeScript, and Vite.

The app should feel like the best parts of the current Capsule web UI, but packaged as a real desktop app that can open the user's existing Capsule database directly, provide fast journaling and browsing workflows, and enforce database safety before every write.

The initial production database is:

```text
C:\Users\jtill\.capsule\capsule.db
```

Observed during spec creation:

- Database exists.
- Database size: `110,792,704` bytes.
- Last modified: `2026-06-29 12:43:21`.
- Existing backups are present in `C:\Users\jtill\.capsule`.

The app must never treat this database as test data. Every database-changing action must create a fresh backup first.

## 2. Guiding Principles

### Local First

Capsule's center of gravity remains local SQLite data. The app must start, browse, search, and write entries without any cloud service.

### Desktop Native, Not A Browser Wrapper

The new app should not be a Tauri shell around the existing FastAPI web server. It should use Tauri commands for desktop operations and SQLite access. A temporary Python bridge may be used for high-risk legacy features, but the main architecture should be native Tauri.

### Preserve Existing Data Semantics

The new app must remain compatible with the existing Capsule CLI and web app database shape:

- Existing entries, tags, moods, history, image attachments, locations, threads, templates, prompts, AI chats, sync metadata, plugin state, and gamification tables must remain readable by the old tools.
- Existing UUIDs remain stable.
- Sequential numeric IDs may be displayed, but UUIDs are the stable identity for links, threads, sync, and external references.

### Backup Before Mutation

Before every write, migration, restore, import, sync, metadata update, attachment update, security change, or bulk operation, the app must create a fresh database backup and must abort the operation if the backup fails.

### Incremental Parity

The current Capsule web app is broad. The Tauri app should reach high-quality desktop journaling first, then add parity modules in phases.

## 3. Inspiration From Music Backup V5

Music Backup V5 provides the implementation pattern to copy:

- Tauri 2 desktop shell.
- React + TypeScript + Vite frontend.
- Small `package.json` scripts:
  - `npm run dev`
  - `npm run build`
  - `npm run tauri:dev`
  - `npm run tauri:build`
- Vite bound to `127.0.0.1` with a strict port.
- Tauri configured with `beforeDevCommand`, `beforeBuildCommand`, `devUrl`, and `frontendDist`.
- Rust modules split by domain, for example:
  - `models.rs`
  - `db.rs`
  - `importer.rs`
  - `covers.rs`
  - `lib.rs`
- Tauri commands that run blocking database work via `tauri::async_runtime::spawn_blocking`.
- Rust structs serialized with camelCase for TypeScript ergonomics.
- A frontend `backend.ts` wrapper that calls Tauri commands in desktop mode and provides mock data in browser-only Vite mode.
- A frontend `types.ts` file mirroring Rust response/request models.
- SQLite pragmas configured centrally.
- Idempotent migrations.
- Long-running work reporting progress via Tauri events.
- Tests in Rust for database/query behavior.

The Capsule Tauri app should use the same pattern, but adapted for the current Capsule database and journaling workflows.

## 4. Current Capsule Web App Features To Preserve

The current web app provides the product vocabulary and UX shape:

- Dashboard with recent entries, pinned entries, random entry, stats, milestones, streaks, sync status, AI Time Capsule, and gamification preview.
- Entries page with inline composer, card/feed views, filters, infinite loading, hidden entries, pinned/starred actions, continuations, image attachments, and draft recovery.
- Full-page composer for longer writing.
- Writer Mode with configurable font, color, font size, line spacing, keyboard shortcuts, save, settings, and exit.
- Threads page with grouped continuation chains, thread titles/summaries, bulk linking, detach, and disband actions.
- Starred page.
- Search page with keyword, semantic, and hybrid modes, structured query tokens, date/tag/mood filters, presets, export, and entry actions.
- Analytics page with period filters, overview metrics, trends, mood/tag charts, writing calendar, word frequency, reading time, weather/location analytics, correlations, and year review.
- Writing Calendar and Wrapped pages.
- AI Features page with OpenAI, Google Gemini, and OpenRouter provider selection, scoped journal chat, persisted conversations, summaries, patterns, journey narration, time capsules, and metadata suggestions.
- Settings with general preferences, Cover Wall, security, backups, AI configuration, export/import, template and prompt library, moods, tags, presets, cloud sync, plugins, and advanced history/undo.
- Cover Wall for generated cover images linked to visible entries.
- Built-in image attachments and location/weather metadata.
- Optional plugins: dream log, coding ideas, writing ideas, post ideas.
- RPG-style gamification and profile badges.
- Shared-folder sync and GitHub Gist mobile import.
- Security modes: plain, AES, and SQLCipher.

The Tauri build should initially focus on the workflows that make a desktop app immediately better:

1. Fast local startup.
2. Reliable database status and backup safety.
3. Entry browsing.
4. Entry creation/editing.
5. Writer Mode.
6. Search.
7. Threads.
8. Backups/settings.

## 5. Non-Goals For The First Build

The first build should not try to replace every Capsule feature.

Out of scope for MVP:

- Full plugin management parity.
- Full cloud sync setup.
- Full encryption migration UI.
- Full analytics parity.
- Full mobile photo bridge management.
- Rewriting every Python service in Rust.
- Changing the existing database schema unless an MVP feature absolutely requires it.

These are phase-two or later modules.

## 6. Recommended Tech Stack

### Desktop Shell

- Tauri 2
- Rust 2021 edition
- `rusqlite` with bundled SQLite
- `serde`, `serde_json`
- `chrono`
- `anyhow`
- `sha2` if operation manifests or checksums are added
- Tauri event system for progress updates

Useful Tauri plugins to evaluate during implementation:

- Dialog plugin for database/backup/import file pickers.
- Opener plugin for opening backup folders or exported files.
- Shell plugin only if a legacy Python bridge is explicitly needed.
- Global shortcut plugin later for quick capture.

### Frontend

- React 19 if starting from the current Capsule frontend conventions.
- TypeScript 5.
- Vite.
- TanStack Query for async command state.
- Zustand for local UI preferences.
- React Router for desktop app routes.
- lucide-react for icons.
- react-hot-toast or an equivalent toast layer.
- Recharts for analytics once analytics lands.
- react-virtuoso for long entry lists.
- Existing Capsule web CSS/Tailwind ideas may be reused, but the Tauri app should simplify the layout and avoid copying dead web-only code.

### Styling

Use a restrained desktop productivity interface:

- Dense but readable.
- Calm theme.
- Strong typography in writing surfaces.
- Fixed app chrome with a left nav and main workspace.
- No landing page.
- No marketing hero.
- No decorative gradients as the main design language.
- Keep entry cards and modals at modest radius.
- Use icon buttons with tooltips for repeated actions such as star, pin, edit, delete, hide, continue, backup, restore, refresh, export, and settings.

## 7. Proposed Repo Structure

```text
capsule_tauri/
  AGENTS.md
  README.md
  CHANGELOG.md
  SPEC.md
  package.json
  package-lock.json
  index.html
  vite.config.ts
  tsconfig.json
  tsconfig.node.json
  src/
    main.tsx
    App.tsx
    backend.ts
    types.ts
    styles.css
    routes/
      Dashboard.tsx
      Entries.tsx
      NewEntry.tsx
      EditEntry.tsx
      WriterMode.tsx
      Threads.tsx
      Search.tsx
      AI.tsx
      Settings.tsx
      Backups.tsx
      About.tsx
    components/
      layout/
      entries/
      ai/
      writer-mode/
      settings/
      ui/
    hooks/
    lib/
    store/
  src-tauri/
    Cargo.toml
    tauri.conf.json
    build.rs
    capabilities/
      default.json
    src/
      main.rs
      lib.rs
      models.rs
      db.rs
      backup.rs
      entries.rs
      search.rs
      threads.rs
      ai.rs
      ai_providers.rs
      settings.rs
      images.rs
      stats.rs
      security.rs
      python_bridge.rs
```

`python_bridge.rs` should exist only if needed. It must not become the default path for ordinary entry browsing or writing.

## 8. Application Configuration

### Database Resolution

MVP default:

```text
C:\Users\jtill\.capsule\capsule.db
```

The app should display this path clearly in Settings and on the first-run database status panel.

Future resolution order should match Capsule:

1. Explicit Tauri setting selected by the user.
2. `db_path` in active Capsule `config.json`.
3. Active Capsule profile default: `~/.capsule/profiles/<name>/capsule.db`.
4. `CAPSULE_DB_PATH`.
5. `CAPSULE_HOME\capsule.db`.
6. `~\.capsule\capsule.db`.

MVP can use the explicit current path while keeping the resolver interface ready for profile support.

### App Settings

Store UI-only settings in localStorage or a Tauri app settings file at first:

- Theme.
- Sidebar mode.
- Entry list view mode.
- Writer Mode preferences.
- Last selected filters.
- Draft recovery state.

Do not add new settings tables to the existing Capsule database in MVP unless necessary.

### Capsule Config

Existing Capsule configuration lives in `config.json` and environment variables. The Tauri app should read config values needed for display and behavior, but must be conservative about writing config until a proper settings command exists.

### Cloud AI Configuration

The Tauri app should support the same cloud AI provider family selected for the old Capsule AI work, without local Ollama as an initial dependency:

- `openai`
- `gemini`
- `openrouter`

Config keys to preserve:

- `cloud_provider`
- `openai_model`
- `gemini_model`
- `openrouter_model`
- `ai_chat_context_limit`
- `ai_chat_context_since`
- `ai_chat_context_until`

Default models should be explicit and editable from Settings. The implementation may update default model IDs over time, but the Settings UI must always show the exact provider and model that will receive a request.

API keys:

- `OPENAI_API_KEY`
- `GEMINI_API_KEY`
- `OPENROUTER_API_KEY`

Secrets must not be stored in the Capsule SQLite database, exported JSON, sync sidecars, backups, or conversation metadata. Preferred storage is the OS credential store through a Tauri-compatible secret plugin. Environment variables and an ignored local `.env` file are acceptable fallback sources. The Settings UI should report key presence as configured/missing without displaying full secret values.

AI settings writes are file/settings mutations, not journal mutations. They should create a config backup before writing local config, but they do not need a database backup unless the operation also mutates SQLite rows.

## 9. Database Safety Contract

### Hard Rule

Every database-changing operation must run through one write guard:

```text
with_database_backup(operation_name, source_context, write_fn)
```

The guard must:

1. Resolve the active database path.
2. Confirm the database exists unless the operation explicitly creates a new database.
3. Create the backup directory.
4. Create a consistent SQLite backup.
5. Verify the backup file exists and is non-empty.
6. Optionally verify the backup can open and contains an `entries` table.
7. Write a small JSON manifest next to the backup.
8. Only then run the write.
9. Return the backup path in the operation response.

If backup creation fails, the write must not run.

### Backup Naming

Use Capsule-compatible names:

```text
capsule_backup_YYYYMMDD_HHMMSS.db
capsule_backup_YYYYMMDD_HHMMSS.json
```

Manifest shape:

```json
{
  "createdAt": "2026-06-29T12:00:00Z",
  "operation": "entry.create",
  "app": "capsule-tauri",
  "dbPath": "C:\\Users\\jtill\\.capsule\\capsule.db",
  "dbSizeBytes": 110792704,
  "backupPath": "C:\\Users\\jtill\\.capsule\\capsule_backup_20260629_120000.db"
}
```

### Backup Method

Preferred:

- SQLite backup API through `rusqlite` backup support.

Acceptable fallback:

- `VACUUM INTO` to a destination backup file, if safe for the active database mode.

Avoid:

- Raw file copy of the main `.db` file while WAL may contain uncheckpointed data.

### Rotation

- Respect the saved backup retention count, defaulting to 5.
- Rotate only files matching `capsule_backup_YYYYMMDD_HHMMSS.db`.
- Delete the matching JSON manifest for each pruned Capsule backup.
- Never delete `.bak`, `_plain_text.db`, or manually named safety copies unless
  the user explicitly requests cleanup.

### Write Operations Requiring Backups

Backups are required before:

- Creating an entry.
- Editing an entry.
- Deleting an entry.
- Star/unstar.
- Pin/unpin.
- Hide/unhide.
- Tag rename/merge/delete.
- Mood create/update/delete.
- Thread link/detach/disband/title/summary update.
- Image upload/attach/remove.
- Location attach/remove/refresh weather.
- Template/prompt create/update/delete/clone/enable/disable.
- Import.
- Sync.
- Restore.
- Migration.
- Security enable/disable.
- Plugin activation/deactivation/update.
- Any schema change.

## 10. SQLite Compatibility Notes

The existing Capsule schema is migration-by-introspection rather than a single `PRAGMA user_version` migration ladder. The Tauri app should therefore detect tables and columns directly.

Core tables and concepts from the current codebase:

- `entries`
  - `id`
  - `uuid`
  - `created_at`
  - `updated_at`
  - `text`
  - `text_plain`
  - `content_format`
  - `title`
  - `summary`
  - `mood`
  - `starred`
  - `pinned`
  - `hidden`
- `tags`
- `entry_tags`
- `history`
- `entries_fts`
- `entry_continuations`
- `entry_thread_titles`
- `entry_thread_summaries`
- `sync_entry_continuation_tombstones`
- `sync_entry_thread_title_tombstones`
- `sync_entry_thread_summary_tombstones`
- `plugin_media_assets`
- `plugin_entry_media`
- `sync_image_tombstones`
- `plugin_entry_locations`
- `plugin_location_cache`
- `sync_location_tombstones`
- `library_templates`
- `library_prompts`
- `sync_template_tombstones`
- `sync_prompt_tombstones`
- `ai_conversations`
- `ai_conversation_messages`
- `sync_ai_conversation_tombstones`
- `ai_time_capsules`
- `embedding_models`
- `embeddings`
- `sync_tombstones`
- `sync_status`
- `sync_history`
- `plugin_state`
- `gamification_xp_events`
- `gamification_quest_state`
- `gamification_profile`
- `gamification_badge_unlocks`

Plugin tables that may exist:

- `plugin_dreams`
- `plugin_coding_ideas`
- `plugin_coding_idea_media`
- `plugin_post_ideas`
- `plugin_writing_ideas`

## 11. Database Read Model

### Entry

TypeScript:

```ts
export type Entry = {
  id: number;
  uuid: string;
  createdAt: string;
  updatedAt: string | null;
  text: string;
  textPlain: string;
  contentFormat: "plain" | "markdown";
  title: string | null;
  summary: string | null;
  mood: string | null;
  moodInfo: MoodInfo;
  tags: TagInfo[];
  starred: boolean;
  pinned: boolean;
  hidden: boolean;
  location: LocationInfo | null;
  thread: EntryThreadInfo | null;
  attachmentCount: number;
};
```

Rust should serialize as camelCase, even if the database columns are snake_case.

### Entry List Response

```ts
export type EntryListResponse = {
  entries: Entry[];
  total: number;
  limit: number;
  offset: number;
};
```

### Entry Filters

```ts
export type EntryFilters = {
  text?: string;
  location?: string;
  since?: string;
  until?: string;
  tags?: string[];
  excludeTags?: string[];
  moods?: string[];
  excludeMoods?: string[];
  starred?: boolean | null;
  pinned?: boolean | null;
  hidden?: boolean | null;
  includeHidden?: boolean;
  hasImages?: boolean | null;
  limit?: number;
  offset?: number;
  sort?: "asc" | "desc";
};
```

### Create Entry

```ts
export type EntryCreate = {
  text: string;
  contentFormat?: "plain" | "markdown";
  title?: string | null;
  summary?: string | null;
  mood?: string | null;
  tags?: string[];
  when?: string | null;
  starred?: boolean;
  pinned?: boolean;
  continueFromUuid?: string | null;
};
```

Create responses must include:

- Created entry.
- Backup path.
- Optional XP award later.

### AI Provider Types

```ts
export type AICloudProvider = "openai" | "gemini" | "openrouter";

export type AIProviderStatus = {
  provider: AICloudProvider;
  label: string;
  configured: boolean;
  selectedModel: string;
  availableModels: string[];
  missingReason: string | null;
};

export type AISettings = {
  cloudProvider: AICloudProvider;
  openaiModel: string;
  geminiModel: string;
  openrouterModel: string;
  defaultContextLimit: number;
  defaultSince: string | null;
  defaultUntil: string | null;
};
```

### AI Chat Types

```ts
export type AIChatScope = "search" | "entry" | "entries" | "thread";
export type AIChatMessageStatus = "streaming" | "complete" | "interrupted" | "error";

export type AIChatContextFilters = {
  text?: string;
  since?: string | null;
  until?: string | null;
  tags?: string[];
  excludeTags?: string[];
  moods?: string[];
  excludeMoods?: string[];
  starred?: boolean | null;
  pinned?: boolean | null;
  includeHidden?: boolean;
  hasImages?: boolean | null;
  limit?: number;
  sort?: "asc" | "desc" | "relevance";
};

export type AIChatRequest = {
  message: string;
  conversationId?: number | null;
  cloudProvider?: AICloudProvider;
  scope: AIChatScope;
  scopeIdentifiers: string[];
  contextFilters?: AIChatContextFilters;
  contextLimit?: number | null;
  since?: string | null;
  until?: string | null;
};

export type AIConversationSummary = {
  id: number;
  uuid: string;
  title: string;
  preview: string;
  cloudProvider: AICloudProvider;
  model: string | null;
  messageCount: number;
  createdAt: string;
  updatedAt: string;
  lastMessageAt: string;
};

export type AIConversationMessage = {
  id: number;
  uuid: string;
  role: "user" | "assistant";
  content: string;
  status: AIChatMessageStatus;
  createdAt: string;
  updatedAt: string;
};

export type AIConversationDetail = AIConversationSummary & {
  scope: AIChatScope;
  scopeIdentifiers: string[];
  contextLimit: number | null;
  since: string | null;
  until: string | null;
  messages: AIConversationMessage[];
};
```

### AI Metadata Suggestions

```ts
export type AIEntryMetadataSuggestionRequest = {
  text: string;
  contentFormat?: "plain" | "markdown";
  cloudProvider?: AICloudProvider;
};

export type AIEntryMetadataSuggestion = {
  title: string | null;
  summary: string | null;
  cloudProvider: AICloudProvider;
  model: string;
};

export type AIThreadMetadataSuggestionRequest = {
  rootUuid: string;
  cloudProvider?: AICloudProvider;
};

export type AIThreadMetadataSuggestion = AIEntryMetadataSuggestion & {
  entryCount: number;
};
```

## 12. Command API

The frontend should call one `src/backend.ts` facade. Desktop mode uses Tauri commands. Browser-only Vite mode returns mock data.

### Status Commands

```text
get_database_status() -> DatabaseStatus
get_app_info() -> AppInfo
```

`DatabaseStatus`:

```ts
export type DatabaseStatus = {
  dbPath: string;
  dbExists: boolean;
  dbSizeBytes: number;
  dbModifiedAt: string | null;
  readable: boolean;
  schemaSummary: SchemaSummary;
  entryCount: number | null;
  tagCount: number | null;
  backupCount: number | null;
  lastBackupPath: string | null;
  security: SecurityStatus;
  warnings: string[];
};
```

### Entry Commands

```text
list_entries(filters: EntryFilters) -> EntryListResponse
get_entry(identifier: string) -> Entry
get_random_entry(filters: RandomEntryFilters) -> Entry | null
create_entry(input: EntryCreate) -> EntryMutationResponse
update_entry(identifier: string, input: EntryUpdate) -> EntryMutationResponse
delete_entry(identifier: string) -> DeleteEntryResponse
star_entry(identifier: string) -> EntryMutationResponse
unstar_entry(identifier: string) -> EntryMutationResponse
pin_entry(identifier: string) -> EntryMutationResponse
unpin_entry(identifier: string) -> EntryMutationResponse
hide_entry(identifier: string) -> EntryMutationResponse
unhide_entry(identifier: string) -> EntryMutationResponse
list_entry_history(identifier: string) -> EntryHistoryResponse
```

Every mutation response includes:

```ts
export type MutationAudit = {
  backupPath: string;
  operation: string;
  completedAt: string;
};
```

### Thread Commands

```text
list_threads(limit: number, offset: number) -> ThreadListResponse
list_continuation_candidates(current_identifier?: string, query?: string, limit?: number) -> ContinuationCandidateListResponse
list_thread_candidates(filters: ThreadCandidateFilters) -> ThreadCandidateListResponse
list_thread_targets(limit: number, offset: number) -> ThreadTargetListResponse
bulk_link_threads(input: BulkThreadLinkRequest) -> ThreadMutationResponse
bulk_detach_threads(input: BulkThreadDetachRequest) -> ThreadMutationResponse
disband_thread(root_uuid: string) -> ThreadMutationResponse
update_thread_title(root_uuid: string, title?: string | null) -> ThreadMutationResponse
update_thread_metadata(root_uuid: string, title?: string | null, summary?: string | null) -> ThreadMutationResponse
```

### Search Commands

```text
search_entries(input: SearchRequest) -> SearchResponse
```

MVP modes:

- `keyword`

Later modes:

- `semantic`
- `hybrid`

Keyword search should use `entries_fts` if available and allowed by security state. Fallback to indexed `text_plain LIKE` only if FTS is absent.

Structured query syntax should be ported from the web app:

- `tag:work`
- `mood:proud`
- `before:2025-06-01`
- `after:2025-01-01`
- `NOT tag:work`

Semantic and hybrid search should not depend on local Ollama for the initial Tauri AI implementation. Keep keyword search native. AI-assisted retrieval can be implemented as scoped cloud chat over selected entries and filters first; true semantic/vector ranking can come later if a cloud embedding or local indexing strategy is chosen deliberately.

### AI Commands

```text
get_ai_provider_status() -> AIProviderStatus[]
get_ai_settings() -> AISettings
update_ai_settings(input: AISettingsUpdate) -> ConfigMutationResponse
preview_ai_chat_context(input: AIChatContextPreviewRequest) -> AIChatContextPreviewResponse
list_ai_conversations() -> AIConversationListResponse
get_ai_conversation(conversation_id: number) -> AIConversationDetail
delete_ai_conversation(conversation_id: number) -> DeleteAIConversationResponse
start_ai_chat_stream(input: AIChatRequest) -> AIChatStreamStartResponse
cancel_ai_chat_stream(stream_id: string) -> void
suggest_entry_metadata(input: AIEntryMetadataSuggestionRequest) -> AIEntryMetadataSuggestion
suggest_thread_metadata(input: AIThreadMetadataSuggestionRequest) -> AIThreadMetadataSuggestion
```

Streaming should use Tauri events:

- `ai-chat-started` with `streamId`, `conversationId`, provider, and model.
- `ai-chat-context` with the resolved context preview.
- `ai-chat-chunk` with incremental assistant text.
- `ai-chat-complete` when the assistant message is complete.
- `ai-chat-interrupted` when the user cancels or navigation interrupts the stream.
- `ai-chat-error` with a user-safe error and technical detail.

The chat command must persist the user message before the provider call and persist the assistant draft as it streams. Stale `streaming` rows should reopen as `interrupted`.

### Backup Commands

```text
list_backups() -> BackupListResponse
create_backup(input?: BackupCreateRequest) -> BackupCreateResponse
restore_backup(input: BackupRestoreRequest) -> BackupRestoreResponse
open_backup_folder() -> void
```

Restore must create a fresh safety backup before replacing the active database.

### Settings Commands

```text
get_capsule_config() -> CapsuleConfigResponse
set_capsule_config_value(key: string, value: string) -> ConfigMutationResponse
delete_capsule_config_value(key: string) -> ConfigMutationResponse
```

Config writes require backups only when they imply database writes. File-only config writes should create a config backup instead.

### Images Commands

MVP can display attachment counts only.

Later:

```text
list_entry_images(identifier: string) -> ImageEntryListResponse
list_images_for_entries(uuids: string[]) -> ImageEntriesListResponse
get_image_data_url(attachment_id: number, variant: "thumb" | "full") -> string
upload_image(file_path: string) -> ImageUploadResponse
attach_image(input: ImageAttachRequest) -> ImageMutationResponse
remove_image(attachment_id: number, identifier?: string) -> ImageMutationResponse
```

Image data must be served by command or a strict custom protocol that only resolves files under the configured Capsule media root.

### Progress Events

Use Tauri events for long-running tasks:

- `backup-progress`
- `restore-progress`
- `import-progress`
- `sync-progress`
- `export-progress`
- `ai-progress`

## 13. Backend Architecture

### `db.rs`

Responsibilities:

- Resolve database path.
- Open SQLite connections.
- Configure pragmas.
- Inspect schema.
- Provide read-only and read-write connection helpers.
- Normalize row values.
- Map SQLite errors to friendly app errors.

Recommended pragmas for normal connections:

```sql
PRAGMA busy_timeout = 15000;
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
```

Do not force WAL if opening read-only or if the database is encrypted/unsupported.

### `backup.rs`

Responsibilities:

- Backup creation.
- Backup verification.
- Backup listing.
- Restore preview.
- Restore execution.
- Operation manifests.
- Optional future rotation.

### `entries.rs`

Responsibilities:

- Entry list queries.
- Entry create/update/delete.
- Star/pin/hide.
- Tag joins and normalization.
- Mood metadata.
- Entry history payloads.
- Stable UUID resolution.
- `text_plain` projection generation for markdown/plain content.

### `threads.rs`

Responsibilities:

- Read and write continuation links.
- Build visible thread groups.
- Set thread titles and summaries.
- Bulk link/detach/disband.
- Enforce no cycles.
- Prefer UUIDs over numeric IDs.

### `search.rs`

Responsibilities:

- Keyword search.
- Structured query parsing.
- FTS availability detection.
- Security degraded-feature checks.
- Relevance sorting for search results.

### `ai.rs`

Responsibilities:

- Provider readiness and settings read models.
- AI context preview assembly from search filters, explicit entries, selected entry lists, and continuation threads.
- Persistent conversation CRUD using `ai_conversations`, `ai_conversation_messages`, and `sync_ai_conversation_tombstones`.
- Chat stream orchestration through Tauri events.
- Entry title/summary and thread title/summary suggestion commands.
- Prompt construction that includes only the entries visible in the resolved context preview.
- Conversation status transitions for `streaming`, `complete`, `interrupted`, and `error`.
- Audit metadata for provider/model used by each AI action.

### `ai_providers.rs`

Responsibilities:

- Direct HTTPS clients for OpenAI, Google Gemini, and OpenRouter text generation.
- Streaming adapters that normalize provider-specific chunk formats into plain text deltas.
- Non-streaming generation for metadata suggestions and analysis workflows.
- Consistent timeout, retry, cancellation, and error mapping behavior.
- Secret lookup from OS credential storage, environment variables, or ignored local `.env` fallback.
- Model allowlists and configured-model fallback.

Provider modules should not have database access. They accept finalized prompts and return generated text or stream chunks.

### `security.rs`

Responsibilities:

- Detect configured security mode.
- Report locked/encrypted state.
- Prevent unsupported direct reads when data is encrypted.
- Later: implement unlock/lock or delegate to Python core.

Important: do not silently show encrypted text blobs as journal content.

### `python_bridge.rs`

Only for features not yet ported to Rust:

- Temporary AI compatibility only if direct Rust provider calls are blocked.
- SQLCipher/AES migrations.
- Complex sync/import/export paths.
- Plugin activation/update.

Rules:

- Must be optional.
- Must not be used for basic entry browsing.
- Must not be the primary path for OpenAI, Gemini, or OpenRouter once `ai_providers.rs` exists.
- Must not spawn a long-lived FastAPI server for the normal desktop app.
- Must surface clear errors when Python or dependencies are missing.

## 14. Frontend Architecture

### `backend.ts`

Use the Music Backup pattern:

- Exports one function per command.
- Internally detects Tauri runtime.
- Calls `invoke` in Tauri.
- Returns mock data in browser-only Vite mode.
- Normalizes error messages.
- Exposes event listeners for long-running operations.

### React Query

Use TanStack Query for command-backed server state:

- Query keys should include filters.
- Mutations should invalidate affected query keys.
- Write mutations should show the returned backup path in a toast or audit panel.

### Zustand UI Store

Use Zustand for:

- Theme.
- Sidebar mode.
- Entry view mode.
- Filter presets if kept local.
- Writer Mode preferences.
- Draft recovery banner state.

### Routing

MVP routes:

```text
/
/entries
/entries/new
/entries/:entryId/edit
/writer
/threads
/search
/settings
/settings/backups
/about
```

Later routes:

```text
/covers
/analytics
/writing-calendar
/wrapped
/ai
/profile
/gamification
/plugins/dreams
/plugins/coding-ideas
/plugins/post-ideas
/plugins/writing-ideas
/debug
```

## 15. UX Specification

### App Shell

Desktop shell:

- Top bar with app name, database status, quick search, and quick new-entry action.
- Left sidebar navigation.
- Main workspace.
- Optional right detail rail for selected entry or status details.
- Command palette later.

Required sidebar items for MVP:

- Dashboard
- Entries
- Threads
- Search
- Backups
- Settings
- About

Later sidebar items:

- AI
- Cover Wall
- Analytics
- Writing Calendar
- Wrapped
- Gamification
- Profile
- Plugins
- Debug

### Startup Flow

On first launch:

1. Resolve the database path.
2. Show database status.
3. Show entry count if readable.
4. Show last backup if available.
5. Warn if no recent backup exists.
6. Continue into Dashboard.

If the database is unreadable:

- Show exact path.
- Show error.
- Offer "Choose database".
- Offer "Open folder".
- Do not create a new database silently.

### Dashboard MVP

Dashboard should include:

- Database status card.
- Backup status card.
- Recent entries.
- Pinned entries.
- Random entry.
- Basic stats:
  - total entries
  - total tags
  - current year entries
  - current month entries
  - longest current streak if fast enough
- Quick actions:
  - New entry
  - Open Writer Mode
  - Create backup
  - Open database folder

### Entries MVP

Entries page should include:

- Infinite list or virtualized list.
- Cards view.
- Feed view.
- Filters:
  - text
  - tag
  - mood
  - date range
  - starred
  - pinned
  - hidden/include hidden
  - has images later
- Sort:
  - newest first
  - oldest first
- Actions per entry:
  - star/unstar
  - pin/unpin
  - edit
  - continue
  - hide/unhide
  - delete with confirmation
- Grouping by date.
- Visible backup confirmation after successful mutations.

### Composer MVP

The full-page composer should preserve the web app's strongest writing ideas:

- Large writing canvas.
- Markdown content stored as `content_format = markdown`.
- Optional title.
- Optional summary.
- Generate title/summary button using the currently selected cloud AI provider.
- Mood picker.
- Tag input with suggestions.
- Continuation picker.
- Template insertion if template reading is implemented.
- Draft recovery in localStorage.
- Save shortcut.
- Unsaved navigation guard.
- Writing stats:
  - words
  - characters
  - reading time
- Optional writing session timer.

AI-generated title/summary behavior:

- The button should be available in New Entry and Edit Entry when there is enough text to analyze.
- The request must use the configured provider/model unless the user picks a different provider for the action.
- The app should show the provider/model before sending the request.
- The suggestion fills a preview state first; the user must accept, edit, or discard it.
- No entry database write happens merely because a suggestion was generated.
- On save, accepted title/summary values follow the normal backup-guarded entry create/edit flow.
- Suggested titles are plain text, trimmed, and capped at 120 characters.
- Suggested summaries are plain text, trimmed, and capped at 320 characters.
- Markdown entries should be converted to plain text before sending context to the provider.

### Writer Mode MVP

Writer Mode should be a first-class route or overlay:

- No sidebar.
- No dashboard chrome.
- Wide centered editor.
- Save, settings, and exit controls visible on hover/focus.
- Keyboard shortcuts:
  - `Ctrl/Cmd+S`: save
  - `Esc`: exit
  - `Ctrl/Cmd+,`: settings
  - `Ctrl/Cmd+Shift+.`: toggle Writer Mode from composer surfaces
- Configurable:
  - background color
  - text color
  - font family
  - font size
  - line spacing

### Search MVP

Search should support:

- Keyword search.
- Date filters.
- Tag include/exclude.
- Mood include/exclude.
- Structured tokens.
- Results as entry cards.
- Star/pin/edit/continue/delete actions.
- Export current result set later.

Semantic and hybrid modes should remain hidden or disabled until a non-Ollama strategy is chosen. The first AI search-like experience should be scoped cloud chat over explicit context, not background vector indexing.

### Threads MVP

Threads should support:

- Thread groups with title, summary, latest activity, and entry count.
- Entries shown in continuation order.
- Continue from any thread entry.
- Edit title and summary.
- Detach leaf entries.
- Disband a thread with confirmation.
- Bulk-link can come after basic thread read/update.

### Backups MVP

Backups page should support:

- List backups.
- Create backup now.
- Show backup size and timestamp.
- Open backup folder.
- Restore preview.
- Restore with safety backup.
- Clear warning that restore changes the live database.

### Settings MVP

Settings should support:

- Database path display.
- Choose database path, if implemented.
- Theme.
- Sidebar default.
- Backup directory display.
- Backup count display.
- Security status display.
- App version.
- Links to old Capsule README/CLI docs if helpful.

## 16. Data Mutation Semantics

### Entry Creation

Native create flow:

1. Backup.
2. Parse `when`, defaulting to local now.
3. Normalize title and summary.
4. Normalize tags.
5. Generate unique `entry_XXXXXXXX` UUID.
6. Insert into `entries`.
7. Insert missing tags.
8. Insert `entry_tags`.
9. If continuation parent is set, insert `entry_continuations`.
10. Update `entries_fts` if table exists and text search is enabled.
11. Insert history row only if matching current Capsule semantics requires it.
12. Commit.
13. Return created entry and backup path.

### Entry Editing

Native edit flow:

1. Backup.
2. Resolve identifier to entry.
3. Capture old snapshot.
4. Update text/title/summary/mood/tags/starred/pinned/hidden/continuation as requested.
5. Maintain `text_plain`.
6. Update FTS.
7. Insert history row compatible with undo/redo.
8. Commit.
9. Return updated entry and backup path.

### Deletion

Current Capsule delete behavior resequences numeric IDs. The new app must choose one of these approaches before implementation:

- Compatibility mode: match the existing CLI resequencing behavior exactly.
- Safer mode: soft-delete/hide by default and reserve hard delete for advanced settings.

MVP recommendation:

- Default UI action should be hide.
- Hard delete should exist only behind a confirmation dialog.
- If hard delete is implemented, match existing CLI behavior or call the Python core until the Rust implementation is verified.

## 17. Security And Encryption

The current Capsule supports:

- Plain SQLite.
- AES application-level encrypted text fields.
- SQLCipher database encryption.

MVP requirements:

- Detect security config.
- If plain mode: direct native SQLite access is allowed.
- If AES mode and locked: show locked state and do not read text fields.
- If AES mode and unlocked: either implement compatible decrypt/encrypt in Rust or route secure operations through the Python core.
- If SQLCipher mode: do not open with normal SQLite; show unsupported/locked state unless SQLCipher support is implemented.
- Show degraded feature messages for text search and semantic search when encryption policy disables them.

Never:

- Display encrypted ciphertext as entry text.
- Write plaintext into an encrypted database.
- Disable encryption settings from Tauri without a backup and explicit confirmation.

## 18. Images And Attachments

Existing image data uses:

- `plugin_media_assets`
- `plugin_entry_media`
- media files under the active database parent unless configured otherwise
- thumbnails and image routes in the current web backend

MVP:

- Show attachment count on entries.

Phase 2:

- Render thumbnails.
- Open full image viewer.
- Upload/attach images.
- Remove attachments.

Implementation options:

- Command returns data URLs for thumbnails/full images.
- Strict Tauri custom protocol serves only whitelisted media roots.

Security rules:

- No arbitrary path reads from frontend.
- Validate media asset IDs against database before serving files.
- Normalize and canonicalize file paths before opening.

## 19. Location And Weather

MVP:

- Display existing location metadata if inexpensive.
- Show location filter only after `plugin_entry_locations` availability is confirmed.

Later:

- Attach location manually.
- Refresh weather.
- Auto-capture location if configured.
- Weather analytics.

## 20. AI Features

AI should be a first-class planned module for the Tauri app, not only a placeholder. It remains optional and capability-gated because it can send private journal content to a cloud provider.

### Supported Providers

Initial provider scope:

- OpenAI
- Google Gemini
- OpenRouter

Out of initial scope:

- Local Ollama.
- Background local embedding/indexing.
- Automatic cloud processing of entries without a direct user action.

Provider behavior:

- All three providers must expose the same internal generation interface.
- Streaming chat should be supported for providers that offer streaming.
- Non-streaming generation is sufficient for title/summary suggestions and analysis jobs.
- Provider-specific errors should be mapped to user-friendly messages such as missing API key, unsupported model, rate limit, timeout, provider unavailable, and malformed provider response.
- Every AI response shown in the UI should identify the provider and model used.

### Privacy And Consent

Rules:

- Do not send journal content to any provider until the user explicitly enables a provider and invokes an AI action.
- The first cloud AI use should show a concise privacy confirmation explaining that selected journal context will be sent to the chosen provider.
- Hidden entries are excluded from AI context by default.
- Include hidden entries only when the user explicitly enables that option for the action.
- Show a context preview before chat requests that include more than one entry.
- Allow the user to remove individual entries from a context preview before sending.
- Do not include image files by default. Image attachment metadata may be shown in context only if useful; image upload/vision can be a separate future feature.
- Do not store API keys in SQLite, sync payloads, exports, or backups.

### AI Chat Interface

The `/ai` route should be a two-pane workspace:

- Left pane:
  - saved conversations
  - provider/model badge
  - last updated time
  - message count
  - delete conversation action
  - New chat action
- Main pane:
  - message transcript
  - streaming assistant response
  - stop/cancel button during streaming
  - retry action after errors
  - provider/model selector
  - context controls
  - context preview
  - message composer

Chat context scopes:

- `search`: resolve context from filters and the user's message.
- `entry`: include exactly one selected entry.
- `entries`: include a user-selected ordered list of entries.
- `thread`: include the visible continuation thread that contains the selected anchor entry.

Context filters:

- text query
- date range
- include tags
- exclude tags
- include moods
- exclude moods
- starred
- pinned
- include hidden
- has images
- context limit
- newest/oldest/relevance sort

Useful launch points:

- Start chat from the AI page.
- Ask about the current Search results.
- Ask about the current Entry.
- Ask about selected Entries.
- Ask about a Thread from the Threads page.

Prompt construction should include:

- entry number and UUID
- created date/time
- title, if present
- summary, if present
- mood, if present
- tags, if present
- thread position/root when relevant
- entry text/plain text

The prompt should not include unrelated entries merely because they match old fuzzy search behavior. The context preview is the contract: only previewed entries are sent.

### Persistent Conversations

Reuse the existing Capsule database concepts:

- `ai_conversations`
- `ai_conversation_messages`
- `sync_ai_conversation_tombstones`
- `capsule_ai_chats_sync.json`

Conversation rows should store:

- stable conversation UUID
- cloud provider
- model used for the latest message or conversation default
- scope
- canonical scope identifiers using UUIDs where possible
- context limit
- date bounds
- generated title
- generated preview
- created/updated/last-message timestamps

Message rows should store:

- stable message UUID
- role: `user` or `assistant`
- content
- status: `streaming`, `complete`, `interrupted`, or `error`
- sort key
- created/updated timestamps

If a stream is interrupted by navigation, app close, cancel, or provider error, preserve the partial assistant message and mark it `interrupted` or `error`. Loading a conversation should convert stale `streaming` messages to `interrupted`.

### Entry Title/Summary Suggestions

The composer and edit page should include a Generate title/summary action for posts.

Behavior:

- Use the selected cloud provider.
- Send only the current entry draft text and content format.
- Ask for JSON with `title` and `summary`.
- Parse strict JSON, including JSON fenced blocks if needed.
- Reject non-string field values except `null`.
- Normalize and cap the title at 120 characters.
- Normalize and cap the summary at 320 characters.
- Insert into editable fields only after user acceptance.
- Save through the normal backup-guarded entry create/edit mutation.

### Thread Metadata Suggestions

The Threads page should preserve the old app's useful extra:

- Generate thread title/summary from the full thread, oldest post to newest post.
- Include hidden entries only if the user has permission and explicitly includes them.
- Return title, summary, entry count, provider, and model.
- Save through the normal backup-guarded `update_thread_metadata` command only after user acceptance.

### AI Analysis Follow-On

After chat and metadata suggestions, preserve these old Capsule workflows behind the same provider layer:

- tag suggestion
- mood suggestion
- date/tag-filtered journal summaries
- pattern detection
- sentiment journey narration
- AI Time Capsule letters
- related-entry suggestions using cloud ranking over an explicit context set

These features should share the same privacy, provider/model display, context preview, and explicit-invocation rules.

## 21. Sync And Import/Export

Later-phase parity modules:

- Manual backup and restore.
- JSON/CSV/Markdown/PDF/DOCX export.
- Markdown vault export.
- Shared-folder sync.
- GitHub Gist mobile import.
- Flutter import.
- Notion sync.

For sync/import/export, prefer calling existing Python core initially because those paths encode many compatibility rules.

Every import/sync/restore must create a fresh database backup first.

## 22. Plugin Strategy

Current plugins:

- `dream_log`
- `coding_ideas`
- `writing_ideas`
- `post_ideas`

MVP:

- Read plugin enabled state and show plugin nav only when a module is implemented.

Later:

- Native screens for coding ideas, writing ideas, post ideas, and dream log.
- Plugin activation/deactivation/update UI.

Plugin management can initially use the Python bridge because the current plugin store/runtime is Python-oriented.

## 23. Gamification Strategy

MVP:

- Do not block entry creation on gamification.
- If gamification tables exist, optionally show a small dashboard read model.

Later:

- Award XP on entry create.
- Quest claiming.
- Profile hero selection.
- Badge display.

If XP awarding is implemented in Rust, it must match Python `GamificationService` behavior and be covered by tests.

## 24. Performance Targets

Targets:

- Warm startup to app shell: under 2 seconds.
- Database status query: under 250 ms.
- First page of entries: under 300 ms.
- Entry list filter change: under 250 ms for common indexed filters.
- Keyword search: under 500 ms for normal searches.
- Entry create/edit after backup: backup time plus under 300 ms write work.
- UI must remain responsive during backup/restore/import/export.
- Long lists should use virtualization or incremental loading.

If backup creation dominates mutation time, the UI should say so clearly.

## 25. Testing Strategy

### Rust Unit Tests

Use in-memory SQLite for:

- Schema detection.
- Entry row mapping.
- Tag normalization.
- Entry create.
- Entry update.
- FTS rebuild/update.
- Thread cycle prevention.
- Backup naming logic.
- Backup guard aborting writes on backup failure.

Use temp-file SQLite for:

- Backup creation.
- Backup verification.
- Restore preview.
- WAL backup safety.

### Integration Tests

Create a fixture database containing:

- Entries.
- Tags.
- Moods.
- Hidden/starred/pinned rows.
- Threaded entries.
- Image attachment rows.
- Location rows.
- Templates/prompts.

Test:

- List entries.
- Search entries.
- Create/edit/hide.
- Thread grouping.
- Backup creation before mutation.

### Frontend Tests

At minimum:

- TypeScript build.
- Component smoke tests if a test runner is added.
- Manual Playwright verification for:
  - startup status
  - entries list
  - composer
  - Writer Mode
  - search
  - backups

### Manual Safety Test

Before pointing the app at the real `C:\Users\jtill\.capsule\capsule.db`, run all mutation tests against a copied fixture database.

First real-database run should be read-only.

First real-database write should be a harmless test entry only after a backup is created and verified.

## 26. Development Phases

### Phase 0: Scaffold And Safety Baseline

- Create Tauri 2 + React + TypeScript + Vite project.
- Add README and CHANGELOG.
- Add Tauri config.
- Add Rust module skeleton.
- Add frontend `backend.ts` with mock runtime.
- Add database resolver.
- Add read-only database status.
- Add backup listing.
- Add create-backup command.
- Add tests for backup path generation and database status.

Exit criteria:

- `npm run build` passes.
- `cargo test` passes.
- `npm run tauri:dev` opens a desktop window.
- Real database status displays without writing to the database.

### Phase 1: Read-Only Journal

- Dashboard database status.
- Entry count.
- Recent entries.
- Pinned entries.
- Random entry.
- Entries list.
- Basic filters.
- Entry detail view.
- Tag and mood display.
- Thread metadata display if available.

Exit criteria:

- App can browse the current database without modifying it.
- Empty/missing/locked DB states are clear.
- No write commands are reachable except manual backup.

### Phase 2: Write-Safe Core Journaling

- Backup guard.
- Create entry.
- Edit entry.
- Star/unstar.
- Pin/unpin.
- Hide/unhide.
- Hard delete only if compatibility is solved.
- Draft recovery.
- Full-page composer.
- Writer Mode.
- Entry history view.

Exit criteria:

- Every mutation returns a backup path.
- Mutation fails if backup fails.
- Old Capsule CLI/web can still read entries written by Tauri.

### Phase 3: Search And Threads

- Keyword search.
- Structured query tokens.
- Advanced filters.
- Search result actions.
- Thread groups.
- Continue from entry.
- Thread title/summary update.
- Detach/disband with confirmation.

Exit criteria:

- Search and thread flows match common web app behavior.
- Thread operations prevent cycles and preserve UUID identity.

### Phase 4: Backups, Settings, And Data Tools

- Backup restore preview.
- Restore with safety backup.
- Config display.
- Theme/sidebar settings.
- Mood/tag management.
- Template/prompt library basic management.
- Export current entry/search result set as Markdown/JSON.

Exit criteria:

- Restore is tested against fixture databases.
- Backup page is suitable for real use.

### Phase 5: Images, Location, Analytics

- Thumbnail/full image viewing.
- Image upload/attach/remove.
- Location display/filtering.
- Analytics dashboard subset.
- Writing Calendar.
- Cover Wall.

Exit criteria:

- Existing web media remains compatible.
- Image serving is path-safe.

### Phase 6: AI, Sync, Plugins, Gamification

- Cloud AI provider settings for OpenAI, Google Gemini, and OpenRouter.
- Direct provider readiness checks and model selection.
- Entry title/summary suggestions in New Entry and Edit Entry.
- Thread title/summary suggestions in Threads.
- Scoped AI chat with context preview, filters, persisted conversations, streaming, cancel, retry, and provider/model display.
- AI analysis follow-ons: summaries, patterns, sentiment journey, Time Capsules, tag/mood suggestions, and related-entry suggestions.
- Shared-folder sync.
- GitHub Gist import.
- Plugin screens.
- Gamification XP/quests/profile.

Exit criteria:

- Advanced features are capability-gated.
- No cloud request happens without explicit user action/configuration.
- AI provider calls use direct Rust/Tauri integration unless a temporary compatibility shim is explicitly documented.
- Persisted AI chats remain compatible with old Capsule sync sidecars.

## 27. Build And Run Commands

Planned commands:

```powershell
npm install
npm run dev
npm run build
npm run tauri:dev
npm run tauri:build
```

Recommended Vite settings:

- Host: `127.0.0.1`
- Port: `1430` or another port not used by Music Backup V5.
- `strictPort: true`
- Ignore:
  - `src-tauri/**`
  - `dist/**`
  - local database/backups/media folders if ever symlinked into the repo

## 28. Tauri Configuration

Recommended app window:

- Title: `Capsule`
- Width: `1440`
- Height: `960`
- Minimum width: `1040`
- Minimum height: `720`

Product:

- Product name: `Capsule`
- Identifier: `com.local.capsule`

Security:

- Keep Tauri capabilities minimal.
- Do not grant broad filesystem reads to the frontend.
- All filesystem access goes through Rust commands.
- Use CSP deliberately; do not leave broad permissions without cause.

## 29. Error Handling

Errors should be typed and user-friendly.

Important error classes:

- `DatabaseMissing`
- `DatabaseUnreadable`
- `DatabaseLocked`
- `UnsupportedEncryption`
- `BackupFailed`
- `MutationFailed`
- `SchemaUnsupported`
- `EntryNotFound`
- `ThreadCycleRejected`
- `PythonBridgeUnavailable`
- `FeatureNotImplemented`

Frontend should display:

- Short human message.
- Technical detail in expandable area.
- Recommended action.
- Database path when relevant.
- Backup path when relevant.

## 30. Acceptance Criteria For MVP

The MVP is ready when:

- The app launches as a Tauri desktop app.
- It opens `C:\Users\jtill\.capsule\capsule.db`.
- It displays database status and recent entries.
- It can create a manual backup.
- It can list and filter entries.
- It can create and edit entries.
- Every write creates a verified backup first.
- Writer Mode works.
- Keyword search works.
- Existing Capsule CLI/web can read entries created or edited by Tauri.
- `npm run build` passes.
- Rust tests for backup guard and core database operations pass.
- README and CHANGELOG are updated for implemented behavior.

## 31. Open Decisions

These should be resolved before implementation reaches write support:

- Should hard delete match the current CLI resequencing behavior, or should the Tauri UI default to hide/soft-delete?
- Should MVP support only plain SQLite, with encrypted databases displayed as locked/unsupported?
- Which Tauri-compatible secret storage should hold OpenAI, Gemini, and OpenRouter API keys?
- Should AI chat be enabled before AES/SQLCipher write support, or should encrypted databases show AI as unavailable until compatible decryption is native?
- Should cloud AI context previews support full manual entry picking in the first AI slice, or only current search/entry/thread launch points?
- Should the first cloud AI implementation use direct Rust HTTP clients for all three providers, or temporarily wrap the old provider modules while the Rust clients are built?
- Should app settings live only in localStorage/Tauri app data, or should a `capsule_tauri_settings` table eventually be added?
- Should backup rotation be disabled until after several real-database writes are validated?
- Should the first implementation use React 19 to match Capsule Web or React 18 to match Music Backup V5?

## 32. Recommended First Implementation Slice

Start with the smallest slice that proves the new architecture:

1. Scaffold Tauri 2 + React + TypeScript + Vite.
2. Add database status command.
3. Add backup creation command.
4. Add read-only entry list command.
5. Build Dashboard and Entries with mock fallback.
6. Verify against a copied database.
7. Verify against the real database in read-only mode.
8. Only then implement `create_entry` behind the backup guard.

This keeps the first build clean, useful, and safe.
