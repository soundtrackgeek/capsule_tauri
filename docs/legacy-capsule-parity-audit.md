# Legacy Capsule feature-parity audit

**Audit date:** 2026-07-23

**Current app:** `C:\_code\capsule_tauri` (`0.31.0`)

**Legacy app:** `C:\_code2\capsule_exp_ai` (`0.63.65` web frontend)

**Purpose:** Identify legacy capabilities that are absent or only partly implemented in the Tauri app, then decide whether to port, improve, replace, or skip them.

## Executive conclusion

The Tauri app already covers the journal's core loop well: safe local writes, entry creation and editing, images, threads, search, analytics, Wrapped, calendar, cover wall, AI chat, sync, backups, and gamification. It also improves on legacy Capsule in several desktop-specific areas.

The largest remaining parity gaps are not whole top-level pages. They are depth gaps inside Analytics, entry browsing, the composer, threads, the writing calendar, AI analysis, gamification, data portability, and location/weather management. The other large category is the legacy plugin ecosystem: Dream Log, Coding Ideas, Post Ideas, and Writing Ideas have no native Tauri equivalents.

My recommendation is **not** to reproduce legacy Capsule screen for screen. Several legacy features duplicate newer Tauri pages or would make the desktop app busier. The best path is:

1. Finish small, high-value gaps in existing flows.
2. Add selected analytical depth without duplicating Calendar and Wrapped.
3. Turn the four legacy plugin screens into one reusable Collections/Logs system.
4. Treat encryption, import, semantic search, and global undo as separate high-risk projects.
5. Keep the CLI, web server, and companion capture apps interoperable rather than absorbing them into Tauri.

## How to read this report

### Current-state labels

- **Missing:** no corresponding user-facing capability was found in the Tauri app.
- **Partial:** the Tauri app covers the main feature, but not all meaningful legacy behavior.
- **Backend-ready:** supporting data or commands exist, but the Tauri UI does not expose them.
- **Replaced:** the Tauri app solves the same need differently.
- **Keep separate:** part of the wider legacy ecosystem, but not a sensible desktop-app feature.

### Effort guide

- **XS:** less than one day.
- **S:** roughly 1–3 days.
- **M:** roughly 3–8 days.
- **L:** roughly 2–4 weeks.
- **XL:** a month or more.

These are relative implementation estimates, not delivery promises. Security, migration, cross-platform behavior, tests, and data compatibility are included where relevant.

### Recommendation labels

- **Implement:** a clear improvement to the current app.
- **Improve:** port the underlying idea, but redesign it for Tauri.
- **Consider:** useful only if the workflow is still wanted.
- **Skip:** low-value, duplicative, or riskier than its benefit.

## Priority snapshot

| Priority | Area | Current state | Recommendation | Effort |
|---|---|---|---|---|
| P1 | Safe rendered Markdown in entry reading views | Missing | Implement once and reuse in Entries, Cover Wall, Search, and threads | S |
| P1 | Starred/pinned filters and saved views | Backend-ready/partial | Implement as quick filters and reusable saved views; do not add a separate Starred page | S–M |
| P1 | On This Day | Missing | Implement as an optional dashboard card with entry drill-down | S |
| P1 | Writing Calendar day drill-down and yearly summary | Partial | Implement day click-through first; add summary cards second | S–M |
| P1 | Searchable continuation and thread-building flows | Partial/backend-ready | Replace UUID entry fields with pickers and expose bulk thread linking | M |
| P1 | Templates and prompts in the composer | Missing | Implement lightweight text insertion, not a second editor architecture | S–M |
| P1 | Reading-time and source analytics | Missing | Add to Analytics as focused modules | S–M |
| P2 | Word-frequency explorer | Partial | Turn Top Words into an interactive explorer with period comparison | M |
| P2 | Per-entry revision restore | Partial | Add restore from the existing edit history before attempting global undo | M |
| P2 | Manual location/weather correction | Missing | Add edit/remove/refresh actions per entry | M |
| P2 | Thread AI metadata and bulk builder | Partial/backend-ready | Implement in the current Threads design | M |
| P2 | Locked badge progress and hero selection | Partial | Implement only if gamification remains a product priority | M |
| P2 | Generic Collections/Logs framework | Missing | Use it to replace multiple one-off legacy plugins | L |
| P3 | Semantic/hybrid search and related entries | Missing | Prefer local embeddings or an explicit opt-in cloud mode | L |
| P3 | Deep correlation and weather analytics | Partial | Add sample sizes and uncertainty; never imply causation | L |
| P3 | Broader export/import formats | Partial | Add exports incrementally; treat imports as a migration project | M–L |
| Separate project | SQLCipher encrypted journals | Missing | Implement only with a clear compatibility requirement and migration plan | XL |
| Avoid | Bespoke AES mode, global undo, RPG battle polish | Missing | Skip or defer; cost and risk outweigh current value | L–XL |

---

## 1. Analytics

**Overall status: Partial.** Both apps have Analytics, and the Tauri app already includes the eight activity-trend modes from legacy Capsule. The missing depth is in reading analysis, vocabulary/topic exploration, correlations, weather, and layout customization.

