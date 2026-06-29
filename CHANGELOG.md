# Changelog

## 0.5.0 - 2026-06-29

### Added

- Added backup restore preview and restore commands, with restore constrained to
  Capsule backup files in the active backup directory.
- Added safety-backup creation before restore replaces the live database.
- Added an open-backup-folder command and restore controls to the Backups view.
- Added Capsule config display plus file-backed config set/delete actions with
  config backups.
- Added local theme and sidebar-density settings.
- Added backup-guarded tag rename, merge, and delete tools.
- Added backup-guarded mood rename and clear tools.
- Added template and prompt library management for custom rows, with built-in
  rows limited to enable/disable actions.
- Added Markdown and JSON export for selected entries and search result sets.
- Added Rust tests for restore, tag/mood tools, library CRUD, and export
  generation.

### Changed

- Bumped the app version to 0.5.0.
- Updated the app shell from Phase 3 search and threads to Phase 4 backups,
  settings, and data tools.

## 0.4.0 - 2026-06-29

### Added

- Added native keyword search with structured query token parsing for tags,
  moods, date bounds, and `NOT tag:` filters.
- Added FTS-backed search when `entries_fts` is available, with a compatibility
  fallback to entry text filtering.
- Added Search view result cards with shared entry detail and star, pin, edit,
  continue, hide, and unhide actions.
- Added native thread group listing from continuation links with ordered entries,
  latest activity, titles, and summaries.
- Added backup-guarded thread title/summary updates, bulk link commands, leaf
  detach, and disband operations with cycle prevention.
- Added Threads view for inspecting continuation chains and managing thread
  metadata.
- Added Rust tests for structured search, FTS fallback, thread grouping,
  metadata updates, cycle rejection, detach, and disband behavior.

### Changed

- Bumped the app version to 0.4.0.
- Updated the app shell from Phase 2 write-safe journaling to Phase 3 search and
  thread workflows.

## 0.3.0 - 2026-06-29

### Added

- Added a reusable backup guard for database mutations.
- Added backup-guarded entry creation and editing commands.
- Added backup-guarded star, unstar, pin, unpin, hide, and unhide commands.
- Added entry history retrieval for legacy edit snapshots.
- Added a full-page markdown composer with metadata fields, continuation UUIDs,
  writing stats, and local draft recovery.
- Added Writer Mode with local display preferences.
- Added frontend mutation actions that surface the returned backup path.
- Added Rust tests for backup guard abort behavior, entry creation, entry
  updates, history recording, and hide mutations.

### Changed

- Bumped the app version to 0.3.0.
- Updated the app shell from Phase 1 read-only browsing to Phase 2 write-safe
  journaling.

## 0.2.0 - 2026-06-29

### Added

- Added read-only entry list, entry detail, and random entry Tauri commands.
- Added read-only entry filtering by text, tag, mood, date range, hidden state,
  image attachment presence, and sort order.
- Added tag, mood, location, attachment count, and thread metadata enrichment for
  entry browsing.
- Added Phase 1 dashboard panels for recent entries, pinned entries, random
  entry, and current period entry counts.
- Added an Entries view with filters, loading states, empty states, and a
  read-only detail rail.
- Added Rust tests for read-only entry listing, filtering, relation enrichment,
  entry lookup, and random entry queries.

### Changed

- Updated the app shell from the Phase 0 safety baseline to the Phase 1
  read-only journal experience.

## 0.1.1 - 2026-06-29

### Fixed

- Fixed Phase 0 database resolution so the MVP production database is preferred
  over an older `CAPSULE_HOME` fallback database.

## 0.1.0 - 2026-06-29

### Added

- Scaffolded the Tauri 2, React, TypeScript, and Vite desktop app.
- Added read-only Capsule database status inspection.
- Added Capsule-compatible backup listing and manual backup creation.
- Added JSON backup manifests and backup verification.
- Added frontend mock runtime for browser-only Vite development.
- Added Rust tests for backup path generation and database status inspection.
