# Changelog

## 0.24.2 - 2026-07-05

### Fixed

- Fixed diagnostics `environment.txt` so it distinguishes process environment
  overrides from saved Capsule settings and OS credential store AI keys.

### Changed

- Bumped the app version to 0.24.2.

## 0.24.1 - 2026-07-05

### Fixed

- Fixed diagnostics bundle creation so a missing `debug.log` is represented by
  a placeholder file instead of a warning about an optional file.

### Changed

- Bumped the app version to 0.24.1.

## 0.24.0 - 2026-07-05

### Added

- Added a hidden Debug menu that is off by default and can be enabled from
  Settings, with database health checks, image thumbnail/add-image smoke tests,
  synthetic AI provider testing, debug log notes, and ZIP diagnostics bundle
  creation for support reports.

### Changed

- Bumped the app version to 0.24.0.

## 0.23.2 - 2026-07-05

### Fixed

- Fixed AI Chat New Chat so it stays on a blank conversation instead of
  immediately reselecting the most recent saved chat and appending new messages
  there.

### Changed

- Bumped the app version to 0.23.2.

## 0.23.1 - 2026-07-05

### Added

- Added composer title/summary generation for entries using the selected cloud
  AI provider and model, with first-use privacy confirmation, strict JSON
  parsing, provider/model display, and an apply-before-save review step.

### Changed

- Bumped the app version to 0.23.1.

## 0.23.0 - 2026-07-05

### Added

- Added the AI Chat workspace with saved conversations, context preview/removal,
  provider/model selection, streaming cloud responses, stop/cancel, retry, and
  delete actions.
- Added backend AI chat schema creation/migration, conversation tombstones,
  stale-stream recovery, provider streaming adapters for OpenAI, Gemini, and
  OpenRouter, and safe provider error mapping.
- Added frontend mock streaming, browser smoke coverage, Rust chat/provider
  tests, and a gated synthetic live provider smoke test that sends no journal
  entries.

### Changed

- Updated AI chat sync payloads to round-trip the conversation `model` field
  while remaining compatible with older sync files.
- Bumped the app version to 0.23.0.

## 0.22.3 - 2026-07-05

### Fixed

- Fixed Cloud AI Settings so a missing `cloud_provider` uses the Gemini default
  without showing an invalid-provider warning.

### Changed

- Bumped the app version to 0.22.3.

## 0.22.2 - 2026-07-05

### Fixed

- Fixed Cloud AI Settings so the main save action also stores any entered API
  key fields and clearly marks typed-but-unsaved keys before they are persisted.
- Fixed API key persistence by explicitly initializing the Windows Credential
  Manager store before reading, saving, or clearing Cloud AI keys.

### Changed

- Bumped the app version to 0.22.2.

## 0.22.1 - 2026-07-05

### Added

- Added Cloud AI Settings for Gemini, OpenAI, and OpenRouter provider/model
  selection, chat context defaults, and redacted API key status backed by the OS
  credential store.
- Added updated Gemini and OpenRouter model lists with legacy model ID
  replacement for saved configuration values.

### Changed

- Bumped the app version to 0.22.1.

## 0.22.0 - 2026-07-04

### Added

- Added an inline linked-entry reader to Cover Wall so selecting a cover loads
  the full referenced entry with the existing entry actions and history review.
- Added unit and Playwright coverage for the Cover Wall linked-entry reader.

### Changed

- Bumped the app version to 0.22.0.

## 0.21.8 - 2026-07-04

### Added

- Added a manual sync safety confirmation that summarizes backup protection,
  database and sync paths, GitHub Gist mode, auto-sync state, last result, and
  pending deletion markers before a sync run starts.
- Added Playwright coverage for the sync confirmation flow.

### Changed

- Changed manual Sync page and Settings sync actions to open the review dialog
  before running, while automatic sync continues to run silently.
- Bumped the app version to 0.21.8.

## 0.21.7 - 2026-07-04

### Added

- Added ESLint frontend linting with TypeScript and React hooks checks.
- Added Rust Clippy linting with warnings treated as errors.
- Added frontend and Rust lint gates to GitHub Actions CI.

### Changed

- Fixed existing frontend unused-code findings and Rust Clippy warnings so the
  new lint gates pass cleanly.
- Documented the lint command and expanded the CI docs.
- Bumped the app version to 0.21.7.

## 0.21.6 - 2026-07-04

### Added

- Added React Testing Library coverage for shared UI and entry components.
- Added Playwright E2E coverage for dashboard, entries, search, and composer
  flows against the browser mock backend.
- Added `npm run test:e2e` and wired Playwright into GitHub Actions CI.

