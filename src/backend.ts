import { invoke } from "@tauri-apps/api/core";
import type {
  BackupCreateRequest,
  BackupCreateResponse,
  BackupListResponse,
  BackupRestorePreview,
  BackupRestorePreviewRequest,
  BackupRestoreRequest,
  BackupRestoreResponse,
  CapsuleConfigResponse,
  ConfigMutationResponse,
  DatabaseStatus,
  Entry,
  EntryCreate,
  EntryFilters,
  EntryHistoryResponse,
  EntryListResponse,
  EntryMutationResponse,
  EntryUpdate,
  ExportEntriesRequest,
  ExportEntriesResponse,
  LibraryListResponse,
  LibraryPromptInput,
  LibraryPromptMutationResponse,
  LibraryPromptUpdate,
  LibraryTemplateInput,
  LibraryTemplateMutationResponse,
  LibraryTemplateUpdate,
  MoodCatalogResponse,
  MoodDeleteRequest,
  MoodMutationResponse,
  MoodRenameRequest,
  RandomEntryFilters,
  SearchRequest,
  SearchResponse,
  TagCatalogResponse,
  TagDeleteRequest,
  TagMergeRequest,
  TagMutationResponse,
  TagRenameRequest,
  ThreadGroup,
  ThreadListResponse,
  ThreadMetadataUpdate,
  ThreadMutationResponse,
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

let mockConfig: CapsuleConfigResponse = {
  configPath: "C:\\Users\\jtill\\.capsule\\config.json",
  exists: true,
  values: [
    { key: "backup_count", value: "3" },
    { key: "theme", value: "system" },
  ],
  warnings: [],
};

let mockTags: TagCatalogResponse = {
  tags: [
    { id: 1, name: "capsule", entryCount: 1 },
    { id: 2, name: "tauri", entryCount: 1 },
    { id: 3, name: "rust", entryCount: 2 },
    { id: 4, name: "work", entryCount: 1 },
    { id: 5, name: "codex", entryCount: 2 },
    { id: 6, name: "art", entryCount: 1 },
  ],
  warnings: [],
};

let mockMoods: MoodCatalogResponse = {
  moods: [
    { name: "calm", label: "Calm", entryCount: 1 },
    { name: "excited", label: "Excited", entryCount: 1 },
    { name: "focused", label: "Focused", entryCount: 1 },
    { name: "happy", label: "Happy", entryCount: 1 },
  ],
  warnings: [],
};

let mockLibrary: LibraryListResponse = {
  templates: [
    {
      id: 1,
      slug: "weekly-review",
      name: "Weekly Review",
      description: "End-of-week review with momentum planning.",
      introText: "",
      sections: ["## Highlights", "## Challenges", "## Lessons", "## Next week focus"],
      isBuiltin: true,
      isActive: true,
      createdAt: "2026-06-29 12:00",
      updatedAt: "2026-06-29 12:00",
    },
  ],
  prompts: [
    {
      id: 1,
      slug: "surprise_today",
      promptText: "What's one thing that surprised you today?",
      category: "reflection",
      tags: ["daily", "reflection"],
      isBuiltin: true,
      isActive: true,
      createdAt: "2026-06-29 12:00",
      updatedAt: "2026-06-29 12:00",
    },
  ],
  warnings: [],
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
    thread: {
      rootUuid: "entry_oiuir59w",
      parentUuid: "entry_oiuir59w",
      title: "Desktop Capsule work",
      summary: "Notes about bringing Capsule into a local-first desktop app.",
      entryCount: 3,
      isRoot: false,
    },
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
    thread: {
      rootUuid: "entry_oiuir59w",
      parentUuid: "entry_kree51ux",
      title: "Desktop Capsule work",
      summary: "Notes about bringing Capsule into a local-first desktop app.",
      entryCount: 3,
      isRoot: false,
    },
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

export async function previewRestoreBackup(
  input: BackupRestorePreviewRequest,
): Promise<BackupRestorePreview> {
  try {
    if (runningInTauri()) {
      return await invoke<BackupRestorePreview>("preview_restore_backup", { input });
    }

    await pause(180);
    const backup = mockBackups.backups.find((item) => item.path === input.backupPath);
    if (!backup) {
      throw new Error(`Backup not found: ${input.backupPath}`);
    }
    return {
      backup,
      dbSizeBytes: backup.sizeBytes,
      dbModifiedAt: backup.createdAt,
      schemaSummary: mockStatus.schemaSummary,
      entryCount: mockEntries.length,
      tagCount: mockTags.tags.length,
      warnings: [],
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function restoreBackup(
  input: BackupRestoreRequest,
): Promise<BackupRestoreResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<BackupRestoreResponse>("restore_backup", { input });
    }

    await pause(500);
    if (input.confirmation !== "RESTORE") {
      throw new Error("Restore confirmation must be RESTORE.");
    }
    const restoredFrom = mockBackups.backups.find((item) => item.path === input.backupPath);
    if (!restoredFrom) {
      throw new Error(`Backup not found: ${input.backupPath}`);
    }
    const safetyBackup = (await createBackup({ operation: "backup.restore.safety" })).backup;
    return {
      restoredFrom,
      safetyBackup,
      completedAt: new Date().toISOString(),
      status: mockStatus,
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function openBackupFolder(): Promise<void> {
  try {
    if (runningInTauri()) {
      await invoke<void>("open_backup_folder");
      return;
    }

    await pause(100);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getCapsuleConfig(): Promise<CapsuleConfigResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<CapsuleConfigResponse>("get_capsule_config");
    }

    await pause(120);
    return mockConfig;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function setCapsuleConfigValue(
  key: string,
  value: string,
): Promise<ConfigMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ConfigMutationResponse>("set_capsule_config_value", { key, value });
    }

    await pause(180);
    const nextValues = mockConfig.values.filter((item) => item.key !== key);
    nextValues.push({ key, value });
    mockConfig = { ...mockConfig, exists: true, values: nextValues.sort((a, b) => a.key.localeCompare(b.key)) };
    return {
      config: mockConfig,
      backupPath: "C:\\Users\\jtill\\.capsule\\config_backup_20260629_120000.json",
      operation: "config.set",
      completedAt: new Date().toISOString(),
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function deleteCapsuleConfigValue(
  key: string,
): Promise<ConfigMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ConfigMutationResponse>("delete_capsule_config_value", { key });
    }

    await pause(180);
    mockConfig = { ...mockConfig, values: mockConfig.values.filter((item) => item.key !== key) };
    return {
      config: mockConfig,
      backupPath: "C:\\Users\\jtill\\.capsule\\config_backup_20260629_120000.json",
      operation: "config.delete",
      completedAt: new Date().toISOString(),
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listTags(): Promise<TagCatalogResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<TagCatalogResponse>("list_tags");
    }

    await pause(120);
    return mockTags;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function renameTag(input: TagRenameRequest): Promise<TagMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<TagMutationResponse>("rename_tag", { input });
    }

    await pause(220);
    const from = input.from.trim().toLowerCase();
    const to = input.to.trim().toLowerCase();
    if (!from || !to) {
      throw new Error("Both tag names are required.");
    }
    if (mockTags.tags.some((tag) => tag.name === to)) {
      throw new Error(`Tag '${to}' already exists. Use merge instead.`);
    }
    mockTags = {
      ...mockTags,
      tags: mockTags.tags
        .map((tag) => (tag.name === from ? { ...tag, name: to } : tag))
        .sort((a, b) => a.name.localeCompare(b.name)),
    };
    mockEntries = mockEntries.map((entry) => ({
      ...entry,
      tags: entry.tags.map((tag) => (tag.name === from ? { ...tag, name: to } : tag)),
    }));
    return { tags: mockTags.tags, audit: mockAudit("tag.rename") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function mergeTag(input: TagMergeRequest): Promise<TagMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<TagMutationResponse>("merge_tag", { input });
    }

    await pause(240);
    const source = input.source.trim().toLowerCase();
    const target = input.target.trim().toLowerCase();
    if (!source || !target) {
      throw new Error("Both tag names are required.");
    }
    const sourceTag = mockTags.tags.find((tag) => tag.name === source);
    if (!sourceTag) {
      throw new Error(`Tag not found: ${source}`);
    }
    const targetTag = mockTags.tags.find((tag) => tag.name === target) ?? {
      id: Math.max(0, ...mockTags.tags.map((tag) => tag.id)) + 1,
      name: target,
      entryCount: 0,
    };
    mockEntries = mockEntries.map((entry) => {
      const hasSource = entry.tags.some((tag) => tag.name === source);
      if (!hasSource) {
        return entry;
      }
      const tags = entry.tags.filter((tag) => tag.name !== source);
      if (!tags.some((tag) => tag.name === target)) {
        tags.push({ id: targetTag.id, name: target });
      }
      return { ...entry, tags };
    });
    mockTags = rebuildMockTagUsage(mockTags.tags.filter((tag) => tag.name !== source || tag.name === target));
    return { tags: mockTags.tags, audit: mockAudit("tag.merge") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function deleteTag(input: TagDeleteRequest): Promise<TagMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<TagMutationResponse>("delete_tag", { input });
    }

    await pause(220);
    const name = input.name.trim().toLowerCase();
    mockEntries = mockEntries.map((entry) => ({
      ...entry,
      tags: entry.tags.filter((tag) => tag.name !== name),
    }));
    mockTags = rebuildMockTagUsage(mockTags.tags.filter((tag) => tag.name !== name));
    return { tags: mockTags.tags, audit: mockAudit("tag.delete") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listMoods(): Promise<MoodCatalogResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<MoodCatalogResponse>("list_moods");
    }

    await pause(120);
    return mockMoods;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function renameMood(input: MoodRenameRequest): Promise<MoodMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<MoodMutationResponse>("rename_mood", { input });
    }

    await pause(220);
    const from = input.from.trim().toLowerCase();
    const to = input.to.trim().toLowerCase();
    mockEntries = mockEntries.map((entry) =>
      entry.mood?.toLowerCase() === from
        ? { ...entry, mood: to, moodInfo: { name: to, label: labelize(to) } }
        : entry,
    );
    mockMoods = rebuildMockMoodUsage();
    return { moods: mockMoods.moods, audit: mockAudit("mood.rename") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function deleteMood(input: MoodDeleteRequest): Promise<MoodMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<MoodMutationResponse>("delete_mood", { input });
    }

    await pause(220);
    const name = input.name.trim().toLowerCase();
    mockEntries = mockEntries.map((entry) =>
      entry.mood?.toLowerCase() === name
        ? { ...entry, mood: null, moodInfo: { name: null, label: null } }
        : entry,
    );
    mockMoods = rebuildMockMoodUsage();
    return { moods: mockMoods.moods, audit: mockAudit("mood.delete") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listLibraryItems(): Promise<LibraryListResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<LibraryListResponse>("list_library_items");
    }

    await pause(120);
    return mockLibrary;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function createTemplate(
  input: LibraryTemplateInput,
): Promise<LibraryTemplateMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<LibraryTemplateMutationResponse>("create_template", { input });
    }

    await pause(260);
    const now = new Date().toISOString();
    const template = {
      id: Math.max(0, ...mockLibrary.templates.map((item) => item.id)) + 1,
      slug: slugify(input.slug),
      name: input.name.trim(),
      description: input.description?.trim() ?? "",
      introText: input.introText ?? "",
      sections: input.sections ?? [],
      isBuiltin: false,
      isActive: input.isActive ?? true,
      createdAt: now,
      updatedAt: now,
    };
    mockLibrary = { ...mockLibrary, templates: [...mockLibrary.templates, template] };
    return { template, audit: mockAudit("library.template.create") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function updateTemplate(
  slug: string,
  input: LibraryTemplateUpdate,
): Promise<LibraryTemplateMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<LibraryTemplateMutationResponse>("update_template", { slug, input });
    }

    await pause(240);
    let updated = null;
    mockLibrary = {
      ...mockLibrary,
      templates: mockLibrary.templates.map((template) => {
        if (template.slug !== slug) {
          return template;
        }
        updated = {
          ...template,
          name: input.name ?? template.name,
          description: input.description ?? template.description,
          introText: input.introText ?? template.introText,
          sections: input.sections ?? template.sections,
          isActive: input.isActive ?? template.isActive,
          updatedAt: new Date().toISOString(),
        };
        return updated;
      }),
    };
    return { template: updated, audit: mockAudit("library.template.update") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function deleteTemplate(slug: string): Promise<LibraryTemplateMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<LibraryTemplateMutationResponse>("delete_template", { slug });
    }

    await pause(220);
    mockLibrary = {
      ...mockLibrary,
      templates: mockLibrary.templates.filter((template) => template.slug !== slug),
    };
    return { template: null, audit: mockAudit("library.template.delete") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function createPrompt(
  input: LibraryPromptInput,
): Promise<LibraryPromptMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<LibraryPromptMutationResponse>("create_prompt", { input });
    }

    await pause(260);
    const now = new Date().toISOString();
    const prompt = {
      id: Math.max(0, ...mockLibrary.prompts.map((item) => item.id)) + 1,
      slug: slugify(input.slug),
      promptText: input.promptText.trim(),
      category: input.category?.trim() || "general",
      tags: normalizeTags(input.tags),
      isBuiltin: false,
      isActive: input.isActive ?? true,
      createdAt: now,
      updatedAt: now,
    };
    mockLibrary = { ...mockLibrary, prompts: [...mockLibrary.prompts, prompt] };
    return { prompt, audit: mockAudit("library.prompt.create") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function updatePrompt(
  slug: string,
  input: LibraryPromptUpdate,
): Promise<LibraryPromptMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<LibraryPromptMutationResponse>("update_prompt", { slug, input });
    }

    await pause(240);
    let updated = null;
    mockLibrary = {
      ...mockLibrary,
      prompts: mockLibrary.prompts.map((prompt) => {
        if (prompt.slug !== slug) {
          return prompt;
        }
        updated = {
          ...prompt,
          promptText: input.promptText ?? prompt.promptText,
          category: input.category ?? prompt.category,
          tags: input.tags ?? prompt.tags,
          isActive: input.isActive ?? prompt.isActive,
          updatedAt: new Date().toISOString(),
        };
        return updated;
      }),
    };
    return { prompt: updated, audit: mockAudit("library.prompt.update") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function deletePrompt(slug: string): Promise<LibraryPromptMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<LibraryPromptMutationResponse>("delete_prompt", { slug });
    }

    await pause(220);
    mockLibrary = {
      ...mockLibrary,
      prompts: mockLibrary.prompts.filter((prompt) => prompt.slug !== slug),
    };
    return { prompt: null, audit: mockAudit("library.prompt.delete") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function exportEntries(
  input: ExportEntriesRequest,
): Promise<ExportEntriesResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ExportEntriesResponse>("export_entries", { input });
    }

    await pause(260);
    const entries = input.uuids
      ? mockEntries.filter((entry) => input.uuids?.includes(entry.uuid))
      : input.search
        ? applyMockFilters(mockEntries, parseMockSearch(input.search).filters)
        : applyMockFilters(mockEntries, input.filters ?? {});
    const createdAt = new Date().toISOString();
    const stamp = createdAt
      .replace(/[-:]/g, "")
      .replace(/\.\d{3}Z$/, "")
      .replace("T", "_");
    return {
      path: `C:\\Users\\jtill\\.capsule\\exports\\capsule_export_${stamp}.${input.format === "markdown" ? "md" : "json"}`,
      format: input.format,
      entryCount: entries.length,
      createdAt,
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

export async function searchEntries(input: SearchRequest): Promise<SearchResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<SearchResponse>("search_entries", { input });
    }

    await pause(180);
    const parsed = parseMockSearch(input);
    const filtered = applyMockFilters(mockEntries, parsed.filters);
    const offset = input.offset ?? 0;
    const limit = input.limit ?? 40;
    const mode: SearchResponse["mode"] = "keyword";
    return {
      entries: filtered.slice(offset, offset + limit),
      total: filtered.length,
      limit,
      offset,
      mode,
      usedFts: false,
      parsedTokens: parsed.tokens,
      warnings:
        input.mode && input.mode !== "keyword"
          ? [
              "Semantic and hybrid search are not implemented yet; using keyword search.",
              ...parsed.warnings,
            ]
          : parsed.warnings,
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listThreads(limit = 30, offset = 0): Promise<ThreadListResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ThreadListResponse>("list_threads", { limit, offset });
    }

    await pause(180);
    const threads = buildMockThreads();
    return {
      threads: threads.slice(offset, offset + limit),
      total: threads.length,
      limit,
      offset,
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function updateThreadTitle(
  rootUuid: string,
  title?: string | null,
): Promise<ThreadMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ThreadMutationResponse>("update_thread_title", { rootUuid, title });
    }

    return updateMockThreadMetadata(rootUuid, { title });
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function updateThreadMetadata(
  rootUuid: string,
  input: ThreadMetadataUpdate,
): Promise<ThreadMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ThreadMutationResponse>("update_thread_metadata", { rootUuid, input });
    }

    return updateMockThreadMetadata(rootUuid, input);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function bulkLinkThreads(input: {
  parentUuid: string;
  childUuids: string[];
}): Promise<ThreadMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ThreadMutationResponse>("bulk_link_threads", { input });
    }

    await pause(240);
    const parent = mockEntries.find((entry) => entry.uuid === input.parentUuid);
    if (!parent) {
      throw new Error(`Entry not found: ${input.parentUuid}`);
    }
    const rootUuid = parent.thread?.rootUuid ?? parent.uuid;
    const rootThread = mockEntries.find((entry) => entry.uuid === rootUuid)?.thread;
    const affected = new Set<string>();
    mockEntries = mockEntries.map((entry) => {
      if (!input.childUuids.includes(entry.uuid)) {
        return entry;
      }
      if (entry.uuid === input.parentUuid) {
        throw new Error("An entry cannot continue itself.");
      }
      affected.add(entry.uuid);
      return {
        ...entry,
        thread: {
          rootUuid,
          parentUuid: parent.uuid,
          title: rootThread?.title ?? parent.title,
          summary: rootThread?.summary ?? parent.summary,
          entryCount: 1,
          isRoot: false,
        },
      };
    });
    syncMockThreadCounts();
    const thread = buildMockThreads().find((item) => item.rootUuid === rootUuid) ?? null;
    return { thread, affectedUuids: [...affected], audit: mockAudit("thread.link") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function bulkDetachThreads(input: {
  childUuids: string[];
}): Promise<ThreadMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ThreadMutationResponse>("bulk_detach_threads", { input });
    }

    await pause(240);
    const affected = new Set<string>();
    const rootUuid = mockEntries.find((entry) => input.childUuids.includes(entry.uuid))?.thread
      ?.rootUuid;
    mockEntries = mockEntries.map((entry) => {
      if (!input.childUuids.includes(entry.uuid) || entry.thread?.isRoot) {
        return entry;
      }
      affected.add(entry.uuid);
      return { ...entry, thread: null };
    });
    syncMockThreadCounts();
    const thread = rootUuid
      ? buildMockThreads().find((item) => item.rootUuid === rootUuid) ?? null
      : null;
    return { thread, affectedUuids: [...affected], audit: mockAudit("thread.detach") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function disbandThread(rootUuid: string): Promise<ThreadMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ThreadMutationResponse>("disband_thread", { rootUuid });
    }

    await pause(260);
    const affected: string[] = [];
    mockEntries = mockEntries.map((entry) => {
      if (entry.thread?.rootUuid !== rootUuid) {
        return entry;
      }
      affected.push(entry.uuid);
      return { ...entry, thread: null };
    });
    return { thread: null, affectedUuids: affected, audit: mockAudit("thread.disband") };
  } catch (error) {
    throw normalizeError(error);
  }
}

