# Capsule Tauri

Capsule Tauri is a local-first desktop shell for Capsule built with Tauri 2,
React, TypeScript, Vite, Rust, and SQLite.

Phase 0 establishes the scaffold and safety baseline:

- Tauri 2 desktop configuration.
- React + TypeScript + Vite frontend.
- Browser-only mock backend for `npm run dev`.
- Read-only database status for the active Capsule database.
- Backup listing for Capsule-compatible backup files.
- Manual SQLite backup creation using SQLite's backup API.
- JSON manifests written next to generated backups.
- Rust tests for backup naming and database status inspection.

The default database resolution checks `CAPSULE_DB_PATH`, then
`CAPSULE_HOME\capsule.db`, then `%USERPROFILE%\.capsule\capsule.db`.

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

The Phase 0 app does not expose journal write operations. It only reads database
status and creates manual backups on explicit user action.

Backups are named with the Capsule-compatible pattern:

```text
capsule_backup_YYYYMMDD_HHMMSS.db
capsule_backup_YYYYMMDD_HHMMSS.json
```

The backup command verifies that the generated database exists, is non-empty,
and can be opened with SQLite before reporting success.
