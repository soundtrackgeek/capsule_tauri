import { invoke } from "@tauri-apps/api/core";
import type {
  BackupCreateRequest,
  BackupCreateResponse,
  BackupListResponse,
  DatabaseStatus,
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
  entryCount: 1240,
  tagCount: 52,
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
