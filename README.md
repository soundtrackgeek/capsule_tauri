# Capsule Tauri

Capsule Tauri is a local-first desktop journal for Capsule built with Tauri 2,
React, TypeScript, Vite, Rust, and SQLite.

Phase 6 provides images, location-aware browsing, analytics, a visual cover
wall, and capability-gated AI/sync/gamification surfaces over the active
Capsule database:

- Tauri 2 desktop configuration.
- React + TypeScript + Vite frontend.
- Browser-only mock backend for `npm run dev`.
- Read-only database status for the active Capsule database.
- Dashboard counts for total entries, total tags, current year, and current month.
- Recent entries, pinned entries, and random entry panels.
- Entries list with text, tag, mood, location, date, image, hidden, and sort
  filters.
- Entry detail view with full text, tags, mood, location, attachment count, and
  thread metadata when those tables are available, including stored weather
  condition, Celsius-first temperature, humidity, wind, icon, source, and
  fetched timestamp.
- Backup-guarded entry creation and editing, with Capsule-compatible location
  and weather auto-capture for new entries.
- Backup-guarded star, unstar, pin, unpin, hide, unhide, and confirmed delete
  entry actions.
- Full-page markdown composer with metadata fields, save-time entry dating,
  continuation UUID support, multi-select queued local image attachments with
  thumbnail previews, writing stats, and local draft recovery.
- Distraction-free Writer Mode with local display preferences.
- Entry history review for legacy Capsule edit snapshots.
- Keyword search using `entries_fts` when available, with a compatibility
  fallback to entry text matching.
- Structured search tokens for `tag:`, `mood:`, `before:`, `after:`, and
  `NOT tag:` queries, plus include/exclude tag and mood filters and a location
  text filter.
- Search results with the same star, pin, edit, continue, delete, and hide/unhide
  actions used by the Entries view.
- Thread groups built from Capsule continuation links with ordered entries,
  latest activity, titles, and summaries.
- Backup-guarded thread title/summary updates, bulk link commands, leaf detach,
  and thread disband actions with cycle prevention.
- Backup listing for Capsule-compatible backup files.
- Manual SQLite backup creation using SQLite's backup API.
- Backup restore preview with schema, entry, tag, size, and timestamp checks.
- Restore with a fresh safety backup before the live database is replaced.
- Open-backup-folder command for the active backup directory.
- JSON manifests written next to generated backups.
- Capsule `config.json` display plus file-backed set/delete actions that create
  config backups before writing.
- Entry location Settings controls for choosing IP lookup or a fixed default
  place for new-entry weather capture.
- Editable Settings paths for the active database file, image media root,
  backup directory, and shared sync folder, with native browse buttons.
- Local System, Light, Dark, MS-DOS, Commodore 64, and ZX Spectrum theme
  settings plus sidebar-density settings stored outside the journal database.
- Tag rename, merge, and delete tools guarded by verified database backups.
- Mood rename and clear tools guarded by verified database backups.
- Template and prompt library management for custom rows, with built-in rows
  limited to enable/disable actions.
- Markdown and JSON exports for selected entries and current search result sets.
- Image attachment browsing with thumbnail/full-size rendering from the local
  image media root.
- Backup-guarded image upload from local file paths, including native image
  file picking in the Images page, multi-select composer image picking, entry
  attachment previews, batched composer image save, removal, and sync tombstone
  recording.
- Analytics dashboard with overview counts, monthly trend, tag/mood/location
  breakdowns, weather breakdowns, top words, and streaks.
- Writing Calendar heatmap for active days, words, images, and mood metadata.
- Cover Wall view backed by ignored local cover files under `local-assets/covers`
  with generated thumbnails under `local-assets/cover_thumbnails`.
- AI overview for provider/model readiness, persisted conversations, AI Time
  Capsules, embedding models, and local metadata suggestions that do not make
  cloud requests.
- Native Capsule-compatible shared-folder sync using `capsule_sync.json`,
  `capsule_threads_sync.json`, and `capsule_ai_chats_sync.json`, including entry,
  image metadata, location, custom library, thread, AI chat, and tombstone
  merging.
- Sync controls in Settings for manual runs and configurable automatic sync
  intervals, plus a Sync page with status, history, tombstone counts, and GitHub
  Gist import readiness.
- Signed in-app updates, including an hourly background check, an update banner
  when a new version is available, and a manual Check for updates button in
  Settings. The Settings Application panel reports the installed Tauri runtime
  version so it matches updater decisions.
- Legacy plugin-prefixed media and location tables remain supported for Capsule
  compatibility, while plugin registry navigation and activation toggles are not
  exposed in the UI.