The legacy layout customizer exposes these 19 sections:

1. Overview Stats
2. Mobile vs Desktop
3. Selected View Reading Time
4. Entire Journal Reading Time
5. Reading Time by Month
6. Reading Time by Year
7. Activity Trends
8. Word Frequency Explorer
9. Daily Topic Mix
10. Calendar Heatmap
11. Mood Sentiment Over Time
12. Mood by Time
13. Mood Distribution
14. Top Tags
15. Correlation Dashboard
16. Weather Analytics
17. Tag Cloud
18. All Moods
19. Year Wrapped

The Tauri page currently has:

- period filtering;
- entries, words, average words, average mood, images, location coverage, and streak metrics;
- Entry Frequency, Words Over Time, First/Last Capsule, Words per Hour, Words per Day, Notes per Hour, Notes per Day, and Entries per Location charts;
- monthly activity and mood summaries;
- simple top tag, mood, location, weather, and word lists.

### Exact gaps

| Missing or partial capability | Current Tauri state | Comment and recommendation | Effort |
|---|---|---|---|
| Selected-period reading time | No reading-time summary in Analytics; only per-entry writing stats | **Implement.** It is cheap, understandable, and useful. Show estimated reading time beside words, with the formula documented in a tooltip. | XS–S |
| Lifetime reading time | Missing | **Consider.** A lifetime total is pleasant but not very actionable. It can share the same calculation as selected-period reading time. | XS |
| Reading time by month/year | Missing | **Improve.** One trend chart with a month/year granularity selector is cleaner than four separate legacy sections. | S |
| Mobile vs Desktop capture source | Missing | **Implement if source provenance is reliable.** Use explicit `source` metadata rather than inferring mobile from location rows. Include CLI, Tauri, mobile, and import as separate sources when known. | S–M |
| Word Frequency Explorer | Tauri only shows a static Top Words list | **Implement.** Let a word open matching entries, change minimum frequency, exclude common words, and compare two periods for rising/cooling vocabulary. This is one of the better legacy analytical tools. | M |
| Daily Topic Mix | Missing | **Consider.** It can surface interesting shifts, but the output needs an understandable topic-definition method. A tag-based “Themes over time” chart is safer than opaque automatic topics. | M–L |
| Mood by time of day | Missing | **Implement.** Straightforward aggregation with enough entries. Disable or qualify buckets with tiny sample sizes. | S–M |
| Mood by weekday | Missing | **Implement with the same module as time-of-day mood.** Show count and average sentiment together. | S |
| Rich mood distribution | Simple counts only | **Improve.** Add a compact bar or donut visualization and click-through to entries. A giant mood cloud is less readable. | S |
| Correlation Dashboard | Missing | **Consider, with strong statistical guardrails.** Legacy includes same-day and next-day tag effects, tag relationships, word count versus mood, and timing correlations. Always show sample size, effect size, and a “correlation is not causation” note. | L |
| Deep weather analytics | Tauri only counts conditions | **Improve.** Start with coverage, condition distribution, and temperature trend. Add mood/weather and tag/weather relationships only when sample sizes are meaningful. | M–L |
| Interactive tag cloud | Simple top-tag bars | **Skip the literal cloud.** Add click-through, search, trend direction, and period comparison to the existing readable bars instead. | S–M |
| Interactive all-moods cloud | Simple mood bars | **Skip the literal cloud.** Improve the existing distribution instead. | S |
| Analytics layout hide/reorder | Missing | **Consider after more modules exist.** With the current page, customization adds more control than value. A fixed Overview/Explore structure would be simpler. | M |
| Calendar heatmap inside Analytics | Calendar exists as a dedicated Tauri page | **Skip duplication.** Add a link or small preview to the dedicated Calendar instead. | XS |
| Year Wrapped inside Analytics | Wrapped already has a dedicated, stronger Tauri page with week/month/year navigation | **Skip duplication.** Keep Wrapped separate and link to it. | XS |
| “Health Score” | Missing | **Skip unless its formula is made explicit and actionable.** The legacy 0–100 number looks authoritative without explaining what “healthy journaling” means. Prefer concrete streak, frequency, and reflection metrics. | S if retained |

### Suggested Analytics redesign

Keep one **Overview** page for high-level activity, then add an **Explore** area with selectable modules:

- Reading and volume
- Vocabulary and themes
- Mood and timing
- Location and weather
- Relationships

This preserves the strongest legacy analysis without recreating a very long configurable dashboard.

---

## 2. Dashboard

**Overall status: Partial by choice.** The Tauri dashboard is intentionally calmer and more safety-oriented. It shows journal counts, database health, backup status, recent entries, pinned entries, a random entry, and warnings. The legacy dashboard is much denser.

