# Changelog

## 0.14.0 - 2026-07-02

### Added

- Added bundled mood sentiment scoring from legacy Capsule moods for average
  mood analytics, monthly sentiment trend graphs, and writing-calendar mood
  markers.

### Changed

- Bumped the app version to 0.14.0.

## 0.13.0 - 2026-07-02

### Added

- Added chip-style tag entry in the New Entry and Edit Entry composer, with
  autocomplete selection, Tab/Enter pill creation, comma paste handling, and
  removable tag pills.

### Changed

- Bumped the app version to 0.13.0.

## 0.12.0 - 2026-07-02

### Added

- Added mood and tag autocomplete in the New Entry and Edit Entry composer,
  including keyboard selection with arrow keys and Enter.

### Changed

- Bumped the app version to 0.12.0.

## 0.11.0 - 2026-07-02

### Added

- Added native window state restoration so Capsule reopens with the last size,
  position, maximized, or fullscreen state used before closing.

### Changed

- Bumped the app version to 0.11.0.

## 0.10.1 - 2026-07-02

### Changed

- Changed Writer Mode default background, text color, and font to follow the
  selected Interface theme.
- Bumped the app version to 0.10.1.

## 0.10.0 - 2026-07-02

### Added

- Added MS-DOS, Commodore 64, and ZX Spectrum inspired Interface themes.

### Changed

- Bumped the app version to 0.10.0.

## 0.9.4 - 2026-07-02

### Fixed

- Fixed dark theme contrast for the sidebar menu, settings labels, and local
  path metadata text.

### Changed

- Bumped the app version to 0.9.4.

## 0.9.3 - 2026-07-02

### Fixed

- Fixed the Settings Application panel showing a stale hardcoded app version
  after an in-app update.

### Changed

- Bumped the app version to 0.9.3.

## 0.9.2 - 2026-07-02

### Removed

- Removed the New Entry composer `When` metadata field so new entries use the
  actual save time instead of a user-selected timestamp.

### Changed

- Bumped the app version to 0.9.2.

## 0.9.1 - 2026-07-02

### Fixed

- Fixed Windows release builds opening an extra blank console window at app
  startup.

### Changed

- Bumped the app version to 0.9.1.

## 0.9.0 - 2026-07-02

### Added

- Added signed in-app updates with hourly background checks, an update banner,
  install progress, and a manual Check for updates button in Settings.
- Added Tauri updater artifact signing and `latest.json` generation to the
  Windows release workflow so GitHub Releases can serve update metadata.

### Changed

- Bumped the app version to 0.9.0.

## 0.8.1 - 2026-07-02

### Added

- Added a tag-triggered GitHub Actions workflow that builds Windows Tauri
  bundles and attaches the NSIS setup executable and MSI to GitHub Releases.

## 0.8.0 - 2026-07-01

### Added

- Added native Capsule-compatible shared-folder sync for entries, image
  metadata, locations, custom library items, thread sidecars, and AI chats using
  the legacy `capsule_sync.json`, `capsule_threads_sync.json`, and
  `capsule_ai_chats_sync.json` files.
- Added Settings controls for the shared sync folder, manual sync runs, and a
  configurable automatic sync interval.

### Changed

- Changed the Sync page from a read-only status view into a runnable sync
  surface with configuration details, latest-file retry handling, and tombstone
  aware merge status.
- Bumped the app version to 0.8.0.

## 0.7.13 - 2026-06-30

### Fixed

- Fixed slow composer saves with multiple queued images by batching all image
  upload/attach writes behind one backup instead of backing up once per image
  upload and once per image attachment.

### Changed

- Bumped the app version to 0.7.13.

## 0.7.12 - 2026-06-30

### Added

- Added queued image thumbnail previews in the composer so selected local images
  can be checked before saving the entry.

### Changed

- Bumped the app version to 0.7.12.

## 0.7.11 - 2026-06-30

### Changed

- Changed the composer Add Image action to open a native multi-select image
  picker immediately and append all selected images to the queued attachments.
- Bumped the app version to 0.7.11.

## 0.7.10 - 2026-06-30

### Added

- Added native image file picking and queued image attachments to the new-entry
  and edit-entry composer flows.
- Added edit-composer attached image browsing/removal while preserving
  Capsule-compatible `plugin_media_assets` and `plugin_entry_media` storage.

### Changed

- Bumped the app version to 0.7.10.

