import { invoke } from "@tauri-apps/api/core";
import type {
  AiMetadataSuggestionRequest,
  AiMetadataSuggestionResponse,
  AiOverviewResponse,
  AnalyticsPeriodRequest,
  AnalyticsResponse,
  BackupCreateRequest,
  BackupCreateResponse,
  BackupListResponse,
  BackupRestorePreview,
  BackupRestorePreviewRequest,
  BackupRestoreRequest,
  BackupRestoreResponse,
  CapsuleConfigResponse,
  ConfigMutationResponse,
  CoverWallRequest,
  CoverWallResponse,
  DatabaseStatus,
  DeleteEntryResponse,
  Entry,
  EntryCreate,
  EntryFilters,
  EntryHistoryResponse,
  EntryListResponse,
  EntryMutationResponse,
  EntryUpdate,
  ExportEntriesRequest,
  ExportEntriesResponse,
  GamificationOverviewResponse,
  GamificationQuest,
  QuestClaimResponse,
  ImageAttachRequest,
  ImageAttachment,
  ImageEntriesListResponse,
  ImageEntryListResponse,
  ImageMutationResponse,
  ImageUploadResponse,
  ImageVariant,
  LibraryListResponse,
  LibraryPromptInput,
  LibraryPromptMutationResponse,
  LibraryPromptUpdate,
  LibraryTemplateInput,
  LibraryTemplateMutationResponse,
  LibraryTemplateUpdate,
  LocationConfigUpdateRequest,
  MoodCatalogResponse,
  MoodDeleteRequest,
  MoodMutationResponse,
  MoodRenameRequest,
  PathSettingsResponse,
  PathSettingsUpdateRequest,
  RandomEntryFilters,
  SearchRequest,
  SearchResponse,
  SyncOverviewResponse,
  TagCatalogResponse,
  TagDeleteRequest,
  TagMergeRequest,
  TagMutationResponse,
  TagRenameRequest,
  ThreadGroup,
  ThreadListResponse,
  ThreadMetadataUpdate,
  ThreadMutationResponse,
  WritingCalendarResponse,
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const runningInTauri = () =>
  typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);

const defaultMockDatabasePath = "C:\\Users\\jtill\\.capsule\\capsule.db";
const defaultMockBackupDirectory = "C:\\Users\\jtill\\.capsule";
const mockImageMediaRoot = "C:\\Users\\jtill\\OneDrive\\_capsule\\images";
const mockPathSettingsPath = "C:\\Users\\jtill\\AppData\\Roaming\\Capsule\\path_settings.json";

