# capsule-core

`capsule-core` is the headless shared behavior used by Capsule desktop and
non-desktop clients. It owns the SQLite database, backup, entry, location, and
model modules without a Tauri, WebView, tray, window, or shell dependency.

Use `ResolveRequest` with an explicit `ResolverEnvironment` snapshot when a
client needs deterministic path/config resolution. `entries::*_read_only_*`
queries inspect supported schema and refuse layouts that would require ID
repair; the repair-capable desktop convenience wrappers remain available for
legacy behavior. `CaptureRequest`, `CommitReceipt`, and context DTOs are
renderer-independent seams for later capture/context implementations.