| Legacy capability not present on the Tauri dashboard | Comment and recommendation | Effort |
|---|---|---|
| Seven-day trend sparklines | **Consider.** A single compact activity sparkline would add context; four separate sparklines are unnecessary. | S |
| Streak Center with daily and weekly streaks | **Implement as a compact optional card.** Current Profile already has broader gamification, so the dashboard only needs current/best streak and the next milestone. | S |
| On This Day | **Implement.** It suits a journal dashboard and provides more value than several statistical cards. Make it expandable and link to full entry reading. | S |
| Top tag card | **Skip on the default dashboard.** Analytics already covers it. It could be an optional widget later. | XS |
| Health Score | **Skip or redefine.** See the Analytics note above. | S |
| “Simply The Best” records: biggest day/month, longest entry, most tags | **Consider as a Records section in Profile or Wrapped.** It is engaging, but too much for the default dashboard. | S–M |
| Adventure Snapshot | Profile covers XP, level, quests, and hero path | **Skip duplication.** If quests need visibility, show only a small “rewards ready” badge in navigation. | XS–S |
| Badge Display Case | Profile lists earned badges | **Improve Profile instead of duplicating it.** Add locked-badge progress there. | M |
| Sync Status card | Tauri has a dedicated Sync page | **Skip duplication.** A small status indicator in the shell is enough if background sync needs visibility. | XS–S |
| AI Time Capsule card | No equivalent user flow | **Consider as a later opt-in reflection feature**, not a permanent dashboard block. See AI section. | L |
| Milestone wall | Profile has a simpler gamification overview | **Move to Profile if implemented.** Do not make the main dashboard scroll through every milestone. | M |

**Recommended dashboard additions:** On This Day and one compact streak/activity card. Everything else should remain in Profile, Analytics, Sync, or Wrapped.

---

## 3. Entry list, browsing, and saved views

**Overall status: Partial.** The Tauri app has strong text, tag, mood, location, date, image, hidden-entry, and sort filters. Its backend can also filter starred and pinned entries, but the main UI does not expose those filters.

| Missing or partial capability | Current Tauri state | Comment and recommendation | Effort |
|---|---|---|---|
| Starred-only filter | Backend-ready | **Implement as a quick chip.** Do not add a separate Starred page; the legacy page is just a fixed saved view. | XS |
| Pinned-only filter | Backend-ready | **Implement as a quick chip.** | XS |
| Combined starred/pinned/hidden state filters | Hidden is exposed; starred and pinned are not | **Implement** with clear Any/Yes/No states rather than many checkboxes. | S |
| Saved filter presets | Missing | **Implement as Saved Views.** Preserve keyword, tags, moods, date range, media, location, state filters, and sort. Saved Views can replace the legacy Starred page too. | M |
| Preset manager | Missing | **Implement with Saved Views**, including rename, duplicate, set as default, and delete. | S |
| Cards versus Feed layout | One list/detail layout | **Consider.** A reading feed is useful for long chronological review, but list/detail is more efficient for management. Add only if there is a real browsing need. | M |
| Infinite/virtualized feed | Missing | **Do not add pre-emptively.** Use virtualization only if real databases make the current list slow. | M |
| Multiple mood OR selection | Comma-separated filter input can express multiple values, but the interaction is not discoverable | **Improve** with tokenized multi-select and autocomplete. | S |
| Export the exact filtered view | Search export exists, but the workflow is less explicit than legacy | **Improve** by showing the active filter summary in the export dialog and naming the scope in the file. | S |
| Dedicated Starred route | Missing | **Skip.** Saved Views plus a one-click Starred chip are better. | XS |

---

## 4. Entry reading and editing

**Overall status: Partial.** The Tauri composer is robust and safe, but intentionally plain: Markdown textarea, draft recovery, queued attachments, metadata, AI title/summary, and writing statistics. The legacy composer has more guided-writing and editor tooling.

| Missing or partial capability | Current Tauri state | Comment and recommendation | Effort |
|---|---|---|---|
| Rendered Markdown when reading an entry | Entry detail displays the stored text rather than a rendered Markdown document | **Implement first.** Use GitHub-flavored Markdown with sanitization, safe links, and controlled local-image handling. Reuse it in Entry Detail, Search results, Cover Wall, threads, and previews. | S |
| Split Markdown preview | Missing | **Consider.** A toggleable preview is a lower-risk improvement than replacing the editor. | S–M |
| Rich-text/WYSIWYG editing | Plain Markdown textarea | **Consider carefully.** A full WYSIWYG path increases conversion, paste, image, and round-trip complexity. Prefer a small Markdown toolbar and preview unless non-technical editing is a priority. | L |
| Apply a reusable template in the composer | Tauri already manages built-in/custom template records in Settings, but the composer has no template picker or Apply action | **Implement by wiring the existing library into the composer.** Add preview, Apply, and “replace or append” behavior; keep template management in Settings. | S |
| Prompt of the Day and random prompt insertion | Tauri already manages a prompt library in Settings, but the composer does not surface it | **Implement as an optional prompt drawer backed by the existing library.** Support random/category selection and insertion without crowding the writing canvas. | S |
| Writing session timer | Missing | **Consider.** Start/pause/resume plus a small session summary is useful for deliberate writing, but should not be required for normal entries. | M |
| Writing goals: words/day, entries/day, words/entry | Current editor only reports statistics | **Improve as optional goals in Profile/Settings**, with a subtle composer progress indicator. Avoid alerts that punish short journal entries. | M |
| Word-target completion alert | Missing | **Consider only as part of optional goals.** Use a quiet visual completion state, not a bell by default. | S |
| Typing/key sounds | Missing | **Skip by default.** If retained, make it an accessibility-conscious optional sound theme with volume control. | S–M |
| Searchable continuation picker | Raw “Continue from UUID” field | **Implement.** Search recent entries by title, date, text, tag, or ID and show the selected parent clearly. This is much safer than manually entering an internal UUID. | M |
| Focus/toolbar configuration | Writer provides a focused window, but the editor itself has little configurable chrome | **Consider.** Writer already solves most focus needs; add only small controls such as preview, width, and metadata visibility. | S–M |
| Restore a prior revision | Edit-history snapshots are displayed, but there is no complete restore workflow | **Implement.** Restore should create a new revision and preserve the current state, not overwrite history destructively. | M |

