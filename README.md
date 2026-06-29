# Capsule Tauri

Capsule Tauri is a local-first desktop journal for Capsule built with Tauri 2,
React, TypeScript, Vite, Rust, and SQLite.

Phase 2 provides write-safe core journaling over the active Capsule database:

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
- Backup listing for Capsule-compatible backup files.
- Manual SQLite backup creation using SQLite's backup API.
- JSON manifests written next to generated backups.
- Rust tests for backup naming, database status inspection, read-only entry
  queries, backup-guarded mutations, and entry history.

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

Every Phase 2 journal mutation runs through a backup guard before opening a
write transaction. If backup creation or verification fails, the mutation is not
run. Mutation responses include the backup path used for that operation.

Backups are named with the Capsule-compatible pattern:

```text
capsule_backup_YYYYMMDD_HHMMSS.db
capsule_backup_YYYYMMDD_HHMMSS.json
```

The backup command verifies that the generated database exists, is non-empty,
and can be opened with SQLite before reporting success.

Hard delete is intentionally not exposed in Phase 2. Entries can be hidden and
unhidden safely; true delete remains reserved until the legacy resequencing
behavior is matched and tested.