function parseMockSearch(input: SearchRequest) {
  const filters: EntryFilters = {
    since: input.since,
    until: input.until,
    tags: input.tags,
    excludeTags: input.excludeTags,
    moods: input.moods,
    excludeMoods: input.excludeMoods,
    starred: input.starred,
    pinned: input.pinned,
    hidden: input.hidden,
    includeHidden: input.includeHidden,
    hasImages: input.hasImages,
    limit: input.limit,
    offset: input.offset,
    sort: input.sort,
  };
  const tokens: SearchResponse["parsedTokens"] = [];
  const keywords: string[] = [];
  const parts = input.query.split(/\s+/).filter(Boolean);

  for (let index = 0; index < parts.length; index += 1) {
    let negated = false;
    let token = parts[index];
    if (token.toUpperCase() === "NOT" && parts[index + 1]) {
      negated = true;
      index += 1;
      token = parts[index];
    }

    const [rawKey, ...rawValue] = token.split(":");
    const key = rawKey.toLowerCase();
    const value = rawValue.join(":").trim();
    if (value && key === "tag") {
      if (negated) {
        filters.excludeTags = appendUnique(filters.excludeTags, value);
        tokens.push({ kind: "excludeTag", value });
      } else {
        filters.tags = appendUnique(filters.tags, value);
        tokens.push({ kind: "tag", value });
      }
    } else if (value && key === "mood") {
      if (negated) {
        filters.excludeMoods = appendUnique(filters.excludeMoods, value);
        tokens.push({ kind: "excludeMood", value });
      } else {
        filters.moods = appendUnique(filters.moods, value);
        tokens.push({ kind: "mood", value });
      }
    } else if (value && key === "before") {
      filters.until = value;
      tokens.push({ kind: "before", value });
    } else if (value && key === "after") {
      filters.since = value;
      tokens.push({ kind: "after", value });
    } else {
      if (negated) {
        keywords.push("NOT");
      }
      keywords.push(token);
    }
  }

  const keyword = keywords.join(" ").trim();
  if (keyword) {
    filters.text = keyword;
    tokens.push({ kind: "keyword", value: keyword });
  }

  return { filters, tokens, warnings: [] as string[] };
}

