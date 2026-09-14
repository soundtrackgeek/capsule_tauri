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
implementations. Integrations that own a backup/mutation guard can call
`ContextService::prepare` before acquiring it and `persist_prepared` inside
the short write phase; provider work never needs the guard. Provider work
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
