import { invoke } from "@tauri-apps/api/core";
import type {
  BackupCreateRequest,
  BackupCreateResponse,
  BackupListResponse,
  DatabaseStatus,
  Entry,
  EntryCreate,
  EntryFilters,
  EntryHistoryResponse,
  EntryListResponse,
  EntryMutationResponse,
  EntryUpdate,
  RandomEntryFilters,
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const runningInTauri = () =>
  typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);

const mockStatus: DatabaseStatus = {
  dbPath: "C:\\Users\\jtill\\.capsule\\capsule.db",
  dbExists: true,
  dbSizeBytes: 110_792_704,
  dbModifiedAt: "2026-06-29T12:43:21Z",
  readable: true,
  schemaSummary: {
    tableCount: 8,
    detectedTables: ["entries", "tags", "entry_tags", "entries_fts"],
    hasEntriesTable: true,
    hasTagsTable: true,
    hasFtsTable: true,
    missingCoreTables: [],
  },
  entryCount: 608,
  tagCount: 792,
  backupCount: 3,
  lastBackupPath: "C:\\Users\\jtill\\.capsule\\capsule_backup_20260629_120000.db",
  security: {
    mode: "plain",
    locked: false,
    readable: true,
    message: null,
  },
  warnings: [],
};

const mockBackups: BackupListResponse = {
  backupDirectory: "C:\\Users\\jtill\\.capsule",
  backups: [
    {
      path: "C:\\Users\\jtill\\.capsule\\capsule_backup_20260629_120000.db",
      manifestPath: "C:\\Users\\jtill\\.capsule\\capsule_backup_20260629_120000.json",
      createdAt: "2026-06-29T12:00:00Z",
      sizeBytes: 110_780_416,
      operation: "manual",
      verified: true,
    },
  ],
};

let mockEntries: Entry[] = [
  {
    id: 608,
    uuid: "entry_ti99r1ya",
    createdAt: "2026-06-29 12:40",
    updatedAt: "2026-06-29 12:40",
    text: "Finished the first read-only pass for Capsule Tauri. The important part is that the app can sit beside the real journal without touching it, and still feel fast enough to browse.",
    textPlain:
      "Finished the first read-only pass for Capsule Tauri. The important part is that the app can sit beside the real journal without touching it, and still feel fast enough to browse.",
    contentFormat: "markdown",
    title: "Phase 1 shape",
    summary: "Read-only journal browsing starts to feel real.",
    mood: "excited",
    moodInfo: { name: "excited", label: "Excited" },
    tags: [
      { id: 1, name: "capsule" },
      { id: 2, name: "tauri" },
      { id: 3, name: "rust" },
    ],
    starred: false,
    pinned: false,
    hidden: false,
    location: {
      latitude: 69.65,
      longitude: 18.96,
      placeName: "Utsikten, Tromso, Norway",
      weatherCondition: "Overcast",
      weatherTempC: 8,
      weatherTempF: 46.4,
    },
    thread: null,
    attachmentCount: 0,
  },
  {
    id: 607,
    uuid: "entry_oiuir59w",
    createdAt: "2026-06-28 21:15",
    updatedAt: "2026-06-28 21:15",
    text: "A small note about the Codex workflow and how much easier the desktop version should make database safety feel.",
    textPlain:
      "A small note about the Codex workflow and how much easier the desktop version should make database safety feel.",
    contentFormat: "plain",
    title: null,
    summary: null,
    mood: "focused",
    moodInfo: { name: "focused", label: "Focused" },
    tags: [
      { id: 4, name: "work" },
      { id: 5, name: "codex" },
    ],
    starred: false,
    pinned: false,
    hidden: false,
    location: null,
    thread: {
      rootUuid: "entry_oiuir59w",
      parentUuid: null,
      title: "Desktop Capsule work",
      summary: "Notes about bringing Capsule into a local-first desktop app.",
      entryCount: 3,
      isRoot: true,
    },
    attachmentCount: 2,
  },
  {
    id: 606,
    uuid: "entry_kree51ux",
    createdAt: "2026-06-27 00:12",
    updatedAt: "2026-06-27 00:12",
    text: "A late-night art experiment with a note about what should become searchable later.",
    textPlain: "A late-night art experiment with a note about what should become searchable later.",
    contentFormat: "markdown",
    title: "Art note",
    summary: null,
    mood: "happy",
    moodInfo: { name: "happy", label: "Happy" },
    tags: [
      { id: 5, name: "codex" },
      { id: 6, name: "art" },
    ],
    starred: false,
    pinned: false,
    hidden: false,
    location: null,
    thread: null,
    attachmentCount: 1,
  },
  {
    id: 605,
    uuid: "entry_2490ytiy",
    createdAt: "2026-06-26 09:59",
    updatedAt: "2026-06-29 06:08",
    text: "Music backup cleanup and a couple of Rust details worth carrying forward.",
    textPlain: "Music backup cleanup and a couple of Rust details worth carrying forward.",
    contentFormat: "markdown",
    title: null,
    summary: "Notes from music backup work.",
    mood: "calm",
    moodInfo: { name: "calm", label: "Calm" },
    tags: [
      { id: 7, name: "music-backup" },
      { id: 3, name: "rust" },
    ],
    starred: false,
    pinned: false,
    hidden: false,
    location: null,
    thread: null,
    attachmentCount: 0,
  },
];