## 0.7.9 - 2026-06-30

### Changed

- Unregistered the plugin enable/disable IPC command and removed the native
  toggle helper while keeping legacy plugin-prefixed media and location tables
  available for existing app data.
- Bumped the app version to 0.7.9.

### Removed

- Removed the Plugins navigation item, Plugin Registry view, and browser mock
  plugin toggle helpers from the UI.

## 0.7.8 - 2026-06-30

### Added

- Added backup-guarded entry deletion with an explicit warning dialog, sync
  tombstone recording, relation cleanup, legacy ID resequencing, and FTS
  rebuilds.

### Changed

- Bumped the app version to 0.7.8.

## 0.7.7 - 2026-06-30

### Added

- Added Settings controls for entry location capture, including a fixed default
  place that new entries use for weather lookups.

### Changed

- Bumped the app version to 0.7.7.

## 0.7.6 - 2026-06-30

### Added

- Added editable Settings controls for the active database file, image media
  root, and backup directory.
- Added native browse dialogs for selecting the database file and image/backup
  folders.
- Added local path settings persistence for database, image, and backup paths.

### Changed

- Changed backup resolution to allow a saved local backup directory or
  `CAPSULE_BACKUP_DIR` override instead of always using the database directory.
- Bumped the app version to 0.7.6.

## 0.7.5 - 2026-06-30

### Added

- Added a Settings path overview for the active database file, resolved image
  media root, and active backup directory.

### Changed

- Bumped the app version to 0.7.5.

## 0.7.4 - 2026-06-29

### Changed

- Changed weather temperature displays to prefer Celsius, falling back to
  Fahrenheit only when Celsius is unavailable.
- Bumped the app version to 0.7.4.

## 0.7.3 - 2026-06-29

### Changed

- Bumped the app version to 0.7.3.
- Expanded entry location responses and UI details to include stored Capsule
  weather icon, humidity, wind speed, fetched timestamp, and location source.

## 0.7.2 - 2026-06-29

### Added

- Added Capsule-compatible location and weather auto-capture for newly created
  Tauri entries, including default-location, IP lookup, Nominatim geocoding,
  Open-Meteo, and MET Norway support.

### Fixed

- Fixed new entries returning location rows without weather by attaching weather
  metadata during the native create-entry flow.

### Changed

- Bumped the app version to 0.7.2.

## 0.7.1 - 2026-06-29

### Fixed

- Fixed Entries, Search, and Cover Wall filter controls so they stay inside the
  filter rail instead of overlapping adjacent result panels.

### Changed

- Bumped the app version to 0.7.1.

## 0.7.0 - 2026-06-29

### Added

- Added Phase 6 backend read models for AI, sync, plugins, and gamification.
- Added local AI metadata suggestions that do not make cloud provider requests.
- Added AI overview for persisted conversations, AI Time Capsules, embedding
  models, and bridge/provider capability status.
- Added sync overview for shared-folder status, recent sync history, tombstone
  counts, and GitHub Gist import readiness.
- Added Plugins view with implemented plugin module counts and backup-guarded
  enable/disable state changes.
- Added Gamification/Profile view with XP totals, derived level, quests, badges,
  recent XP events, and backup-guarded quest claiming.
- Added Rust tests for Phase 6 read models, plugin toggles, and quest claiming.

### Changed

- Bumped the app version to 0.7.0.
- Updated the app shell from Phase 5 images/location/analytics/covers to Phase 6
  AI, sync, plugins, and gamification.

## 0.6.0 - 2026-06-29

### Added

- Added image attachment commands for listing, data-url rendering, upload,
  attach, removal, and sync tombstone recording.
- Added Images view with entry image browsing, thumbnail/full-size previews, and
  guarded local-path attachment flow.
- Added location filters to Entries and Search.
- Added analytics commands and dashboard for counts, streaks, monthly trend,
  tag/mood/location/weather breakdowns, and top words.
- Added Writing Calendar command and heatmap view.
- Added Cover Wall command and UI backed by ignored local cover assets.
- Added Rust tests for image mutations, analytics, calendar aggregation, and
  cover indexing.

### Changed

- Bumped the app version to 0.6.0.
- Updated the app shell from Phase 4 backup/data tools to Phase 5 images,
  location, analytics, and cover browsing.
- Allowed `data:` images in the Tauri content security policy for local
  thumbnails and full-size previews.

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