---

## 5. Search and discovery

**Overall status: Partial.** Structured keyword/FTS search is implemented. Semantic and hybrid options are visibly disabled in the Tauri app, and related-entry discovery is absent.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Semantic search | **Consider as a strategic feature.** Prefer local embeddings stored beside the journal. If cloud embeddings are offered, make indexing scope, provider, cost, and transmitted text explicit. | L |
| Hybrid keyword + semantic search | **Implement only after semantic search is trustworthy.** Use keyword ranking as the stable base and semantic similarity as a visible reranking factor. | M after semantic |
| AI-assisted search | **Consider.** A natural-language filter translator can be safer and cheaper than sending full journal contents to a model. Show the generated filters before running them. | M |
| Related entries | **Implement a local first version.** Start with shared tags, thread links, title terms, and TF-IDF similarity before requiring embeddings. Explain why each result is related. | M |
| Saved search/filter presets | Missing; same Saved Views gap as Entries | **Implement once and share with Entries.** | M |
| Starred/pinned search filters | Backend supports the state; Search UI does not expose it | **Implement.** | XS–S |

---

## 6. Threads and continuations

**Overall status: Partial/backend-ready.** Tauri can list threads, edit thread title/summary, detach leaves, and disband a thread. A bulk-link command exists, but there is no corresponding bulk builder in the current UI.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Bulk-select entries and create/link a thread | **Implement.** Add selection from Entries/Search, then a review step that shows the resulting order and existing links. | M |
| Searchable parent/target entry picker | **Implement together with the composer continuation picker.** One reusable entry-picker component can serve both. | M |
| Reorder or validate thread chronology before linking | Missing | **Improve over legacy.** Default to chronological order, flag cycles/duplicates, and let the user reorder before saving. | M |
| AI thread title and summary suggestions | **Implement as an opt-in action.** The current AI provider layer already supports metadata generation patterns. Preview before applying. | M |
| Thread search/filter | Basic thread browsing only | **Consider** title, tag, date, and length filters if the thread count grows. | S–M |
| Visual thread map | Missing | **Consider later.** A simple ordered timeline is more useful than a decorative graph for most threads. | M |

---

## 7. Writing Calendar

**Overall status: Partial.** Tauri has a useful year/month calendar with entry-count intensity, sentiment markers, word/image/mood details, and year navigation. Legacy adds drill-down and more annual summaries.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Click a day to see that day's entries | **Implement.** Open a compact drawer or navigate to Entries with an exact-date Saved View. | S |
| Total words, entries, active days, longest streak, longest dry spell, busiest day | **Implement as a small annual summary row.** These are clear and useful. | S–M |
| Monthly word totals chart | **Consider.** A compact chart adds useful seasonality context but overlaps Analytics. A link to a preconfigured Analytics view may be enough. | S |
| Word-intensity mode | Tauri heatmap is primarily entry-count based | **Implement a count/words selector** if word volume matters. Keep sentiment as the existing overlay. | S |
| Calendar heatmap duplicated inside Analytics | Not duplicated | **Keep it that way.** The dedicated page is the better home. | XS |

---

## 8. Cover Wall

**Overall status: Partial.** Tauri supports covers, filtering, thumbnails, and an inline reader. Legacy adds more visual layout controls.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Masonry versus even grid | **Consider.** Masonry suits mixed cover ratios; even grid is easier to scan. Offer two named layouts, not a large settings panel. | M |
| Fit/natural/uniform image sizing | **Consider as part of the two layout modes.** | S |
| Cover width, gap, and corner-radius controls | **Skip most knobs.** A compact/comfortable density control is enough. | S |
| Fullscreen cover browsing | **Implement if Cover Wall is used as a visual memory browser.** It is a natural desktop feature. | S–M |
| Rich Markdown reader in the cover drawer | Raw/plain reading path | **Solved by the shared safe Markdown renderer recommended above.** | Included in S |
| More discoverable multi-mood filtering | Text input can represent multiple moods | **Improve with the shared tokenized multi-select.** | S |

---

## 9. AI and local intelligence