const pause = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));

const normalizeError = (error: unknown): Error => {
  if (error instanceof Error) {
    return error;
  }

  if (typeof error === "string") {
    return new Error(error);
  }

  return new Error("Unexpected Capsule backend error");
};

export async function getDatabaseStatus(): Promise<DatabaseStatus> {
  try {
    if (runningInTauri()) {
      return await invoke<DatabaseStatus>("get_database_status");
    }

    await pause(150);
    return mockStatus;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listBackups(): Promise<BackupListResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<BackupListResponse>("list_backups");
    }

    await pause(150);
    return mockBackups;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function createBackup(
  input: BackupCreateRequest = {},
): Promise<BackupCreateResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<BackupCreateResponse>("create_backup", { input });
    }

    await pause(400);
    const createdAt = new Date().toISOString();
    const stamp = createdAt
      .replace(/[-:]/g, "")
      .replace(/\.\d{3}Z$/, "")
      .replace("T", "_");
    return {
      backup: {
        path: `C:\\Users\\jtill\\.capsule\\capsule_backup_${stamp}.db`,
        manifestPath: `C:\\Users\\jtill\\.capsule\\capsule_backup_${stamp}.json`,
        createdAt,
        sizeBytes: mockStatus.dbSizeBytes,
        operation: input.operation ?? "manual",
        verified: true,
      },
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listEntries(filters: EntryFilters = {}): Promise<EntryListResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<EntryListResponse>("list_entries", { filters });
    }

    await pause(180);
    const filtered = applyMockFilters(mockEntries, filters);
    const offset = filters.offset ?? 0;
    const limit = filters.limit ?? 40;
    return {
      entries: filtered.slice(offset, offset + limit),
      total: filtered.length,
      limit,
      offset,
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getEntry(identifier: string): Promise<Entry> {
  try {
    if (runningInTauri()) {
      return await invoke<Entry>("get_entry", { identifier });
    }

    await pause(120);
    const entry = mockEntries.find(
      (item) => item.uuid === identifier || String(item.id) === identifier,
    );
    if (!entry) {
      throw new Error(`Entry not found: ${identifier}`);
    }
    return entry;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getRandomEntry(filters: RandomEntryFilters = {}): Promise<Entry | null> {
  try {
    if (runningInTauri()) {
      return await invoke<Entry | null>("get_random_entry", { filters });
    }

    await pause(120);
    const filtered = applyMockFilters(mockEntries, {
      includeHidden: filters.includeHidden,
      tags: filters.tags,
      moods: filters.moods,
    });
    if (filtered.length === 0) {
      return null;
    }
    return filtered[Math.floor(Math.random() * filtered.length)];
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function createEntry(input: EntryCreate): Promise<EntryMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<EntryMutationResponse>("create_entry", { input });
    }

    await pause(320);
    const createdAt = (input.when?.trim() || new Date().toISOString().slice(0, 16)).replace("T", " ");
    const nextId = Math.max(0, ...mockEntries.map((entry) => entry.id)) + 1;
    const entry: Entry = {
      id: nextId,
      uuid: `entry_mock${nextId.toString(36).padStart(4, "0")}`,
      createdAt,
      updatedAt: createdAt,
      text: input.text,
      textPlain: toTextPlain(input.text),
      contentFormat: input.contentFormat ?? "markdown",
      title: normalizeNullable(input.title),
      summary: normalizeNullable(input.summary),
      mood: normalizeNullable(input.mood),
      moodInfo: {
        name: normalizeNullable(input.mood),
        label: normalizeNullable(input.mood) ? labelize(normalizeNullable(input.mood) ?? "") : null,
      },
      tags: normalizeTags(input.tags).map((name, index) => ({ id: 10_000 + nextId + index, name })),
      starred: input.starred ?? false,
      pinned: input.pinned ?? false,
      hidden: false,
      location: null,
      thread: input.continueFromUuid
        ? {
            rootUuid: input.continueFromUuid,
            parentUuid: input.continueFromUuid,
            title: null,
            summary: null,
            entryCount: 2,
            isRoot: false,
          }
        : null,
      attachmentCount: 0,
    };
    mockEntries = [entry, ...mockEntries];
    return { entry, audit: mockAudit("entry.create") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function updateEntry(
  identifier: string,
  input: EntryUpdate,
): Promise<EntryMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<EntryMutationResponse>("update_entry", { identifier, input });
    }

    await pause(300);
    const index = mockEntries.findIndex(
      (entry) => entry.uuid === identifier || String(entry.id) === identifier,
    );
    if (index === -1) {
      throw new Error(`Entry not found: ${identifier}`);
    }
    const current = mockEntries[index];
    const nextMood = input.mood === undefined ? current.mood : normalizeNullable(input.mood);
    const updated: Entry = {
      ...current,
      text: input.text ?? current.text,
      textPlain: input.text === undefined ? current.textPlain : toTextPlain(input.text),
      contentFormat: input.contentFormat ?? current.contentFormat,
      title: input.title === undefined ? current.title : normalizeNullable(input.title),
      summary: input.summary === undefined ? current.summary : normalizeNullable(input.summary),
      mood: nextMood,
      moodInfo: { name: nextMood, label: nextMood ? labelize(nextMood) : null },
      tags:
        input.tags === undefined
          ? current.tags
          : normalizeTags(input.tags).map((name, tagIndex) => ({
              id: 20_000 + current.id + tagIndex,
              name,
            })),
      starred: input.starred ?? current.starred,
      pinned: input.pinned ?? current.pinned,
      hidden: input.hidden ?? current.hidden,
      updatedAt: new Date().toISOString(),
      thread:
        input.continueFromUuid === undefined
          ? current.thread
          : input.continueFromUuid
            ? {
                rootUuid: input.continueFromUuid,
                parentUuid: input.continueFromUuid,
                title: null,
                summary: null,
                entryCount: 2,
                isRoot: false,
              }
            : null,
    };
    mockEntries = mockEntries.map((entry, entryIndex) => (entryIndex === index ? updated : entry));
    return { entry: updated, audit: mockAudit("entry.update") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function starEntry(identifier: string): Promise<EntryMutationResponse> {
  return setEntryFlag(identifier, "starred", true, "entry.star");
}

export async function unstarEntry(identifier: string): Promise<EntryMutationResponse> {
  return setEntryFlag(identifier, "starred", false, "entry.unstar");
}

export async function pinEntry(identifier: string): Promise<EntryMutationResponse> {
  return setEntryFlag(identifier, "pinned", true, "entry.pin");
}

export async function unpinEntry(identifier: string): Promise<EntryMutationResponse> {
  return setEntryFlag(identifier, "pinned", false, "entry.unpin");
}

export async function hideEntry(identifier: string): Promise<EntryMutationResponse> {
  return setEntryFlag(identifier, "hidden", true, "entry.hide");
}

export async function unhideEntry(identifier: string): Promise<EntryMutationResponse> {
  return setEntryFlag(identifier, "hidden", false, "entry.unhide");
}

export async function listEntryHistory(identifier: string): Promise<EntryHistoryResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<EntryHistoryResponse>("list_entry_history", { identifier });
    }

    await pause(140);
    const entry = mockEntries.find(
      (item) => item.uuid === identifier || String(item.id) === identifier,
    );
    if (!entry) {
      throw new Error(`Entry not found: ${identifier}`);
    }
    return {
      entryId: entry.id,
      current: {
        id: entry.id,
        uuid: entry.uuid,
        text: entry.text,
        title: entry.title,
        summary: entry.summary,
        mood: entry.mood,
        tags: entry.tags.map((tag) => tag.name),
      },
      history: [],
      count: 0,
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

async function setEntryFlag(
  identifier: string,
  flag: "starred" | "pinned" | "hidden",
  value: boolean,
  operation: string,
): Promise<EntryMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<EntryMutationResponse>(operationToCommand(operation), { identifier });
    }

    await pause(180);
    const index = mockEntries.findIndex(
      (entry) => entry.uuid === identifier || String(entry.id) === identifier,
    );
    if (index === -1) {
      throw new Error(`Entry not found: ${identifier}`);
    }
    const entry = {
      ...mockEntries[index],
      [flag]: value,
      updatedAt: new Date().toISOString(),
    };
    mockEntries = mockEntries.map((item, itemIndex) => (itemIndex === index ? entry : item));
    return { entry, audit: mockAudit(operation) };
  } catch (error) {
    throw normalizeError(error);
  }
}

function operationToCommand(operation: string) {
  return {
    "entry.star": "star_entry",
    "entry.unstar": "unstar_entry",
    "entry.pin": "pin_entry",
    "entry.unpin": "unpin_entry",
    "entry.hide": "hide_entry",
    "entry.unhide": "unhide_entry",
  }[operation] ?? operation;
}

function applyMockFilters(entries: Entry[], filters: EntryFilters) {
  const text = filters.text?.trim().toLowerCase();
  const tagSet = new Set(filters.tags?.map((tag) => tag.trim().toLowerCase()).filter(Boolean));
  const moodSet = new Set(filters.moods?.map((mood) => mood.trim().toLowerCase()).filter(Boolean));

  return entries
    .filter((entry) => {
      if (!filters.includeHidden && !filters.hidden && entry.hidden) {
        return false;
      }
      if (filters.hidden !== undefined && filters.hidden !== null && entry.hidden !== filters.hidden) {
        return false;
      }
      if (filters.starred !== undefined && filters.starred !== null && entry.starred !== filters.starred) {
        return false;
      }
      if (filters.pinned !== undefined && filters.pinned !== null && entry.pinned !== filters.pinned) {
        return false;
      }
      if (filters.hasImages === true && entry.attachmentCount === 0) {
        return false;
      }
      if (filters.hasImages === false && entry.attachmentCount > 0) {
        return false;
      }
      if (text) {
        const haystack = `${entry.textPlain} ${entry.title ?? ""} ${entry.summary ?? ""}`.toLowerCase();
        if (!haystack.includes(text)) {
          return false;
        }
      }
      if (tagSet.size > 0) {
        const entryTags = new Set(entry.tags.map((tag) => tag.name.toLowerCase()));
        for (const tag of tagSet) {
          if (!entryTags.has(tag)) {
            return false;
          }
        }
      }
      if (moodSet.size > 0 && (!entry.mood || !moodSet.has(entry.mood.toLowerCase()))) {
        return false;
      }
      if (filters.since && new Date(entry.createdAt) < new Date(filters.since)) {
        return false;
      }
      if (filters.until && new Date(entry.createdAt) > new Date(filters.until)) {
        return false;
      }
      return true;
    })
    .sort((left, right) => {
      const leftTime = new Date(left.createdAt).getTime();
      const rightTime = new Date(right.createdAt).getTime();
      return filters.sort === "asc" ? leftTime - rightTime : rightTime - leftTime;
    });
}

function mockAudit(operation: string) {
  const completedAt = new Date().toISOString();
  const stamp = completedAt
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "")
    .replace("T", "_");
  return {
    backupPath: `C:\\Users\\jtill\\.capsule\\capsule_backup_${stamp}.db`,
    operation,
    completedAt,
  };
}

function toTextPlain(text: string) {
  return text.split(/\s+/).filter(Boolean).join(" ");
}

function normalizeNullable(value: string | null | undefined) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function normalizeTags(tags: string[] | undefined) {
  const seen = new Set<string>();
  const normalized: string[] = [];
  for (const tag of tags ?? []) {
    const value = tag.trim().toLowerCase();
    if (value && !seen.has(value)) {
      seen.add(value);
      normalized.push(value);
    }
  }
  return normalized;
}

function labelize(value: string) {
  return value
    .split(/[-_]/)
    .filter(Boolean)
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(" ");
}
