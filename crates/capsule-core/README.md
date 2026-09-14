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