function buildMockThreads(): ThreadGroup[] {
  const groups = new Map<string, Entry[]>();
  for (const entry of mockEntries) {
    if (!entry.thread) {
      continue;
    }
    groups.set(entry.thread.rootUuid, [...(groups.get(entry.thread.rootUuid) ?? []), entry]);
  }

  return [...groups.entries()]
    .map(([rootUuid, entries]) => {
      const ordered = orderMockThreadEntries(rootUuid, entries);
      const root = ordered.find((entry) => entry.uuid === rootUuid) ?? ordered[0];
      return {
        rootUuid,
        title: root.thread?.title ?? root.title,
        summary: root.thread?.summary ?? root.summary,
        latestActivity:
          ordered
            .map((entry) => entry.updatedAt ?? entry.createdAt)
            .sort()
            .at(-1) ?? null,
        entryCount: ordered.length,
        entries: ordered,
      };
    })
    .filter((thread) => thread.entries.length > 1)
    .sort((left, right) => (right.latestActivity ?? "").localeCompare(left.latestActivity ?? ""));
}

function orderMockThreadEntries(rootUuid: string, entries: Entry[]) {
  const byParent = new Map<string, Entry[]>();
  for (const entry of entries) {
    if (!entry.thread?.parentUuid) {
      continue;
    }
    byParent.set(entry.thread.parentUuid, [...(byParent.get(entry.thread.parentUuid) ?? []), entry]);
  }
  for (const children of byParent.values()) {
    children.sort((left, right) => left.createdAt.localeCompare(right.createdAt));
  }

  const ordered: Entry[] = [];
  const append = (uuid: string) => {
    const entry = entries.find((item) => item.uuid === uuid);
    if (entry && !ordered.some((item) => item.uuid === entry.uuid)) {
      ordered.push(entry);
    }
    for (const child of byParent.get(uuid) ?? []) {
      append(child.uuid);
    }
  };
  append(rootUuid);

  return ordered.length === entries.length
    ? ordered
    : [...entries].sort((left, right) => left.createdAt.localeCompare(right.createdAt));
}