**Overall status: Partial.** Tauri has a stronger persistent AI chat experience than legacy in several respects: conversations, provider selection, streaming, cancel/retry, context preview, and secure credential storage. It also supports title/summary generation and local heuristic metadata. The gaps are dedicated analysis recipes, local model support, semantic indexing, and scheduled reflection.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Ollama/local model provider | **Consider if offline privacy is a product goal.** Add health/model discovery and clear capability limits. Avoid silently falling back to cloud. | L |
| Local embedding index | Missing | **Consider together with semantic search.** It needs incremental indexing, model/version tracking, rebuild controls, storage estimates, and deletion handling. | L |
| Dedicated tag suggestions | Metadata generation already includes tags | **Do not create a separate screen.** Improve the composer metadata card with accept/reject suggestions. | S |
| Dedicated mood suggestions | Local heuristic mood exists; cloud metadata focuses on title/summary | **Integrate as an optional metadata suggestion**, with an explanation and manual override. | S–M |
| Period/tag-filtered summaries | Missing as a guided action | **Implement as “Analysis recipes” in AI.** Let the user select the visible entry scope, inspect it, and run Summary, Themes, Questions, or Changes Over Time. | M |
| Pattern detection | Missing | **Consider.** Prefer explicit, reproducible recipes over an open-ended “find patterns” button. Qualify weak or sparse signals. | M–L |
| Sentiment journey | Analytics has aggregated mood sentiment; AI narrative is missing | **Consider as a Wrapped/AI recipe**, not a separate top-level page. | M |
| AI Time Capsules with trigger dates, due state, read/dismiss/retry | Legacy rows can be recognized during compatibility reads, but there is no full Tauri workflow | **Improve into an opt-in Scheduled Reflection feature.** Let the user choose the source date/window, generation timing, local/cloud provider, and whether generation happens only while the app is open. | L |
| AI thread title/summary | Missing | **Implement in Threads.** | M |
| Save/export AI analysis output | Chat persists, but there is no explicit “save this as an entry/report” flow | **Implement.** Save as a new draft with provenance, provider, and selected source scope. | S–M |
| Automatic background cloud analysis | Not implemented | **Keep it off by default.** Journal text should never be transmitted without a deliberate provider/scope decision. | — |

---

## 10. Gamification and Profile

**Overall status: Partial.** Tauri shows total XP, level, next-level progress, recent XP events, quests, earned badges, and the hero path. Legacy adds more quest management, collector progress, hero choice, and RPG presentation.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Daily/weekly/boss quest grouping | Quests exist but are less structured | **Consider.** Grouping improves scanability if the quest system is retained. | S |
| Claim all completed quests | Only individual claim actions | **Implement.** Low effort and removes repetitive clicks. | XS |
| Preview XP for the next entry and bonuses | Missing | **Consider.** Useful for a game-like system, but can distort authentic journaling. Keep it subtle and optional. | S |
| Locked badges with visible progress | Only earned badges are emphasized | **Implement.** This is the clearest missing motivational loop. Allow hidden “surprise” badges too. | M |
| Hero sprite gallery/equip | Hero path is displayed; selection/equip UI is absent | **Consider if the hero is meaningful to users.** Store the choice locally and avoid coupling it to journal data. | M |
| Enemy sprites and battle presentation | Missing | **Skip as a priority.** It is polish on top of a secondary feature and adds asset/animation maintenance. | L |
| Thousand-word day/entry counters | Some badge logic exists; explicit counters are limited | **Consider inside badge progress**, not as more dashboard metrics. | S |
| Adventure Snapshot and badge case on Dashboard | Missing there | **Keep these in Profile.** | XS |
| Calmer goals/achievements framing | Legacy is strongly RPG-themed | **Consider as an alternative presentation mode.** The same progress data could support either “Adventure” or “Writing Goals” without duplicating logic. | M |

---

## 11. Legacy plugins and idea logs

**Overall status: Missing as native user experiences.** Tauri can inspect legacy compatibility/plugin data, but it does not provide the legacy registry lifecycle or dedicated plugin navigation/screens.

### Plugin lifecycle

| Missing capability | Comment and recommendation | Effort |
|---|---|---|
| Plugin registry/listing | Tauri has compatibility/status reporting, not a native runtime registry | **Do not port the Python plugin runtime directly into Tauri.** Decide first whether Tauri plugins are data-defined collections, native modules, or external integrations. | L–XL |
| Activate/deactivate plugin | Missing | **Replace with enable/disable collection definitions** if the generic framework below is chosen. | Included in L |
| Install/update/update all | Missing | **Skip until there is a signed, versioned Tauri plugin format.** Remote code installation is a security and support commitment. | XL |
| Dynamic navigation from active plugins | Missing | **Implement only for trusted local collection definitions**, not arbitrary downloaded code. | M |

### Dream Log

Legacy Dream Log includes:

- dream entry creation and deletion;
- lucid-dream flag;
- vividness score from 1–5;
- themes;
- text, theme, and lucid filters;
- total dreams, lucid rate, average vividness, and top theme.