let mockStatus: DatabaseStatus = {
  dbPath: defaultMockDatabasePath,
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

let mockBackups: BackupListResponse = {
  backupDirectory: defaultMockBackupDirectory,
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
    { key: "images.media_root", value: "C:\\Users\\jtill\\OneDrive\\_capsule\\images" },
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

const phase6Capabilities = {
  ai: [
    {
      key: "metadata-suggestions",
      label: "Metadata suggestions",
      available: true,
      configured: true,
      requiresCloud: false,
      readOnly: true,
      detail: "Local suggestions are available without sending journal text to a provider.",
    },
    {
      key: "ai-chat-bridge",
      label: "AI chat bridge",
      available: true,
      configured: false,
      requiresCloud: true,
      readOnly: true,
      detail: "Persisted chats are readable; live chat requires a configured Python bridge.",
    },
  ],
  sync: [
    {
      key: "shared-folder-sync",
      label: "Shared-folder sync",
      available: true,
      configured: false,
      requiresCloud: false,
      readOnly: true,
      detail: "Status and history are visible; execution remains bridge-gated.",
    },
    {
      key: "github-gist-import",
      label: "GitHub Gist import",
      available: true,
      configured: false,
      requiresCloud: true,
      readOnly: true,
      detail: "Mobile import needs the legacy bridge and explicit user action.",
    },
  ],
  gamification: [
    {
      key: "quest-claim",
      label: "Quest claiming",
      available: true,
      configured: true,
      requiresCloud: false,
      readOnly: false,
      detail: "Completed quests can be claimed with backup-guarded XP events.",
    },
  ],
};

let mockGamificationQuests: GamificationQuest[] = [
  {
    instanceId: "daily:word_surge:2026-06-29",
    questKey: "word_surge",
    kind: "daily",
    title: "Word Surge",
    description: "Write at least 150 words today.",
    enemySpritePath: null,
    targetValue: 150,
    progressValue: 300,
    rewardXp: 40,
    status: "complete",
    periodKey: "2026-06-29",
    startsAt: "2026-06-29 00:00",
    expiresAt: "2026-06-30 00:00",
    completedAt: "2026-06-29 09:19",
    claimedAt: null,
    updatedAt: "2026-06-29 09:19",
  },
  {
    instanceId: "weekly:storyteller:2026-W27",
    questKey: "storyteller",
    kind: "weekly",
    title: "Storyteller",
    description: "Write 750 words this week.",
    enemySpritePath: null,
    targetValue: 750,
    progressValue: 420,
    rewardXp: 120,
    status: "active",
    periodKey: "2026-W27",
    startsAt: "2026-06-29 00:00",
    expiresAt: "2026-07-06 00:00",
    completedAt: null,
    claimedAt: null,
    updatedAt: "2026-06-29 09:20",
  },
];

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
      source: "manual",
      weatherCondition: "Overcast",
      weatherTempC: 8,
      weatherTempF: 46.4,
      weatherIcon: "cloudy",
      weatherHumidity: 82,
      weatherWindKph: 11.4,
      weatherFetchedAt: "2026-06-29 12:41",
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

let mockUploadedAssets: ImageUploadResponse["asset"][] = [];

let mockImageAttachments: Record<string, ImageAttachment[]> = {
  entry_oiuir59w: [
    {
      attachmentId: 1,
      entryUuid: "entry_oiuir59w",
      mediaId: 1,
      position: 0,
      caption: "Desk reference",
      altText: "A quiet desktop reference image",
      createdAt: "2026-06-28 21:16",
      hash: "mock-desk",
      mimeType: "image/jpeg",
      bytes: 102_817,
      width: 533,
      height: 800,
      storageBackend: "local_fs",
      storageKey: "33/mock-desk.jpg",
      deletedAt: null,
      thumbnailAvailable: true,
      originalAvailable: true,
    },
    {
      attachmentId: 2,
      entryUuid: "entry_oiuir59w",
      mediaId: 2,
      position: 1,
      caption: null,
      altText: "A second journal attachment",
      createdAt: "2026-06-28 21:17",
      hash: "mock-note",
      mimeType: "image/jpeg",
      bytes: 384_718,
      width: 1024,
      height: 1024,
      storageBackend: "local_fs",
      storageKey: "33/mock-note.jpg",
      deletedAt: null,
      thumbnailAvailable: true,
      originalAvailable: true,
    },
  ],
  entry_kree51ux: [
    {
      attachmentId: 3,
      entryUuid: "entry_kree51ux",
      mediaId: 3,
      position: 0,
      caption: "Art draft",
      altText: "A square art experiment",
      createdAt: "2026-06-27 00:14",
      hash: "mock-art",
      mimeType: "image/png",
      bytes: 277_117,
      width: 1200,
      height: 1200,
      storageBackend: "local_fs",
      storageKey: "0d/mock-art.png",
      deletedAt: null,
      thumbnailAvailable: true,
      originalAvailable: true,
    },
  ],
};

const mockCoverFiles = [
  "magazine-entry_ti99r1ya.png",
  "magazine-entry_oiuir59w.png",
  "notebook-entry_kree51ux.png",
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

export async function getImageMediaRoot(): Promise<string> {
  try {
    if (runningInTauri()) {
      return await invoke<string>("get_image_media_root");
    }

    await pause(120);
    return mockConfig.values.find((item) => item.key === "images.media_root")?.value ?? mockImageMediaRoot;
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getPathSettings(): Promise<PathSettingsResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<PathSettingsResponse>("get_path_settings");
    }

    await pause(120);
    return mockPathSettings();
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function setPathSettings(
  input: PathSettingsUpdateRequest,
): Promise<PathSettingsResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<PathSettingsResponse>("set_path_settings", { input });
    }

    await pause(180);
    const databasePath = normalizeNullable(input.databasePath);
    const imageMediaRoot = normalizeNullable(input.imageMediaRoot);
    const backupDirectory = normalizeNullable(input.backupDirectory);

    mockStatus = {
      ...mockStatus,
      dbPath: databasePath ?? defaultMockDatabasePath,
    };
    mockBackups = {
      ...mockBackups,
      backupDirectory: backupDirectory ?? defaultMockBackupDirectory,
    };
    mockConfig = {
      ...mockConfig,
      values: upsertConfigValue(mockConfig.values, "images.media_root", imageMediaRoot),
    };

    return mockPathSettings();
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function browseDatabasePath(currentPath?: string | null): Promise<string | null> {
  try {
    if (runningInTauri()) {
      return await invoke<string | null>("browse_database_path", { currentPath });
    }

    await pause(80);
    return window.prompt("Database path", currentPath ?? mockStatus.dbPath);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function browseDirectoryPath(currentPath?: string | null): Promise<string | null> {
  try {
    if (runningInTauri()) {
      return await invoke<string | null>("browse_directory_path", { currentPath });
    }

    await pause(80);
    return window.prompt("Folder path", currentPath ?? defaultMockBackupDirectory);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function browseImagePath(currentPath?: string | null): Promise<string | null> {
  try {
    if (runningInTauri()) {
      return await invoke<string | null>("browse_image_path", { currentPath });
    }

    await pause(80);
    return window.prompt("Image path", currentPath ?? "");
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function browseImagePaths(currentPath?: string | null): Promise<string[]> {
  try {
    if (runningInTauri()) {
      return await invoke<string[]>("browse_image_paths", { currentPath });
    }

    await pause(80);
    const selected = window.prompt("Image paths, separated by semicolons", currentPath ?? "");
    if (!selected) {
      return [];
    }
    return selected
      .split(";")
      .map((path) => path.trim())
      .filter(Boolean);
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

export async function setLocationConfig(
  input: LocationConfigUpdateRequest,
): Promise<ConfigMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ConfigMutationResponse>("set_location_config", { input });
    }

    await pause(180);
    const nextValues = mockConfig.values.filter(
      (item) =>
        ![
          "location.auto_capture",
          "location.use_default_location",
          "location.default_location_name",
        ].includes(item.key),
    );
    nextValues.push({ key: "location.auto_capture", value: String(input.autoCapture) });
    nextValues.push({
      key: "location.use_default_location",
      value: String(input.useDefaultLocation),
    });
    if (input.useDefaultLocation && input.defaultLocationName?.trim()) {
      nextValues.push({
        key: "location.default_location_name",
        value: input.defaultLocationName.trim(),
      });
    }
    mockConfig = { ...mockConfig, exists: true, values: nextValues.sort((a, b) => a.key.localeCompare(b.key)) };
    return {
      config: mockConfig,
      backupPath: "C:\\Users\\jtill\\.capsule\\config_backup_20260630_120000.json",
      operation: "config.location.set",
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

export async function getAiOverview(): Promise<AiOverviewResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<AiOverviewResponse>("get_ai_overview");
    }

    await pause(160);
    return {
      provider: "local",
      model: "mock-read-model",
      capabilities: phase6Capabilities.ai,
      conversations: [
        {
          id: 1,
          uuid: "chat_mock",
          title: "Search reflection",
          preview: "A saved AI conversation over recent Capsule entries.",
          cloudProvider: "gemini",
          scope: "search",
          messageCount: 4,
          lastMessageAt: "2026-06-29 09:05",
          updatedAt: "2026-06-29 09:05",
        },
      ],
      timeCapsules: [
        {
          id: 1,
          triggerLabel: "One year ago",
          dueDate: "2026-06-29",
          status: "ready",
          sourceEntryCount: 6,
          cloudProvider: "gemini",
          llmModel: "capsule-legacy",
          readAt: null,
          dismissedAt: null,
          errorMessage: null,
        },
      ],
      embeddingModels: [
        {
          id: 1,
          name: "text-embedding-mock",
          dimensions: 768,
          provider: "local",
          isActive: true,
          entryCount: 3,
        },
      ],
      conversationCount: 1,
      messageCount: 4,
      timeCapsuleCount: 1,
      embeddedEntryCount: 3,
      warnings: [
        "Mock mode does not call cloud providers; live AI chat remains bridge-gated.",
      ],
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function suggestAiMetadata(
  input: AiMetadataSuggestionRequest,
): Promise<AiMetadataSuggestionResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<AiMetadataSuggestionResponse>("suggest_ai_metadata", { input });
    }

    await pause(180);
    const entry = findMockEntry(input.identifier);
    const words = entry.textPlain.split(/\s+/).filter(Boolean);
    return {
      entryUuid: entry.uuid,
      source: "local-read-model",
      suggestedTitle: entry.title ?? words.slice(0, 7).join(" "),
      suggestedSummary: entry.summary ?? words.slice(0, 26).join(" "),
      suggestedMood: entry.mood ?? "focused",
      suggestedTags: ["capsule", "reflection"].filter(
        (tag) => !entry.tags.some((entryTag) => entryTag.name === tag),
      ),
      confidence: words.length > 20 ? 0.62 : 0.42,
      warnings: ["No cloud request was made in mock mode."],
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getSyncOverview(): Promise<SyncOverviewResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<SyncOverviewResponse>("get_sync_overview");
    }

    await pause(140);
    return {
      status: {
        lastSuccessfulSyncAt: "2026-06-29 09:00",
        lastSyncFilePath: "C:\\Users\\jtill\\.capsule\\mobile_sync.json",
        lastSyncFileSizeBytes: 42_000,
        lastSyncImported: 1,
        lastSyncUpdated: 2,
        lastSyncDeleted: 0,
        lastSyncTotal: 3,
        lastSyncSummary: "Mock sync completed.",
        lastConflictCount: 0,
        lastConflictSummary: null,
        lastSyncError: null,
      },
      recentHistory: [
        {
          id: 1,
          timestamp: "2026-06-29 09:00",
          status: "success",
          syncFilePath: "mobile_sync.json",
          importedCount: 1,
          updatedCount: 2,
          deletedCount: 0,
          exportedCount: 3,
          conflictCount: 0,
          summary: "Imported mobile entries.",
          error: null,
        },
      ],
      tombstones: [
        { table: "sync_tombstones", count: 2 },
        { table: "sync_image_tombstones", count: 1 },
      ],
      capabilities: phase6Capabilities.sync,
      warnings: ["Bridge execution is disabled in browser mock mode."],
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getGamificationOverview(): Promise<GamificationOverviewResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<GamificationOverviewResponse>("get_gamification_overview");
    }

    await pause(140);
    const totalXp = 4720 + mockGamificationQuests
      .filter((quest) => quest.claimedAt)
      .reduce((sum, quest) => sum + quest.rewardXp, 0);
    const level = Math.floor(totalXp / 500) + 1;
    return {
      profile: {
        heroSpritePath: "local-assets/heroes/default.png",
        updatedAt: "2026-06-29 09:00",
      },
      totalXp,
      level,
      xpToNextLevel: level * 500 - totalXp,
      eventCount: 224,
      recentEvents: [
        {
          id: 1,
          sourceType: "entry",
          sourceKey: "entry_ti99r1ya",
          amount: 20,
          reason: "Entry created",
          createdAt: "2026-06-29 09:00",
        },
      ],
      quests: mockGamificationQuests,
      badges: [
        {
          badgeKey: "hundred_entries",
          unlockedAt: "2026-03-13 08:17",
          updatedAt: "2026-03-13 08:17",
        },
      ],
      capabilities: phase6Capabilities.gamification,
      warnings: [],
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function claimQuest(instanceId: string): Promise<QuestClaimResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<QuestClaimResponse>("claim_quest", { input: { instanceId } });
    }

    await pause(240);
    const quest = mockGamificationQuests.find((item) => item.instanceId === instanceId);
    if (!quest) {
      throw new Error(`Quest not found: ${instanceId}`);
    }
    if (quest.claimedAt) {
      throw new Error(`Quest already claimed: ${quest.title}`);
    }
    if (quest.progressValue < quest.targetValue) {
      throw new Error(`Quest is not complete yet: ${quest.title}`);
    }
    const claimedAt = new Date().toISOString();
    mockGamificationQuests = mockGamificationQuests.map((item) =>
      item.instanceId === instanceId
        ? { ...item, status: "claimed", claimedAt, completedAt: item.completedAt ?? claimedAt }
        : item,
    );
    const updated = mockGamificationQuests.find((item) => item.instanceId === instanceId)!;
    const overview = await getGamificationOverview();
    return {
      quest: updated,
      totalXp: overview.totalXp,
      level: overview.level,
      xpToNextLevel: overview.xpToNextLevel,
      audit: mockAudit("gamification.quest.claim"),
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listEntryImages(identifier: string): Promise<ImageEntryListResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ImageEntryListResponse>("list_entry_images", { identifier });
    }

    await pause(120);
    const entry = findMockEntry(identifier);
    return {
      entryUuid: entry.uuid,
      images: mockImageAttachments[entry.uuid] ?? [],
      warnings: [],
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listImagesForEntries(uuids: string[]): Promise<ImageEntriesListResponse> {
  try {
    if (runningInTauri()) {
      return await invoke("list_images_for_entries", { uuids });
    }

    await pause(120);
    return {
      entries: uuids.map((entryUuid) => ({
        entryUuid,
        images: mockImageAttachments[entryUuid] ?? [],
      })),
      warnings: [],
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getImageDataUrl(
  attachmentId: number,
  variant: ImageVariant,
): Promise<string> {
  try {
    if (runningInTauri()) {
      return await invoke<string>("get_image_data_url", { attachmentId, variant });
    }

    await pause(80);
    return mockImageDataUrl(attachmentId, variant);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getLocalImagePreviewDataUrl(filePath: string): Promise<string> {
  try {
    if (runningInTauri()) {
      return await invoke<string>("get_local_image_preview_data_url", { filePath });
    }

    await pause(80);
    return mockLocalImagePreviewDataUrl(filePath);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function uploadImage(filePath: string): Promise<ImageUploadResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ImageUploadResponse>("upload_image", { filePath });
    }

    await pause(240);
    const id = 100 + mockUploadedAssets.length;
    const asset = {
      id,
      hash: `mock-upload-${id}`,
      mimeType: filePath.toLowerCase().endsWith(".png") ? "image/png" : "image/jpeg",
      bytes: 240_000 + id,
      width: 1200,
      height: 900,
      storageBackend: "local_fs",
      storageKey: `mock/${id}.jpg`,
      createdAt: new Date().toISOString(),
      deletedAt: null,
    };
    mockUploadedAssets = [...mockUploadedAssets, asset];
    return { asset, audit: mockAudit("image.upload") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function attachImage(input: ImageAttachRequest): Promise<ImageMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ImageMutationResponse>("attach_image", { input });
    }

    await pause(220);
    const entry = findMockEntry(input.identifier);
    const asset = mockUploadedAssets.find((item) => item.id === input.mediaId) ?? {
      id: input.mediaId,
      hash: `mock-existing-${input.mediaId}`,
      mimeType: "image/jpeg",
      bytes: 180_000,
      width: 900,
      height: 900,
      storageBackend: "local_fs",
      storageKey: `mock/${input.mediaId}.jpg`,
      createdAt: new Date().toISOString(),
      deletedAt: null,
    };
    const current = mockImageAttachments[entry.uuid] ?? [];
    const attachment: ImageAttachment = {
      attachmentId: Math.max(0, ...Object.values(mockImageAttachments).flat().map((item) => item.attachmentId)) + 1,
      entryUuid: entry.uuid,
      mediaId: asset.id,
      position: input.position ?? current.length,
      caption: normalizeNullable(input.caption),
      altText: normalizeNullable(input.altText),
      createdAt: new Date().toISOString(),
      hash: asset.hash,
      mimeType: asset.mimeType,
      bytes: asset.bytes,
      width: asset.width,
      height: asset.height,
      storageBackend: asset.storageBackend,
      storageKey: asset.storageKey,
      deletedAt: null,
      thumbnailAvailable: true,
      originalAvailable: true,
    };
    mockImageAttachments = {
      ...mockImageAttachments,
      [entry.uuid]: [...current, attachment],
    };
    mockEntries = mockEntries.map((item) =>
      item.uuid === entry.uuid ? { ...item, attachmentCount: current.length + 1 } : item,
    );
    return {
      entryUuid: entry.uuid,
      images: mockImageAttachments[entry.uuid],
      audit: mockAudit("image.attach"),
    };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function removeImage(
  attachmentId: number,
  identifier?: string | null,
): Promise<ImageMutationResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<ImageMutationResponse>("remove_image", { attachmentId, identifier });
    }

    await pause(220);
    const entryUuid = identifier ? findMockEntry(identifier).uuid : findMockEntryByAttachment(attachmentId);
    const nextImages = (mockImageAttachments[entryUuid] ?? []).filter(
      (item) => item.attachmentId !== attachmentId,
    );
    mockImageAttachments = { ...mockImageAttachments, [entryUuid]: nextImages };
    mockEntries = mockEntries.map((item) =>
      item.uuid === entryUuid ? { ...item, attachmentCount: nextImages.length } : item,
    );
    return { entryUuid, images: nextImages, audit: mockAudit("image.remove") };
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getAnalytics(
  input: AnalyticsPeriodRequest = {},
): Promise<AnalyticsResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<AnalyticsResponse>("get_analytics", { input });
    }

    await pause(180);
    return buildMockAnalytics(input);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getWritingCalendar(year?: number): Promise<WritingCalendarResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<WritingCalendarResponse>("get_writing_calendar", { year });
    }

    await pause(160);
    return buildMockCalendar(year ?? new Date().getFullYear());
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function listCoverWall(
  input: CoverWallRequest = {},
): Promise<CoverWallResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<CoverWallResponse>("list_cover_wall", { input });
    }

    await pause(180);
    return buildMockCoverWall(input);
  } catch (error) {
    throw normalizeError(error);
  }
}

export async function getCoverDataUrl(
  filename: string,
  variant: ImageVariant,
): Promise<string> {
  try {
    if (runningInTauri()) {
      return await invoke<string>("get_cover_data_url", { filename, variant });
    }

    await pause(80);
    return mockCoverDataUrl(filename, variant);
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

export async function deleteEntry(identifier: string): Promise<DeleteEntryResponse> {
  try {
    if (runningInTauri()) {
      return await invoke<DeleteEntryResponse>("delete_entry", { identifier });
    }

    await pause(260);
    const deleted = findMockEntry(identifier);
    mockEntries = mockEntries
      .filter((entry) => entry.uuid !== deleted.uuid)
      .map((entry) => (entry.id > deleted.id ? { ...entry, id: entry.id - 1 } : entry))
      .map((entry) =>
        entry.thread?.parentUuid === deleted.uuid || entry.thread?.rootUuid === deleted.uuid
          ? { ...entry, thread: null }
          : entry,
      );
    delete mockImageAttachments[deleted.uuid];
    syncMockThreadCounts();
    mockStatus = {
      ...mockStatus,
      entryCount: Math.max(0, (mockStatus.entryCount ?? mockEntries.length + 1) - 1),
    };
    return {
      entryId: deleted.id,
      entryUuid: deleted.uuid,
      audit: mockAudit("entry.delete"),
    };
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

function buildMockAnalytics(input: AnalyticsPeriodRequest): AnalyticsResponse {
  const entries = filterMockPeriod(mockEntries.filter((entry) => !entry.hidden), input);
  const totalWords = entries.reduce((sum, entry) => sum + writingWordCount(entry.textPlain), 0);
  const totalImages = entries.reduce((sum, entry) => sum + entry.attachmentCount, 0);
  const entriesWithImages = entries.filter((entry) => entry.attachmentCount > 0).length;
  const entriesWithLocation = entries.filter((entry) => entry.location).length;
  const monthly = new Map<string, { entryCount: number; wordCount: number }>();
  const moods = new Map<string, number>();
  const tags = new Map<string, number>();
  const locations = new Map<string, number>();
  const weather = new Map<string, number>();
  const words = new Map<string, number>();

  for (const entry of entries) {
    const month = entry.createdAt.slice(0, 7);
    const monthValue = monthly.get(month) ?? { entryCount: 0, wordCount: 0 };
    monthly.set(month, {
      entryCount: monthValue.entryCount + 1,
      wordCount: monthValue.wordCount + writingWordCount(entry.textPlain),
    });
    if (entry.mood) moods.set(entry.mood, (moods.get(entry.mood) ?? 0) + 1);
    for (const tag of entry.tags) tags.set(tag.name, (tags.get(tag.name) ?? 0) + 1);
    if (entry.location?.placeName) {
      locations.set(entry.location.placeName, (locations.get(entry.location.placeName) ?? 0) + 1);
    }
    if (entry.location?.weatherCondition) {
      weather.set(entry.location.weatherCondition, (weather.get(entry.location.weatherCondition) ?? 0) + 1);
    }
    for (const word of entry.textPlain.toLowerCase().match(/[a-z0-9']+/g) ?? []) {
      if (word.length > 3 && !["this", "that", "with", "from", "into", "about", "should"].includes(word)) {
        words.set(word, (words.get(word) ?? 0) + 1);
      }
    }
  }

  return {
    overview: {
      totalEntries: entries.length,
      totalWords,
      averageWords: entries.length ? totalWords / entries.length : 0,
      totalImages,
      entriesWithImages,
      entriesWithLocation,
      longestStreakDays: mockStreak(entries),
      currentStreakDays: mockStreak(entries),
    },
    monthlyTrend: [...monthly.entries()].sort().map(([period, value]) => ({ period, ...value })),
    moodBreakdown: mapToBreakdown(moods),
    tagBreakdown: mapToBreakdown(tags),
    locationBreakdown: mapToBreakdown(locations),
    weatherBreakdown: mapToBreakdown(weather),
    topWords: [...words.entries()]
      .map(([word, count]) => ({ word, count }))
      .sort((left, right) => right.count - left.count)
      .slice(0, 12),
    warnings: [],
  };
}

function buildMockCalendar(year: number): WritingCalendarResponse {
  const entries = mockEntries.filter((entry) => !entry.hidden && entry.createdAt.startsWith(String(year)));
  const days = new Map<string, WritingCalendarResponse["days"][number]>();
  for (const entry of entries) {
    const date = entry.createdAt.slice(0, 10);
    const current = days.get(date) ?? {
      date,
      entryCount: 0,
      wordCount: 0,
      imageCount: 0,
      moods: [],
    };
    current.entryCount += 1;
    current.wordCount += writingWordCount(entry.textPlain);
    current.imageCount += entry.attachmentCount;
    if (entry.mood && !current.moods.includes(entry.mood)) current.moods.push(entry.mood);
    days.set(date, current);
  }
  const values = [...days.values()].sort((left, right) => left.date.localeCompare(right.date));
  return {
    year,
    days: values,
    totalDays: isLeapYear(year) ? 366 : 365,
    activeDays: values.length,
    maxEntryCount: Math.max(0, ...values.map((day) => day.entryCount)),
    warnings: [],
  };
}

function buildMockCoverWall(input: CoverWallRequest): CoverWallResponse {
  const tags = new Set(input.tags?.map((tag) => tag.toLowerCase()));
  const moods = new Set(input.moods?.map((mood) => mood.toLowerCase()));
  const coverType = input.type?.trim().toLowerCase();
  const covers = mockCoverFiles
    .map((filename) => {
      const [type, rawUuid] = filename.replace(/\.[^.]+$/, "").split("-");
      const entry = mockEntries.find((item) => item.uuid === rawUuid && !item.hidden);
      if (!entry) return null;
      if (coverType && type !== coverType) return null;
      if (input.since && entry.createdAt.slice(0, 10) < input.since) return null;
      if (input.until && entry.createdAt.slice(0, 10) > input.until) return null;
      if (tags.size > 0 && !entry.tags.some((tag) => tags.has(tag.name.toLowerCase()))) return null;
      if (moods.size > 0 && (!entry.mood || !moods.has(entry.mood.toLowerCase()))) return null;
      return {
        filename,
        coverType: type,
        entryUuid: entry.uuid,
        bytes: 2_748_000,
        modifiedAt: "2026-06-29T10:00:00Z",
        entry: {
          id: entry.id,
          uuid: entry.uuid,
          createdAt: entry.createdAt,
          title: entry.title,
          mood: entry.mood,
          tags: entry.tags.map((tag) => tag.name),
        },
      };
    })
    .filter(Boolean) as CoverWallResponse["covers"];
  const offset = input.offset ?? 0;
  const limit = input.limit ?? 80;
  return {
    covers: covers.slice(offset, offset + limit),
    total: covers.length,
    limit,
    offset,
    availableTypes: ["magazine", "notebook"],
    orphanedCoverCount: 0,
    coversRoot: "C:\\_code\\capsule_tauri\\local-assets\\covers",
  };
}

function mockPathSettings(): PathSettingsResponse {
  return {
    databasePath: mockStatus.dbPath,
    imageMediaRoot:
      mockConfig.values.find((item) => item.key === "images.media_root")?.value ??
      mockImageMediaRoot,
    backupDirectory: mockBackups.backupDirectory,
    settingsPath: mockPathSettingsPath,
    warnings: [],
  };
}

function upsertConfigValue(
  values: CapsuleConfigResponse["values"],
  key: string,
  value: string | null,
) {
  const nextValues = values.filter((item) => item.key !== key);
  if (value) {
    nextValues.push({ key, value });
  }
  return nextValues.sort((left, right) => left.key.localeCompare(right.key));
}

function mockImageDataUrl(attachmentId: number, variant: ImageVariant) {
  const hue = (attachmentId * 53) % 360;
  const label = variant === "thumb" ? `Image ${attachmentId}` : `Attachment ${attachmentId}`;
  return svgDataUrl(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 700">
      <rect width="900" height="700" fill="hsl(${hue} 28% 28%)"/>
      <rect x="58" y="58" width="784" height="584" rx="26" fill="hsl(${hue} 26% 44%)"/>
      <circle cx="690" cy="190" r="74" fill="hsl(${hue} 50% 78%)"/>
      <path d="M110 575 318 344l130 118 84-92 255 205z" fill="hsl(${hue} 38% 68%)"/>
      <text x="80" y="110" fill="white" font-family="Segoe UI, sans-serif" font-size="42" font-weight="700">${label}</text>
    </svg>`,
  );
}

function mockLocalImagePreviewDataUrl(filePath: string) {
  const name = filePath.split(/[\\/]/).filter(Boolean).pop() ?? "Selected image";
  const hue = filePath.split("").reduce((sum, char) => sum + char.charCodeAt(0), 0) % 360;
  return svgDataUrl(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 240">
      <defs>
        <linearGradient id="g" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0" stop-color="hsl(${hue} 52% 70%)"/>
          <stop offset="1" stop-color="hsl(${(hue + 44) % 360} 44% 42%)"/>
        </linearGradient>
      </defs>
      <rect width="320" height="240" fill="url(#g)"/>
      <circle cx="80" cy="70" r="28" fill="rgba(255,255,255,.5)"/>
      <path d="M0 205 82 128l58 48 66-72 114 108v28H0z" fill="rgba(255,255,255,.58)"/>
      <text x="160" y="222" text-anchor="middle" fill="rgba(20,30,25,.72)" font-family="Segoe UI, sans-serif" font-size="17" font-weight="700">${escapeSvgText(name.slice(0, 28))}</text>
    </svg>`,
  );
}

function mockCoverDataUrl(filename: string, variant: ImageVariant) {
  const hue = filename.split("").reduce((sum, char) => sum + char.charCodeAt(0), 0) % 360;
  return svgDataUrl(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 700 1000">
      <rect width="700" height="1000" fill="hsl(${hue} 30% 18%)"/>
      <rect x="62" y="70" width="576" height="860" rx="28" fill="hsl(${hue} 34% 48%)"/>
      <path d="M108 760c150-180 242-246 366-130 50 47 90 65 132 47v183H108z" fill="hsl(${hue} 45% 72%)"/>
      <circle cx="500" cy="250" r="82" fill="hsl(${hue} 62% 80%)"/>
      <text x="92" y="142" fill="white" font-family="Segoe UI, sans-serif" font-size="${variant === "thumb" ? 42 : 48}" font-weight="800">Cover</text>
      <text x="92" y="203" fill="white" opacity=".78" font-family="Segoe UI, sans-serif" font-size="24">${filename}</text>
    </svg>`,
  );
}

function svgDataUrl(svg: string) {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

function escapeSvgText(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function filterMockPeriod(entries: Entry[], input: AnalyticsPeriodRequest) {
  return entries.filter((entry) => {
    const date = entry.createdAt.slice(0, 10);
    if (input.since && date < input.since) return false;
    if (input.until && date > input.until) return false;
    return true;
  });
}

function mapToBreakdown(values: Map<string, number>) {
  return [...values.entries()]
    .map(([label, count]) => ({ label, count }))
    .sort((left, right) => right.count - left.count || left.label.localeCompare(right.label))
    .slice(0, 12);
}

function writingWordCount(value: string) {
  return value.trim().split(/\s+/).filter(Boolean).length;
}

function mockStreak(entries: Entry[]) {
  const dates = [...new Set(entries.map((entry) => entry.createdAt.slice(0, 10)))].sort();
  let longest = 0;
  let current = 0;
  let previous = "";
  for (const date of dates) {
    const previousDate = previous ? new Date(`${previous}T00:00:00`) : null;
    const currentDate = new Date(`${date}T00:00:00`);
    const isNext = previousDate
      ? currentDate.getTime() - previousDate.getTime() === 24 * 60 * 60 * 1000
      : false;
    current = isNext ? current + 1 : 1;
    longest = Math.max(longest, current);
    previous = date;
  }
  return longest;
}

function isLeapYear(year: number) {
  return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
}

function findMockEntry(identifier: string) {
  const entry = mockEntries.find(
    (item) => item.uuid === identifier || String(item.id) === identifier,
  );
  if (!entry) throw new Error(`Entry not found: ${identifier}`);
  return entry;
}

function findMockEntryByAttachment(attachmentId: number) {
  for (const [entryUuid, images] of Object.entries(mockImageAttachments)) {
    if (images.some((image) => image.attachmentId === attachmentId)) {
      return entryUuid;
    }
  }
  throw new Error(`Attachment not found: ${attachmentId}`);
}

function parseMockSearch(input: SearchRequest) {
  const filters: EntryFilters = {
    location: input.location,
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
  const location = filters.location?.trim().toLowerCase();
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
      if (location) {
        const haystack = `${entry.location?.placeName ?? ""} ${entry.location?.weatherCondition ?? ""}`.toLowerCase();
        if (!haystack.includes(location)) {
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