async function updateMockThreadMetadata(
  rootUuid: string,
  input: ThreadMetadataUpdate,
): Promise<ThreadMutationResponse> {
  await pause(240);
  const thread = buildMockThreads().find((item) => item.rootUuid === rootUuid);
  if (!thread) {
    throw new Error(`Thread not found: ${rootUuid}`);
  }

  const title = input.title === undefined ? thread.title : normalizeNullable(input.title);
  const summary = input.summary === undefined ? thread.summary : normalizeNullable(input.summary);
  mockEntries = mockEntries.map((entry) => {
    if (entry.thread?.rootUuid !== rootUuid) {
      return entry;
    }
    return {
      ...entry,
      thread: {
        ...entry.thread,
        title,
        summary,
      },
    };
  });
  const updated = buildMockThreads().find((item) => item.rootUuid === rootUuid) ?? null;
  return { thread: updated, affectedUuids: [rootUuid], audit: mockAudit("thread.metadata.update") };
}

function syncMockThreadCounts() {
  const groups = new Map<string, Entry[]>();
  for (const entry of mockEntries) {
    if (!entry.thread) {
      continue;
    }
    groups.set(entry.thread.rootUuid, [...(groups.get(entry.thread.rootUuid) ?? []), entry]);
  }
  mockEntries = mockEntries.map((entry) => {
    if (!entry.thread) {
      return entry;
    }
    return {
      ...entry,
      thread: {
        ...entry.thread,
        entryCount: groups.get(entry.thread.rootUuid)?.length ?? entry.thread.entryCount,
      },
    };
  });
}