**Recommendation: Consider/Implement as the first specialized Collection.** Dreams have a genuinely distinct schema and analytical value. It can validate a generic framework while still feeling purpose-built.

**Effort:** M as a one-off screen; L as the first implementation of a reusable Collections framework.

### Coding Ideas

Legacy Coding Ideas includes:

- name, body, project, and energy;
- Inbox, Next, Doing, Later, and Completed states;
- list and Kanban views;
- drag-to-change status;
- random idea picker;
- attachments;
- edit/delete;
- inbox and completed exports.

**Recommendation: Improve rather than directly port.** This is a small project-management tool inside a journal. A generic Collection with board/list views can support it, but a dedicated coding-ideas subsystem risks competing with existing task tools.

**Effort:** M–L.

### Writing Ideas

Legacy Writing Ideas is a simpler CRUD collection with body, optional name, and optional project.

**Recommendation: Implement as a saved Collection template, not a dedicated page.**

**Effort:** S after the framework; M as a one-off.

### Post Ideas

Legacy Post Ideas includes body, optional title, series flag, Inbox/Completed states, edit/delete, and moving between states.

**Recommendation: Implement as another Collection template.** It can share list/board, state, tag, and export behavior with Coding and Writing Ideas.

**Effort:** S after the framework; M as a one-off.

### Proposed replacement: Collections/Logs

A better Tauri-native replacement for four separate plugins would support:

- named collection definitions;
- field types such as text, long text, number, rating, boolean, date, tags, state, relation, and attachment;
- list, board, and compact analytics views;
- saved filters;
- import/export of a collection definition and its data;
- optional navigation pinning;
- trusted built-in templates: Dream Log, Coding Ideas, Writing Ideas, Post Ideas.

This costs more up front, but prevents four similar CRUD implementations from drifting apart.

---

## 12. Security and encrypted journals

**Overall status: Missing.** Legacy Capsule supports AES and SQLCipher at-rest modes, lock/unlock sessions, keyfile/keyring status, and degraded feature flags. Tauri currently detects and reports security/readability state but does not provide an encrypted-journal workflow.

| Missing capability | Comment and recommendation | Effort |
|---|---|---|
| SQLCipher database support | **Implement only if encrypted legacy databases must open directly or application-level encryption is a product requirement.** It affects SQLite builds, migrations, backups, recovery, sync, tests, and every supported platform. | XL |
| Bespoke AES storage mode | **Skip unless required for legacy recovery.** Supporting two encryption architectures doubles migration and failure modes. Prefer SQLCipher for a single coherent database. | XL |
| Lock/unlock session UI | **Required if app-level encryption is added.** Include idle lock, failed-attempt behavior, key recovery guidance, and no plaintext previews while locked. | L within encryption project |
| Keyfile/keyring management | Tauri already uses the OS credential store for AI keys, not journal encryption | **Use OS key storage as an option, never the only recovery path.** The user must understand what happens if OS credentials are lost. | L |
| Encryption migration and rollback | Missing | **Mandatory before exposing an Enable Encryption button.** Make a verified backup, dry-run compatibility checks, and preserve a documented recovery route. | L |

**Near-term recommendation:** document OS full-disk encryption as the supported baseline. Treat SQLCipher as a separate, security-reviewed project rather than a normal parity ticket.

---

## 13. Data portability, imports, and integrations

**Overall status: Partial.** Tauri exports Markdown and JSON. Legacy supports more formats and several migration/integration paths.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| CSV export | **Implement.** Useful for analysis and low risk when field escaping and encoding are tested. | S |
| Markdown vault export | **Implement.** One file per entry, stable filenames, front matter, copied attachments, and optional wiki links make Capsule work well with Obsidian/Logseq without continuous sync. | M |
| PDF export | **Consider.** Good for sharing and archival, but layout, fonts, page breaks, images, and very long journals require care. Start with selected entries, not the entire database. | M |
| DOCX export | **Consider after PDF/Markdown vault.** Useful for editing outside Capsule but not core to local-first journaling. | M |
| Generic JSON import | **Implement only with preview, validation, duplicate handling, and automatic backup.** Import is much riskier than export. | L |
| Legacy/Flutter bundle preview and import | Missing | **Consider only if an active migration need remains.** Put format-specific adapters behind one import wizard. | M–L |
| Obsidian/Logseq continuous vault sync | Missing | **Prefer repeatable export before two-way sync.** Two-way file/database reconciliation creates identity and conflict problems. | L–XL |
| Notion sync | Missing | **Consider as an optional integration, not core parity.** It introduces API credentials, rate limits, schema mapping, and ongoing maintenance. | L |
| Reusable template/library import/export | Missing | **Implement for templates and Collection definitions.** Keep executable code out of portable libraries. | S–M |
| Global operation history and undo | Tauri has entry edit snapshots, but no journal-wide operation log | **Do not port directly.** Start with per-entry restore. Global undo becomes dangerous across sync, images, threads, imports, and deletes. | L–XL |
| Export format chooser | Separate actions are limited | **Implement as one scoped export dialog** rather than adding more toolbar buttons. | S |

---

## 14. Location and weather management

