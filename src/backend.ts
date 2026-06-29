import { invoke } from "@tauri-apps/api/core";
import type {
  BackupCreateRequest,
  BackupCreateResponse,
  BackupListResponse,
  DatabaseStatus,
  Entry,
  EntryFilters,
  EntryListResponse,
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

const mockEntries: Entry[] = [
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
