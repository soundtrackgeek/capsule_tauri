# capsule-core

`capsule-core` is the headless shared behavior used by Capsule desktop and
non-desktop clients. It owns the SQLite database, backup, entry, location, and
model modules without a Tauri, WebView, tray, window, or shell dependency.

Use `ResolveRequest` with an explicit `ResolverEnvironment` snapshot when a
client needs deterministic path/config resolution. `entries::*_read_only_*`
queries inspect supported schema and refuse layouts that would require ID
repair; the repair-capable desktop convenience wrappers remain available for
legacy behavior.

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
