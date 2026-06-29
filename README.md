# Capsule Tauri

Capsule Tauri is a local-first desktop journal for Capsule built with Tauri 2,
React, TypeScript, Vite, Rust, and SQLite.

Phase 1 provides a read-only journal surface over the active Capsule database:

- Tauri 2 desktop configuration.
- React + TypeScript + Vite frontend.
- Browser-only mock backend for `npm run dev`.
- Read-only database status for the active Capsule database.
- Dashboard counts for total entries, total tags, current year, and current month.
- Recent entries, pinned entries, and random entry panels.
- Entries list with text, tag, mood, date, image, hidden, and sort filters.
- Entry detail view with full text, tags, mood, location, attachment count, and
  thread metadata when those tables are available.
- Backup listing for Capsule-compatible backup files.
- Manual SQLite backup creation using SQLite's backup API.
- JSON manifests written next to generated backups.
- Rust tests for backup naming, database status inspection, and read-only entry
  queries.

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

The Phase 1 app does not expose journal write operations. It reads database
status and journal entries, and it creates manual backups only on explicit user
action.

Backups are named with the Capsule-compatible pattern:

```text
capsule_backup_YYYYMMDD_HHMMSS.db
capsule_backup_YYYYMMDD_HHMMSS.json
```

The backup command verifies that the generated database exists, is non-empty,
and can be opened with SQLite before reporting success.
