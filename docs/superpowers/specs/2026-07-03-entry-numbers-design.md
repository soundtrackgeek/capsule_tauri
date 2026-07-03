# Entry Numbers Design

Date: 2026-07-03

## Context

Old Capsule showed every journal entry with a human-facing number: the first
entry was `#1`, the second was `#2`, and so on. Capsule Tauri already returns
the SQLite `entries.id` field with each entry, accepts numeric IDs in entry
lookups, and resequences later IDs after deletes. That makes `entries.id` the
right compatibility source for the visible entry number.

The feature should make the number visible in the Dashboard, Entries screen,
and Search. It should also ensure old or imported rows that do not have a valid
number receive one, and that newly created posts continue to get a number.

## Decision

Use `entries.id` as the only public entry number and display it as `#<id>`.
Do not add a separate `entry_number` column and do not compute a rank from
publish date at render time.

## Behavior

- Dashboard recent, pinned, and random entries show each entry number.
- Entries list cards show each entry number.
- Search result cards show each entry number.
- Entry detail metadata includes the same entry number.
- New entries return the SQLite-assigned ID through the existing `Entry`
  response.
- Legacy repair assigns numbers to entries that do not have a valid ID, using
  publish order as the ordering intent: `created_at ASC`, then the previous row
  identity as a stable tie breaker where SQLite exposes one.
- Deleting an entry keeps the existing behavior of resequencing later IDs.

## Architecture

Backend entry reads and writes continue to flow through `entries.rs`.

Add a small ID repair helper near the existing entry ID maintenance code. The
helper should run before creating a new entry and may also be triggered by the
first read command that detects missing or invalid entry IDs. Because this is a
database mutation, any actual repair must run through the existing verified
backup guard before writing. Because SQLite `INTEGER PRIMARY KEY` tables
normally cannot contain rows without an ID, the helper should be conservative
and no-op on the normal schema. It should exist to handle compatible legacy
variants, not to rewrite healthy databases.

The repair should:

- Inspect the `entries` table columns before changing anything.
- Only attempt writes through a backup-guarded read-write connection.
- Preserve current valid IDs when possible.
- Assign missing or invalid IDs after the current max ID.
- Keep `sqlite_sequence` aligned when the table uses AUTOINCREMENT.
- Rebuild `entries_fts` if IDs are changed.

Frontend code already shares `EntryCardContent` for Entries and Search, and
`EntryMini` for the Dashboard. Add one small display helper/component so both
places render the same `#<id>` label.

## Error Handling

ID repair failures should be surfaced from the backend instead of silently
hiding data problems. Normal listing should remain read-only when IDs are
already valid. If repair is needed but the active database shape cannot be
repaired safely, the user should receive a clear backend error rather than
partial renumbering.

## Testing

Add Rust tests for:

- Listing entries exposes the expected existing `id` values.
- Creating a new entry returns the assigned next `id`.
- Any repair helper is a no-op for the standard `INTEGER PRIMARY KEY
  AUTOINCREMENT` fixture.

Add frontend coverage only if the current Vitest setup can render the display
helper cleanly without pulling the full Tauri app shell into the test.

## Documentation And Release

Update `README.md` to mention visible entry numbers on Dashboard, Entries, and
Search. Add a new `CHANGELOG.md` release entry with a semantic minor version.
If this is released, keep `package.json`, `package-lock.json`,
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `CHANGELOG.md` on the
same version, then commit, push, tag, and push the tag.

## Out Of Scope

- Adding a separate manual entry-number editing UI.
- Preserving gaps after deletes.
- Changing sync file formats unless an implementation test proves current sync
  can import rows without valid IDs.
- Showing entry numbers on unrelated surfaces such as Images, Threads,
  Calendar, Analytics, or Cover Wall.
