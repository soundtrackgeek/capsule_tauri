# Entry Numbers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show old-Capsule-style entry numbers from `entries.id` on Dashboard, Entries, Search, and detail views, while ensuring imported entries and new entries receive IDs.

**Architecture:** Keep `entries.id` as the single source of truth. Add a backend helper that validates or repairs the visible ID column before entry reads/writes, then render `#<id>` through a small frontend formatter so all entry cards use the same label.

**Tech Stack:** Rust, rusqlite, Tauri commands, React, TypeScript, Vitest, Cargo tests.

---

### Task 1: Backend Entry ID Tests

**Files:**
- Modify: `src-tauri/src/entries.rs`

- [ ] **Step 1: Write failing tests**

Add tests under the existing `#[cfg(test)] mod tests` in `src-tauri/src/entries.rs`:

```rust
#[test]
fn list_entries_exposes_entry_numbers_from_ids() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = create_fixture_database(temp_dir.path());

    let response = list_entries_for_database(
        &db_path,
        EntryFilters {
            include_hidden: Some(true),
            sort: Some(EntrySort::Asc),
            ..EntryFilters::default()
        },
    )
    .expect("entries");

    let ids = response
        .entries
        .iter()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![1, 2, 3, 4]);
}

#[test]
fn create_entry_returns_next_entry_number() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = create_fixture_database(temp_dir.path());

    let response = create_entry_for_database(
        &db_path,
        EntryCreate {
            text: "Numbered new entry".to_string(),
            when: Some("2026-02-01T09:30".to_string()),
            ..EntryCreate::default()
        },
    )
    .expect("create entry");

    assert_eq!(response.entry.id, 5);
}

#[test]
fn ensure_entry_ids_repairs_nullable_legacy_ids() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = create_nullable_id_fixture_database(temp_dir.path());

    ensure_entry_ids_for_database(&db_path).expect("repair ids");

    let response = list_entries_for_database(
        &db_path,
        EntryFilters {
            include_hidden: Some(true),
            sort: Some(EntrySort::Asc),
            ..EntryFilters::default()
        },
    )
    .expect("entries");
    assert_eq!(
        response
            .entries
            .iter()
            .map(|entry| (entry.id, entry.uuid.as_str()))
            .collect::<Vec<_>>(),
        vec![(1, "legacy_one"), (2, "legacy_two"), (3, "legacy_three")]
    );
}
```

Add a fixture helper:

```rust
fn create_nullable_id_fixture_database(path: &Path) -> std::path::PathBuf {
    let db_path = path.join("legacy-nullable-id.db");
    let connection = Connection::open(&db_path).expect("open db");
    connection
        .execute_batch(
            "
            CREATE TABLE entries (
                id INTEGER,
                uuid TEXT UNIQUE,
                created_at TEXT NOT NULL,
                updated_at TEXT,
                text TEXT NOT NULL,
                text_plain TEXT NOT NULL DEFAULT '',
                content_format TEXT NOT NULL DEFAULT 'plain',
                title TEXT,
                summary TEXT,
                mood TEXT,
                starred INTEGER DEFAULT 0,
                pinned INTEGER DEFAULT 0,
                hidden INTEGER DEFAULT 0
            );
            CREATE TABLE tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE entry_tags (
                entry_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (entry_id, tag_id)
            );
            CREATE TABLE entries_fts (text);
            INSERT INTO entries
                (id, uuid, created_at, updated_at, text, text_plain, content_format, hidden)
            VALUES
                (NULL, 'legacy_two', '2026-01-02 08:00', '2026-01-02 08:00', 'Two', 'Two', 'plain', 0),
                (NULL, 'legacy_one', '2026-01-01 08:00', '2026-01-01 08:00', 'One', 'One', 'plain', 0),
                (NULL, 'legacy_three', '2026-01-03 08:00', '2026-01-03 08:00', 'Three', 'Three', 'plain', 0);
            INSERT INTO tags (name) VALUES ('legacy');
            INSERT INTO entry_tags (entry_id, tag_id) VALUES (1, 1), (2, 1), (3, 1);
            INSERT INTO entries_fts(rowid, text) VALUES (1, 'Two'), (2, 'One'), (3, 'Three');
            ",
        )
        .expect("fixture");
    drop(connection);
    std::fs::write(
        path.join("config.json"),
        r#"{"location.auto_capture": "false"}"#,
    )
    .expect("config");

    db_path
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cd src-tauri
cargo test entries::tests::list_entries_exposes_entry_numbers_from_ids entries::tests::create_entry_returns_next_entry_number entries::tests::ensure_entry_ids_repairs_nullable_legacy_ids
```