### Changed

- Documented the E2E test command and expanded the CI docs.
- Bumped the app version to 0.21.6.

## 0.21.5 - 2026-07-04

### Added

- Added a GitHub Actions CI workflow for pushes and pull requests that runs
  frontend dependency installation, tests, frontend build, Rust formatting, and
  Rust tests.

### Changed

- Documented the CI checks in the README.
- Bumped the app version to 0.21.5.

## 0.21.4 - 2026-07-04

### Changed

- Split shared frontend UI, entry display, media image, analytics, and calendar
  helpers out of `App.tsx` to start decomposing the large React surface without
  changing behavior.
- Added Vitest coverage for the extracted analytics and calendar helpers.
- Bumped the app version to 0.21.4.

## 0.21.3 - 2026-07-03

### Fixed

- Fixed success and information banners lingering across screens by
  automatically dismissing them after five seconds.

### Changed

- Bumped the app version to 0.21.3.

## 0.21.2 - 2026-07-03

### Fixed

- Fixed normal launches reopening only in the system tray after quitting while
  the main window was hidden to the tray.

### Changed

- Bumped the app version to 0.21.2.

## 0.21.1 - 2026-07-03

### Fixed

- Fixed duplicate Capsule instances by adding single-instance startup
  protection that hands secondary launches to the already running app.

### Changed

- Bumped the app version to 0.21.1.

## 0.21.0 - 2026-07-03

### Added

- Added old-Capsule-style entry numbers from `entries.id` on Dashboard,
  Entries, Search, and entry detail views.

### Fixed

- Added conservative entry ID repair so imported legacy rows without visible
  entry IDs receive numbers before new entries are created.

### Changed

- Bumped the app version to 0.21.0.

## 0.20.0 - 2026-07-03

### Added

- Added a Changelog box under the About panel, backed by the bundled
  `CHANGELOG.md` release history.
- Added Vitest coverage for the changelog Markdown parser and an `npm test`
  command.

### Changed

- Bumped the app version to 0.20.0.

## 0.19.2 - 2026-07-03

### Fixed

- Fixed update restarts reopening Capsule only in the system tray by showing the
  main window once the updated app starts.

### Changed

- Bumped the app version to 0.19.2.

## 0.19.1 - 2026-07-03

### Changed

- Changed the app header, About panel, and README introduction to describe
  Capsule as a local-first desktop journal instead of a release phase.
- Bumped the app version to 0.19.1.

## 0.19.0 - 2026-07-03

### Added

- Added a global `Ctrl+Alt+W` shortcut that opens Capsule directly into Writer
  Mode from any Windows app while Capsule is running.

### Changed

- Bumped the app version to 0.19.0.

## 0.18.0 - 2026-07-03

### Added

- Added system tray Writer and Settings actions that open Capsule directly to
  Writer Mode or the Settings page.

### Changed

- Bumped the app version to 0.18.0.

## 0.17.0 - 2026-07-03

### Added

- Added a system tray icon with Open Interface and Quit actions.
- Added a Settings option to hide the app to the tray when the main window is
  closed.

### Changed

- Bumped the app version to 0.17.0.

## 0.16.3 - 2026-07-02

### Fixed

- Fixed Start menu and taskbar icon rendering by adding small-size optimized
  Windows icon layers, matching the runtime AppUserModelID to the installer
  shortcut, and explicitly applying the Capsule window icon at app startup.
- Bumped the app version to 0.16.3.

## 0.16.2 - 2026-07-02

### Fixed

- Fixed Cover Wall thumbnail rendering so generated thumbnails use a writable
  app-local cache and still display when cache writes fail.

### Changed

- Bumped the app version to 0.16.2.

## 0.16.1 - 2026-07-02

### Changed

- Changed the Windows installer and app bundle icon to the Journal Vault app
  icon so installed shortcuts, Start menu entries, and the taskbar use the new
  Capsule branding.
- Bumped the app version to 0.16.1.

## 0.16.0 - 2026-07-02

### Added

- Added a saved Settings path for Cover Wall images, with
  `CAPSULE_COVERS_ROOT` still taking precedence when set.

### Changed

- Bumped the app version to 0.16.0.

## 0.15.0 - 2026-07-02

### Added

- Added per-user GitHub Gist sync settings for a Gist ID and optional token,
  with Sync runs pulling Capsule sync files from the Gist before local merging
  and pushing merged files back when a token is configured.

### Changed

- Changed the Sync page's GitHub Gist capability from bridge-readiness status
  into native pull/push configuration status.
- Bumped the app version to 0.15.0.

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