**Overall status: Partial.** Tauri captures automatic IP or fixed location/weather metadata and can display, filter, and summarize it. Legacy has more correction and exploration tools.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Manually attach or change an entry location | **Implement.** Autodetection can be wrong, and historical entries may need correction. Search by place name and preview coordinates before saving. | M |
| Remove location/weather from an entry | **Implement.** Metadata should always be correctable and removable. | S |
| Refresh weather for an existing entry | **Implement with safeguards.** Use the entry timestamp and coordinates, show the provider/source, and do not silently replace manually edited values. | M |
| Forward and reverse geocoding controls | Mostly hidden behind automatic capture | **Expose only inside the location editor.** No need for a separate utility page. | M |
| Nearby entries | Missing | **Consider.** A “Memories near here” view could be valuable, but exact-location privacy and distance accuracy need clear handling. | M |
| Location browser and per-location stats | Simple Analytics counts and filters exist | **Improve through clickable location analytics** before building a separate page. | S–M |
| Full weather analytics | Partial | Covered in the Analytics section. | M–L |

---

## 15. Backup behavior and scheduled safety

**Overall status: Mostly replaced/improved.** Tauri creates backups before mutations, provides manual backup/restore, retains multiple backups, validates paths, and gives clear safety status. That is stronger than a simple timer for the most dangerous moments.

| Missing capability | Comment and recommendation | Effort |
|---|---|---|
| Scheduled idle/daily backups | **Consider as an additional layer.** Run only when the database is readable and no write/import/restore is in progress. Keep pre-mutation backups. | S–M |
| Automatic-backup enable/disable control | Pre-mutation backup behavior is safety-critical | **Do not make safety backups easy to disable.** A separate scheduled-backup toggle is fine. | S |
| Custom backup folder UI parity | Tauri resolves its backup directory but has less legacy-style scheduling configuration | **Consider only if users need a second disk/cloud folder.** Validate against writing into the live database directory. | M |

---

## 16. Sync, mobile companions, and capture

**Overall status: Partial/replaced.** Tauri has a modern shared-folder/Gist sync workflow with conflict review and mobile-note ingestion. Legacy also maintains companion-facing helpers and a separate quick-capture utility.

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Publish `tags_catalog.json` for companion tag suggestions | No matching Tauri implementation was found | **Implement if the existing mobile/Shortcut companions still consume it.** Derive it from normalized tags during sync and version the schema. | S–M |
| Explicit one-click mobile queue import | Tauri imports mobile notes through sync | **Keep the unified sync path.** Add a visible “N mobile notes waiting” state rather than a second import engine. | S |
| Companion setup wizard | Gist/shared-folder settings exist, but legacy has more guided companion configuration | **Consider.** Generate copyable IDs/paths and test connectivity without exposing secrets. | M |
| Connect/disconnect semantics for cloud sync | Tauri uses direct Gist ID/token settings | **Improve** with a clearer configured/connected/error state and a destructive disconnect confirmation that does not delete remote data. | S–M |
| Compact quick-capture popover | Global `Ctrl+Alt+W` Writer and tray entry replace most of it | **Consider only if Writer startup or size is a friction point.** A small secondary window is a native desktop improvement, but the current hotkey already covers the need. | M |
| iOS Shortcuts, mobile PWA, and Wear OS apps inside Tauri | Not present | **Keep separate.** Maintain protocol compatibility and setup documentation instead. | — |

---

## 17. Profiles, keyboard navigation, and utility surfaces

| Missing or partial capability | Comment and recommendation | Effort |
|---|---|---|
| Multiple local profiles/journals | Tauri selects one active database path | **Consider a Vault Switcher instead of recreating CLI profiles.** Each vault should retain its own database, backup, sync, and appearance settings. | L |
| Command palette | Legacy includes a command-palette component; Tauri has no comparable global launcher | **Implement for keyboard-centric use.** Include navigation, New Entry, Writer, Search, Backup, Sync, theme, and recent entries. | S–M |
| Exposed Debug page | Tauri has internal debug-related code but no normal navigation item in the tested production shell | **Keep it hidden behind a developer toggle or About diagnostics.** It should not be normal navigation. | XS |
| Browser/web deployment | Tauri is a desktop application | **Keep separate.** Do not compromise native safety and filesystem behavior to recreate the FastAPI web deployment in this codebase. | — |
| Typer CLI commands | Not part of the Tauri UI | **Keep separate and interoperable.** Shared database/schema compatibility is more valuable than embedding a terminal feature set. | — |

---

## 18. Features the Tauri app already improves

These should be protected during any parity work:

- native tray integration and a global Writer hotkey;
- start-at-login, desktop window restoration, and updater behavior;
- direct Rust database access with explicit path safety;
- backups before mutations and a clear restore-review flow;
- explicit sync preview/conflict handling and the newer sync protocol;
- AI provider keys stored in the OS credential store rather than plain settings;
- persistent streaming AI conversations with cancel and retry;
- richer Wrapped scopes: week, month, and year;
- sentiment markers in the dedicated writing calendar;
- integrated image picking, preview, and native filesystem handling;
- multiple polished application themes;
- one coherent desktop shell instead of separate web/CLI control surfaces.