function rebuildMockTagUsage(seedTags: TagCatalogResponse["tags"] = mockTags.tags) {
  const tagsByName = new Map<string, TagCatalogResponse["tags"][number]>();
  for (const tag of seedTags) {
    tagsByName.set(tag.name, { ...tag, entryCount: 0 });
  }
  for (const entry of mockEntries) {
    for (const tag of entry.tags) {
      const current = tagsByName.get(tag.name) ?? { id: tag.id, name: tag.name, entryCount: 0 };
      tagsByName.set(tag.name, { ...current, entryCount: current.entryCount + 1 });
    }
  }
  return {
    ...mockTags,
    tags: [...tagsByName.values()].sort((left, right) => left.name.localeCompare(right.name)),
  };
}

function rebuildMockMoodUsage(): MoodCatalogResponse {
  const counts = new Map<string, number>();
  for (const entry of mockEntries) {
    if (!entry.mood) {
      continue;
    }
    counts.set(entry.mood, (counts.get(entry.mood) ?? 0) + 1);
  }
  return {
    ...mockMoods,
    moods: [...counts.entries()]
      .map(([name, entryCount]) => ({ name, label: labelize(name), entryCount }))
      .sort((left, right) => left.name.localeCompare(right.name)),
  };
}

function appendUnique(values: string[] | undefined, value: string) {
  const normalized = value.trim();
  if (!normalized) {
    return values;
  }
  const current = values ?? [];
  return current.some((item) => item.toLowerCase() === normalized.toLowerCase())
    ? current
    : [...current, normalized];
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
  const excludedTagSet = new Set(
    filters.excludeTags?.map((tag) => tag.trim().toLowerCase()).filter(Boolean),
  );
  const moodSet = new Set(filters.moods?.map((mood) => mood.trim().toLowerCase()).filter(Boolean));
  const excludedMoodSet = new Set(
    filters.excludeMoods?.map((mood) => mood.trim().toLowerCase()).filter(Boolean),
  );

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
      if (excludedTagSet.size > 0) {
        const entryTags = new Set(entry.tags.map((tag) => tag.name.toLowerCase()));
        for (const tag of excludedTagSet) {
          if (entryTags.has(tag)) {
            return false;
          }
        }
      }
      if (moodSet.size > 0 && (!entry.mood || !moodSet.has(entry.mood.toLowerCase()))) {
        return false;
      }
      if (
        excludedMoodSet.size > 0 &&
        entry.mood &&
        excludedMoodSet.has(entry.mood.toLowerCase())
      ) {
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

function slugify(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9_-]/g, "")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}
