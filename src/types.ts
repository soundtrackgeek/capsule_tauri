export type SchemaSummary = {
  tableCount: number;
  detectedTables: string[];
  hasEntriesTable: boolean;
  hasTagsTable: boolean;
  hasFtsTable: boolean;
  missingCoreTables: string[];
};

export type SecurityStatus = {
  mode: "plain" | "aes" | "sqlcipher" | "unknown";
  locked: boolean;
  readable: boolean;
  message: string | null;
};

export type DatabaseStatus = {
  dbPath: string;
  dbExists: boolean;
  dbSizeBytes: number;
  dbModifiedAt: string | null;
  readable: boolean;
  schemaSummary: SchemaSummary;
  entryCount: number | null;
  tagCount: number | null;
  backupCount: number | null;
  lastBackupPath: string | null;
  security: SecurityStatus;
  warnings: string[];
};

export type BackupInfo = {
  path: string;
  manifestPath: string | null;
  createdAt: string | null;
  sizeBytes: number;
  operation: string | null;
  verified: boolean;
};

export type BackupListResponse = {
  backups: BackupInfo[];
  backupDirectory: string;
};

export type BackupCreateRequest = {
  operation?: string | null;
};

export type BackupCreateResponse = {
  backup: BackupInfo;
};