Parity work should reuse these strengths rather than importing legacy architectural assumptions.

## Recommended implementation sequence

### Wave 1: finish existing flows

1. Safe Markdown renderer shared across all entry reading surfaces.
2. Starred/pinned quick filters and Saved Views.
3. Searchable continuation picker.
4. On This Day dashboard card.
5. Writing Calendar day drill-down and annual summary.
6. Composer templates and optional prompts.
7. Bulk thread builder plus AI thread title/summary.
8. Per-entry revision restore.
9. Manual location edit/remove/weather refresh.

These are mostly S–M changes with limited schema risk and visible everyday value.

### Wave 2: useful analytical depth

1. Selected/lifetime reading time and monthly/yearly reading trend.
2. Reliable capture-source breakdown.
3. Interactive word-frequency explorer.
4. Mood by time/day with sample counts.
5. Click-through mood, tag, location, and weather distributions.
6. CSV and Markdown-vault export.

This wave expands what is already working without rebuilding the legacy Analytics page wholesale.

### Wave 3: product choices

1. Generic Collections/Logs framework and Dream Log template.
2. Local related-entry discovery.
3. Semantic/hybrid search with a documented privacy model.
4. Analysis recipes and opt-in Scheduled Reflections.
5. Gamification choice: deepen Adventure mode or offer calmer Writing Goals.
6. Vault switching if multiple journals are a real use case.

### Separate projects requiring design and risk review

- SQLCipher encrypted journals and migration;
- broad import/migration support;
- two-way external vault or Notion sync;
- global operation undo;
- signed downloadable native plugins.

## Things I would explicitly avoid porting as-is

- a separate Starred page when a saved filter does the job;
- Calendar and Wrapped embedded again inside Analytics;
- literal tag and mood clouds when readable, clickable distributions are better;
- an unexplained Health Score;
- four unrelated CRUD implementations for the four legacy idea/log plugins;
- a second sync/import engine just for mobile notes;
- bespoke AES encryption alongside SQLCipher;
- global undo before safe per-entry revision restore;
- battle sprites and enemy presentation ahead of core journal improvements;
- silent background cloud analysis of journal contents;
- a full WYSIWYG rewrite before safe Markdown rendering and preview are solved.

## Visual, UX, and accessibility observations

- The Tauri shell has a clearer visual hierarchy than the legacy dashboard and Analytics page. Porting all legacy cards would make the current app materially harder to scan.
- The Tauri activity chart selector uses an explicit tablist and descriptive chart labels. New analytical modules should preserve that pattern and also provide a short text summary or data table.
- Mood, weather, and heatmap information must not rely on color alone. Counts, labels, marker shapes, and tooltips should remain available for color-vision and low-vision users.
- Raw comma-separated filters and the continuation UUID field are technically compact but easy to misunderstand. Token pickers and a searchable entry chooser improve accessibility and prevent input errors at the same time.
- The sidebar is already long. Adding four plugin pages and several legacy utility pages directly to navigation would make keyboard and pointer travel worse; Saved Views, a command palette, and optional pinned Collections are better.
- Icon-only actions should have stable accessible names in addition to hover titles. New restore, location, thread, and chart actions should follow this rule from the start.
- Large analytics and Cover Wall views should retain logical heading order, keyboard focus visibility, and non-hover paths to drill-down information.

## Audit evidence and limitations

### Evidence used

- Current Tauri navigation, views, commands, models, and tests in `src/App.tsx`, `src/backend.ts`, and `src-tauri/src/`.
- Current product documentation in `README.md`, `CHANGELOG.md`, and `SPEC.md`.
- Legacy web routes and React components in `capsule-web/frontend/src/`.
- Legacy FastAPI endpoints and core services in `capsule-web/backend/app/` and `capsule_exp/`.
- Legacy plugin manifests, routes, and UI components under the plugin directories.
- Live visual verification of both apps' Dashboard, Analytics, and New Entry/Composer surfaces on 2026-07-23.

### Audit procedure

1. Inventory both repositories and their top-level user-facing routes.
2. Map current Tauri views to legacy pages and plugins.
3. Compare depth inside matched sections rather than treating a matching page name as parity.
4. Check Tauri backend commands for capabilities that exist without UI.
5. Inspect representative live screens in both apps.
6. Classify each gap by value, implementation effort, risk, and whether a different Tauri-native solution is better.

### Limitations

- “Missing” means no complete user-facing implementation was found. A schema field, compatibility reader, or backend command alone is labeled backend-ready or partial.
- The visual pass used the current browser-compatible Tauri backend and the legacy production web build. Native-only behavior was assessed from code and documentation.
- Screenshots are intentionally not embedded in this repository report because the live legacy UI contains private journal content.
- The legacy app has configuration-dependent plugins and feature flags. Dormant but implemented plugin code is included where it represents a real legacy capability.
- This is a parity and product-value audit, not a test report or implementation specification. High-risk items need separate technical designs before work begins.