- Gamification profile screen with XP totals, derived level, recent XP events,
  badges, quest progress, and backup-guarded quest claiming.
- Rust tests for backup naming, database status inspection, read-only entry
  queries, backup-guarded mutations, entry history, search, thread operations,
  restore, tag/mood tools, library CRUD, export generation, image operations,
  analytics, calendar aggregation, cover indexing, Phase 6 read models, and
  quest claiming.

The database resolver checks an explicit `CAPSULE_DB_PATH` first. When that is
not set, it uses the saved local database path from Settings, then prefers the
MVP production database at `C:\Users\jtill\.capsule\capsule.db`, then falls
back to `%USERPROFILE%\.capsule\capsule.db` and finally
`CAPSULE_HOME\capsule.db`.

Image storage resolves `CAPSULE_IMAGES_MEDIA_ROOT` first, then
the saved local image path from Settings, then `images.media_root` from Capsule
config, then the default `C:\Users\jtill\OneDrive\_capsule\images`. Backup
storage resolves `CAPSULE_BACKUP_DIR` first, then the saved local backup path
from Settings, then the active database directory. Saved local paths are stored
outside the journal database in the app path settings file shown in Settings.
Shared-folder sync resolves `CAPSULE_SYNC_PATH` first, then the saved sync path
from Settings, and writes the same three sync files used by the older Capsule
app.
Uploaded originals use Capsule's legacy image key layout
`<hash-prefix>/<sha256>.<ext>` and thumbnails use
`thumb/<hash-prefix>/<sha256>.jpg`, with attachment metadata stored in
`plugin_media_assets` and `plugin_entry_media` for old Capsule compatibility.
Cover wall assets are local-only and ignored by Git under `local-assets/`; set
`CAPSULE_COVERS_ROOT` to point at a different cover folder.

Location auto-capture on entry creation uses the same Capsule configuration keys
as the existing app: `location.auto_capture`, `location.auto_capture_method`,
`location.use_default_location`, `location.default_location_name`,
`location.weather_provider`, and `location.geocoding_cache_hours`. Settings can
save a fixed default place by writing `location.use_default_location` and
`location.default_location_name`; new entries then geocode that place and fetch
weather for it instead of using IP lookup. It reads `CAPSULE_CONFIG_PATH` first
when set, otherwise `config.json` next to the active database. The built-in
providers match Capsule's current defaults: IP lookup via `ip-api.com` with
`ipinfo.io` fallback, geocoding via Nominatim, and weather via Open-Meteo or MET
Norway.

## Commands

```powershell
npm install
npm run dev
npm run build
npm run tauri:dev
npm run tauri:build
```

`npm run tauri:build` creates signed updater artifacts and Windows release
executables that launch without an extra console window. In CI the signing key
comes from GitHub Secrets; for a local release build, set
`TAURI_SIGNING_PRIVATE_KEY` to the private key content before running the
command. For a passwordless key, set `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to an
empty string.

## Releases

Windows release bundles are built and attached to GitHub Releases by the
`Release Windows Installer` workflow. After updating the app version and
committing the release changes, push a semantic version tag to publish the
installer assets:

```powershell
git tag vMAJOR.MINOR.PATCH
git push origin vMAJOR.MINOR.PATCH
```

The workflow runs `npm ci` and `npm run tauri:build` on `windows-latest`, then
uploads the generated NSIS setup executable, MSI, updater signatures, and
`latest.json` manifest from `src-tauri/target/release/bundle` to the matching
GitHub Release.

In-app updates use Tauri's signed updater. The app contains only the public
verification key; release builds sign updater artifacts with the
`TAURI_SIGNING_PRIVATE_KEY` GitHub secret, plus the optional
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret if the key is password protected.
Users do not need these keys. Anyone on a build older than the first
updater-enabled release must install that release manually once before future
updates can be installed from inside Capsule.

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

Entry delete is exposed only after an explicit warning dialog. Deleting an entry
creates a verified backup first, records a sync tombstone, removes local
entry-owned relation rows, resequences later numeric IDs, and rebuilds
`entries_fts` so legacy Capsule ID ordering stays compatible.

Shared-folder sync execution is local and backup-guarded. Manual sync runs only
from an explicit Settings or Sync-page action, and automatic sync runs only when
enabled with a saved interval. Sync applies remote delete tombstones before
upserts, keeps newer local rows when they win by timestamp, rewrites the latest
sync files after merging, and retries if the shared file changes during a run.

AI chat live requests, semantic vector ranking, and GitHub Gist mobile import
remain capability-gated. Tauri reads their existing state, but it does not send
journal data to cloud providers or run bridge-driven workflows without explicit
bridge configuration and user action.
