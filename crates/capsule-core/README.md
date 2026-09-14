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

`normalize_capture_content(&request)` performs no I/O and returns a Serde/Eq
projection using the writer's authored-content normalization. Clients can hash
its serialized bytes for retry comparison without ambiguous delimiter joins.
The projection excludes invocation timestamps and identity: clients must still
validate the frozen database binding and replay the original request/UUID/time.

Capture has one 15-second operation budget. The same deadline covers preflight,
database and backup-directory lock acquisition, SQLite busy waits (including
`BEGIN IMMEDIATE` and legacy ID repair), backup stepping/verification,
publication, and commit preparation; there are no stacked per-stage waits.
CPU work and filesystem I/O consume that budget too, so a large or slow backup
can report `database_busy` once the deadline expires.

Timed SQLite operations use a scoped monotonic busy handler because SQLite's
default timeout counts requested sleep intervals rather than elapsed wall time.
The callback owns no connection state and is unregistered before its boxed
deadline is freed. Contention can still succeed when the other writer releases
its lock before the deadline; it is not converted to an immediate refusal.

Context or other deadline-bound workers can use
`with_mutation_lock_for_database_with_timeout` or
`with_database_backup_for_database_using_policy_with_timeout` to bound lock
coordination by their remaining budget. The latter shares one timeout across
the database and backup-directory locks, bounds backup stepping/publication,
and keeps both locks through the caller's transaction closure. Its timeout
does not interrupt arbitrary closure CPU/I/O; callers with a wider deadline
should check it inside that closure (as capture does).

`JournalReader` adds an explicit-path, query-only boundary with bounded entry
lists, exact UUID/number lookup, tag and mood discovery, and date/search
metadata pages. `search` keeps Capsule's structured tokens and FTS fallback
diagnostics without invoking repair.

```rust
use capsule_core::{JournalReader, ReadOptions};

let reader = JournalReader::open("/path/to/capsule.db")?;
let recent = reader.recent(ReadOptions::default())?;
let result = reader.search("tag:work", ReadOptions::default())?;
```
