# Capsule Tauri

Capsule Tauri is a local-first desktop journal for Capsule built with Tauri 2,
React, TypeScript, Vite, Rust, and SQLite.

Phase 4 provides backup restore, settings, and data tools over the active
Capsule database:

- Tauri 2 desktop configuration.
- React + TypeScript + Vite frontend.
- Browser-only mock backend for `npm run dev`.
- Read-only database status for the active Capsule database.
- Dashboard counts for total entries, total tags, current year, and current month.
- Recent entries, pinned entries, and random entry panels.
- Entries list with text, tag, mood, date, image, hidden, and sort filters.
- Entry detail view with full text, tags, mood, location, attachment count, and
  thread metadata when those tables are available.
- Backup-guarded entry creation and editing.
- Backup-guarded star, unstar, pin, unpin, hide, and unhide entry actions.
- Full-page markdown composer with metadata fields, continuation UUID support,
  writing stats, and local draft recovery.
- Distraction-free Writer Mode with local display preferences.
- Entry history review for legacy Capsule edit snapshots.
- Keyword search using `entries_fts` when available, with a compatibility
  fallback to entry text matching.
- Structured search tokens for `tag:`, `mood:`, `before:`, `after:`, and
  `NOT tag:` queries, plus include/exclude tag and mood filters.
- Search results with the same star, pin, edit, continue, and hide/unhide
  actions used by the Entries view.
- Thread groups built from Capsule continuation links with ordered entries,
  latest activity, titles, and summaries.
- Backup-guarded thread title/summary updates, bulk link commands, leaf detach,
  and thread disband actions with cycle prevention.
- Backup listing for Capsule-compatible backup files.
- Manual SQLite backup creation using SQLite's backup API.
- Backup restore preview with schema, entry, tag, size, and timestamp checks.
- Restore with a fresh safety backup before the live database is replaced.
- Open-backup-folder command for the active database directory.
- JSON manifests written next to generated backups.
- Capsule `config.json` display plus file-backed set/delete actions that create
  config backups before writing.
- Local theme and sidebar-density settings stored outside the journal database.
- Tag rename, merge, and delete tools guarded by verified database backups.
- Mood rename and clear tools guarded by verified database backups.
- Template and prompt library management for custom rows, with built-in rows
  limited to enable/disable actions.
- Markdown and JSON exports for selected entries and current search result sets.
- Rust tests for backup naming, database status inspection, read-only entry
  queries, backup-guarded mutations, entry history, search, thread operations,
  restore, tag/mood tools, library CRUD, and export generation.

The database resolver checks an explicit `CAPSULE_DB_PATH` first. When that is
not set, it prefers the MVP production database at
`C:\Users\jtill\.capsule\capsule.db`, then falls back to
`%USERPROFILE%\.capsule\capsule.db` and finally `CAPSULE_HOME\capsule.db`.

## Commands

```powershell
npm install
npm run dev
npm run build
npm run tauri:dev
npm run tauri:build
```

Rust tests can be run from the Tauri crate:

```powershell
cd src-tauri
cargo test
```

## Safety Baseline

Every exposed journal and thread mutation runs through a backup guard before
opening a write transaction. If backup creation or verification fails, the
mutation is not run. Mutation responses include the backup path used for that
operation.

Backups are named with the Capsule-compatible pattern:

```text
capsule_backup_YYYYMMDD_HHMMSS.db
capsule_backup_YYYYMMDD_HHMMSS.json
```

The backup command verifies that the generated database exists, is non-empty,
and can be opened with SQLite before reporting success.

Restore is constrained to Capsule-compatible backup files in the active database
backup directory. Before restore replaces the live database, the app creates and
verifies a fresh safety backup of the current database.

Hard delete is intentionally not exposed in Phase 4. Entries can be hidden and
unhidden safely; true delete remains reserved until the legacy resequencing
behavior is matched and tested.
