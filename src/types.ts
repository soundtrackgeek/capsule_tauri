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

export type TagInfo = {
  id: number;
  name: string;
};

export type MoodInfo = {
  name: string | null;
  label: string | null;
};

export type LocationInfo = {
  latitude: number;
  longitude: number;
  placeName: string | null;
  weatherCondition: string | null;
  weatherTempC: number | null;
  weatherTempF: number | null;
};

export type EntryThreadInfo = {
  rootUuid: string;
  parentUuid: string | null;
  title: string | null;
  summary: string | null;
  entryCount: number;
  isRoot: boolean;
};

export type Entry = {
  id: number;
  uuid: string;
  createdAt: string;
  updatedAt: string | null;
  text: string;
  textPlain: string;
  contentFormat: "plain" | "markdown" | string;
  title: string | null;
  summary: string | null;
  mood: string | null;
  moodInfo: MoodInfo;
  tags: TagInfo[];
  starred: boolean;
  pinned: boolean;
  hidden: boolean;
  location: LocationInfo | null;
  thread: EntryThreadInfo | null;
  attachmentCount: number;
};

export type EntryListResponse = {
  entries: Entry[];
  total: number;
  limit: number;
  offset: number;
};

export type EntryFilters = {
  text?: string;
  location?: string;
  since?: string;
  until?: string;
  tags?: string[];
  excludeTags?: string[];
  moods?: string[];
  excludeMoods?: string[];
  starred?: boolean | null;
  pinned?: boolean | null;
  hidden?: boolean | null;
  includeHidden?: boolean;
  hasImages?: boolean | null;
  limit?: number;
  offset?: number;
  sort?: "asc" | "desc";
};

export type RandomEntryFilters = {
  includeHidden?: boolean;
  tags?: string[];
  moods?: string[];
};