Expected: the first two may pass if current ID behavior is already present; the third must fail because `ensure_entry_ids_for_database` does not exist.

### Task 2: Backend ID Validation And Repair

**Files:**
- Modify: `src-tauri/src/entries.rs`
- Modify: `src-tauri/src/search.rs`
- Modify: `src-tauri/src/sync.rs`

- [ ] **Step 1: Implement schema inspection and repair**

Add a small entry ID helper to `src-tauri/src/entries.rs` near `update_sqlite_sequence`:

```rust
#[derive(Debug, Clone)]
struct EntryIdColumnInfo {
    exists: bool,
    primary_key: bool,
}

pub(crate) fn ensure_entry_ids_for_database(db_path: &Path) -> Result<()> {
    let connection = db::open_read_only_connection(db_path)?;
    let tables = detected_tables(&connection)?;
    ensure_entries_table(&tables)?;
    let needs_repair = entry_ids_need_repair(&connection)?;
    drop(connection);

    if !needs_repair {
        return Ok(());
    }

    backup::with_database_backup_for_database(db_path, "entry.ids.repair", |path| {
        let connection = db::open_read_write_connection(path)?;
        repair_entry_ids(&connection)
    })?;
    Ok(())
}
```

Implement `entry_id_column_info`, `entry_ids_need_repair`, `repair_entry_ids`, `next_entry_id`, and `insert_entry_sql` so:

```rust
fn next_entry_id(connection: &Connection) -> Result<i64> {
    connection
        .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM entries", [], |row| row.get(0))
        .context("failed to calculate next entry id")
}
```

The repair function should:

- Add `id INTEGER` when no visible `id` column exists.
- Select rows ordered by `datetime(created_at) ASC, rowid ASC`.
- Assign IDs `1..n` to rows with missing or non-positive IDs.
- Error on duplicate positive IDs.
- Rebuild `entries_fts`.
- Call `update_sqlite_sequence`.

- [ ] **Step 2: Call the helper before entry reads and writes**

Call `ensure_entry_ids_for_database(db_path)?` at the start of:

```rust
list_entries_for_database
get_entry_for_database
list_entries_by_uuids_for_database
get_random_entry_for_database
list_entry_history_for_database
create_entry_inner
update_entry_inner
delete_entry_inner
set_entry_flag_inner
```

For insert paths, include `id` explicitly:

```rust
let entry_id = next_entry_id(&tx)?;
tx.execute(
    "INSERT INTO entries
        (id, uuid, created_at, updated_at, text, text_plain, content_format, title, summary, mood, starred, pinned, hidden)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)",
    params![entry_id, uuid, created_at, updated_at, text, text_plain, content_format, title, summary, mood, bool_to_int(starred), bool_to_int(pinned)],
)?;
```

Change sync inserts in `src-tauri/src/sync.rs` to calculate and insert the next ID instead of relying on `last_insert_rowid`.

- [ ] **Step 3: Ensure Search repairs before direct SQL**

In `src-tauri/src/search.rs`, call:

```rust
entries::ensure_entry_ids_for_database(db_path)?;
```

at the start of `search_entries_for_database`.

- [ ] **Step 4: Run tests and verify GREEN**

Run:

```powershell
cd src-tauri
cargo test entries::tests::list_entries_exposes_entry_numbers_from_ids entries::tests::create_entry_returns_next_entry_number entries::tests::ensure_entry_ids_repairs_nullable_legacy_ids
```

Expected: all three tests pass.

### Task 3: Frontend Entry Number Formatter

**Files:**
- Modify: `src/lib/format.ts`
- Create: `src/lib/format.test.ts`

- [ ] **Step 1: Write failing formatter tests**

Create `src/lib/format.test.ts`:

```ts
import { describe, expect, test } from "vitest";

import { formatEntryNumber } from "./format";

describe("formatEntryNumber", () => {
  test("formats positive entry IDs as old Capsule numbers", () => {
    expect(formatEntryNumber(42)).toBe("#42");
  });

  test("falls back when an entry ID is unavailable", () => {
    expect(formatEntryNumber(0)).toBe("#?");
    expect(formatEntryNumber(null)).toBe("#?");
    expect(formatEntryNumber(undefined)).toBe("#?");
  });
});
```

- [ ] **Step 2: Run test and verify RED**

Run:

```powershell
npm test -- src/lib/format.test.ts
```

Expected: FAIL because `formatEntryNumber` is not exported.

- [ ] **Step 3: Implement formatter**

Add to `src/lib/format.ts`:

```ts
export const formatEntryNumber = (id: number | null | undefined) => {
  if (!id || id < 1) {
    return "#?";
  }

  return `#${id}`;
};
```

- [ ] **Step 4: Run test and verify GREEN**

Run:

```powershell
npm test -- src/lib/format.test.ts
```

Expected: PASS.

### Task 4: Render Entry Numbers

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Import the formatter**

Change:

```ts
import { formatBytes, formatDateTime } from "./lib/format";
```

to:

```ts
import { formatBytes, formatDateTime, formatEntryNumber } from "./lib/format";
```

- [ ] **Step 2: Add a shared number badge**

Add near `EntryMeta`:

```tsx
function EntryNumber({ entry }: { entry: Entry }) {
  return (
    <span className="entry-number" title="Entry number">
      {formatEntryNumber(entry.id)}
    </span>
  );
}
```

- [ ] **Step 3: Show numbers in mini and card views**

In `EntryMini`, add `EntryNumber` next to the heading content. In `EntryCardContent`, add `EntryNumber` beside the existing attachment chip. The cards should show the date eyebrow, title, and `#id` without pushing text outside the card.

- [ ] **Step 4: Show numbers in detail metadata**

Add:

```tsx
<Detail label="Number" value={formatEntryNumber(entry.id)} />
```

above the UUID detail row.

- [ ] **Step 5: Add badge styles**

Add CSS:

```css
.entry-number {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  border-radius: 999px;
  background: #e0eadf;
  color: #214c2f;
  padding: 0 8px;
  font-size: 12px;
  font-weight: 850;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
```

Include dark and retro theme color rules alongside existing chip theme rules.

### Task 5: Documentation, Version, And Release Notes

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Bump version to `0.21.0`**

Change version strings in all listed version files from `0.20.0` to `0.21.0`.

- [ ] **Step 2: Update README feature list**

Add entry-number language to the Dashboard/Entries/Search bullets.

- [ ] **Step 3: Update CHANGELOG**

Add:

```markdown
## 0.21.0 - 2026-07-03

### Added

- Added old-Capsule-style entry numbers from `entries.id` on Dashboard,
  Entries, Search, and entry detail views.

### Fixed

- Added conservative entry ID repair so imported legacy rows without visible
  entry IDs receive numbers before new entries are created.

### Changed

- Bumped the app version to 0.21.0.
```

### Task 6: Verification, Commit, Push, Tag

**Files:**
- All modified files.

- [ ] **Step 1: Run frontend tests**

Run:

```powershell
npm test
```

Expected: PASS.

- [ ] **Step 2: Run frontend build**

Run:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 3: Run Rust tests**

Run:

```powershell
cd src-tauri
cargo test
```

Expected: PASS.

- [ ] **Step 4: Review git diff**

Run:

```powershell
git status --short
git diff --stat
```

Expected: only planned files changed.

- [ ] **Step 5: Commit**

Run:

```powershell
git add README.md CHANGELOG.md package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/src/entries.rs src-tauri/src/search.rs src-tauri/src/sync.rs src/App.tsx src/styles.css src/lib/format.ts src/lib/format.test.ts docs/superpowers/plans/2026-07-03-entry-numbers.md
git commit -m "Add entry numbers"
```

- [ ] **Step 6: Push and tag**

Run:

```powershell
git push
git tag v0.21.0
git push origin v0.21.0
```

Expected: branch push succeeds and release tag push triggers the Windows installer workflow.
