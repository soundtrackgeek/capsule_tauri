# capsule-core

`capsule-core` is the headless shared behavior used by Capsule desktop and
non-desktop clients. It owns the SQLite database, backup, entry, location, and
model modules without a Tauri, WebView, tray, window, or shell dependency.

Use `ResolveRequest` with an explicit `ResolverEnvironment` snapshot when a
client needs deterministic path/config resolution. `entries::*_read_only_*`
queries inspect supported schema and refuse layouts that would require ID
repair; the repair-capable desktop convenience wrappers remain available for
legacy behavior. `CaptureRequest`, `CommitReceipt`, and context DTOs are
renderer-independent seams for capture/context implementations.

## Context capture

`ContextService` is the shared, renderer-independent location/weather boundary.
Create a `ContextRequest` with `ContextRequest::capture` or
`ContextRequest::enrich`, then call `capture_report` or `enrich` with optional
injected `HttpClient`, `Clock`, `Cancellation`, and `ContextCache`
implementations. `capture_report` owns a short mutation-lock phase; headless
`enrich` requires an explicit frozen `BackupPolicy` on the request and owns a
fresh verified-backup phase. Integrations that already own a backup/mutation
guard can call `ContextService::prepare` before acquiring it and
`persist_prepared` inside the short write phase; provider work never needs the
guard. `ContextPreparation::report` exposes provider candidates for inspection;
`into_report` abandons persistence and clears unsaved candidates so the final
report cannot claim they were attached. Provider work
shares one deadline capped at eight seconds;
offline, disabled, skipped, cancelled, malformed, and unavailable outcomes are
reported independently for location and weather. Enrichment re-reads the
location row inside an immediate transaction and fills only missing fields,
while an optional weather cache is limited to fifteen-minute,
coordinate- and provider-specific observations.

The default desktop adapter continues to read Capsule's flat `location.*`
settings and preserves `source=default|ip`, existing geocoding-cache behavior,
Open-Meteo historical timestamps, and MET Norway's current-forecast limitation
for old entries. Context providers receive only coordinates, configured place
names, and entry timestamps; journal text, titles, tags, and summaries are not
part of the request surface. `load_context_settings` exposes only the safe
effective `ContextSettings { policy, valid }` projection for diagnostics and
CLI consumers; it never returns raw configuration or secrets.

For durable headless capture, freeze the destination identity and an explicit
backup policy before serializing a request, then call
`capture_entry_for_database`. The capture path reserves the supplied
`entry_*` UUID, verifies an atomic SQLite backup, performs the entry/tags/
continuation/FTS/resequence mutation under bounded coordination, and returns a
`CommitReceipt` without a post-commit detail query. A retry can call
`reconcile_capture_for_database` read-only; matching normalized fields return
the existing UUID once, while changed content or a replaced database is
reported explicitly. `CaptureRequest`, `CommitReceipt`, and context DTOs are
renderer-independent seams for desktop and non-desktop clients.

Context or other deadline-bound workers can use
`with_mutation_lock_for_database_with_timeout` or
`with_database_backup_for_database_using_policy_with_timeout` to bound lock
coordination by their remaining budget. The latter shares one timeout across
the database and backup-directory locks and keeps both locks through backup
publication and the caller's transaction closure.

`JournalReader` adds an explicit-path, query-only boundary with bounded entry
lists, exact UUID/number lookup, tag and mood discovery, and date/search
metadata pages. `search` keeps Capsule's structured tokens and FTS fallback
diagnostics without invoking repair.

## Memory metrics

`stats` provides the shared, query-only projection used by cap's R2 memory
commands. `memory_calendar_for_database`, `memory_stats_for_database`, and
`memory_garden_for_database` use stored local dates, exclude hidden entries by
default, count words from `text_plain` (falling back to authored text), and
return complete calendar/garden day grids. Aggregate paths stream their
date/text projection without an implicit cap; full entry projections support
explicit `MemoryQuery::page(limit, offset)`. Recall reservoir-samples bounded
metadata before fetching selected bodies, and on-this-day pages fetch bodies
only after metadata pagination. `get_writing_calendar_for_database` is the
desktop-compatible calendar model and keeps mood catalog overrides/image
counts in the shared path. `current_streak` remains current when the latest
writing day is today or yesterday, including month boundaries.
`milestone_crossing_for_database` subtracts the exact committed UUID
contribution before testing the 50-word daily and 500-word weekly thresholds,
using a progress-handler deadline plus bounded busy wait so pre-existing
totals never create a false glint. All these functions accept an explicit
database path and do not write the journal or Capsule gamification tables.

```rust
use capsule_core::{JournalReader, ReadOptions};

let reader = JournalReader::open("/path/to/capsule.db")?;
let recent = reader.recent(ReadOptions::default())?;
let result = reader.search("tag:work", ReadOptions::default())?;
```
