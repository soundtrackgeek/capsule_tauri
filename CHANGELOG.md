# Changelog

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
