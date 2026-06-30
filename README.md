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
- Full-page markdown composer with metadata fields, continuation UUID support,
  queued local image attachments, writing stats, and local draft recovery.
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
- Editable Settings paths for the active database file, image media root, and
  backup directory, with native browse buttons.
- Local theme and sidebar-density settings stored outside the journal database.
- Tag rename, merge, and delete tools guarded by verified database backups.
- Mood rename and clear tools guarded by verified database backups.
- Template and prompt library management for custom rows, with built-in rows
  limited to enable/disable actions.
- Markdown and JSON exports for selected entries and current search result sets.
- Image attachment browsing with thumbnail/full-size rendering from the local
  image media root.
- Backup-guarded image upload from local file paths, including native image
  file picking in the Images page and composer, entry attachment, removal, and
  sync tombstone recording.
- Analytics dashboard with overview counts, monthly trend, tag/mood/location
  breakdowns, weather breakdowns, top words, and streaks.
- Writing Calendar heatmap for active days, words, images, and mood metadata.
- Cover Wall view backed by ignored local cover files under `local-assets/covers`
  with generated thumbnails under `local-assets/cover_thumbnails`.
- AI overview for provider/model readiness, persisted conversations, AI Time
  Capsules, embedding models, and local metadata suggestions that do not make
  cloud requests.
- Sync overview for shared-folder status, recent sync history, tombstone counts,
  and GitHub Gist import readiness without running bridge actions implicitly.
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

AI chat, semantic vector ranking, shared-folder sync execution, and GitHub Gist
mobile import remain capability-gated. Tauri reads their existing state, but it
does not send journal data to cloud providers or run bridge-driven workflows
without explicit bridge configuration and user action.
