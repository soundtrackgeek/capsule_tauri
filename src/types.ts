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

export type BackupRestorePreviewRequest = {
  backupPath: string;
};

export type BackupRestorePreview = {
  backup: BackupInfo;
  dbSizeBytes: number;
  dbModifiedAt: string | null;
  schemaSummary: SchemaSummary;
  entryCount: number | null;
  tagCount: number | null;
  warnings: string[];
};

export type BackupRestoreRequest = {
  backupPath: string;
  confirmation?: string | null;
};

export type BackupRestoreResponse = {
  restoredFrom: BackupInfo;
  safetyBackup: BackupInfo;
  completedAt: string;
  status: DatabaseStatus;
};

export type CapsuleConfigValue = {
  key: string;
  value: string;
};

export type CapsuleConfigResponse = {
  configPath: string;
  exists: boolean;
  values: CapsuleConfigValue[];
  warnings: string[];
};

export type ConfigMutationResponse = {
  config: CapsuleConfigResponse;
  backupPath: string | null;
  operation: string;
  completedAt: string;
};

export type TagUsage = {
  id: number;
  name: string;
  entryCount: number;
};

export type TagCatalogResponse = {
  tags: TagUsage[];
  warnings: string[];
};

export type TagRenameRequest = {
  from: string;
  to: string;
};

export type TagMergeRequest = {
  source: string;
  target: string;
};

export type TagDeleteRequest = {
  name: string;
};

export type TagMutationResponse = {
  tags: TagUsage[];
  audit: MutationAudit;
};

export type MoodUsage = {
  name: string;
  label: string;
  entryCount: number;
};

export type MoodCatalogResponse = {
  moods: MoodUsage[];
  warnings: string[];
};

export type MoodRenameRequest = {
  from: string;
  to: string;
};

export type MoodDeleteRequest = {
  name: string;
};

export type MoodMutationResponse = {
  moods: MoodUsage[];
  audit: MutationAudit;
};

export type LibraryTemplate = {
  id: number;
  slug: string;
  name: string;
  description: string;
  introText: string;
  sections: string[];
  isBuiltin: boolean;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
};

export type LibraryPrompt = {
  id: number;
  slug: string;
  promptText: string;
  category: string;
  tags: string[];
  isBuiltin: boolean;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
};

export type LibraryListResponse = {
  templates: LibraryTemplate[];
  prompts: LibraryPrompt[];
  warnings: string[];
};

export type LibraryTemplateInput = {
  slug: string;
  name: string;
  description?: string | null;
  introText?: string | null;
  sections?: string[];
  isActive?: boolean;
};

export type LibraryTemplateUpdate = {
  name?: string;
  description?: string;
  introText?: string;
  sections?: string[];
  isActive?: boolean;
};

export type LibraryTemplateMutationResponse = {
  template: LibraryTemplate | null;
  audit: MutationAudit;
};

export type LibraryPromptInput = {
  slug: string;
  promptText: string;
  category?: string | null;
  tags?: string[];
  isActive?: boolean;
};

export type LibraryPromptUpdate = {
  promptText?: string;
  category?: string;
  tags?: string[];
  isActive?: boolean;
};

export type LibraryPromptMutationResponse = {
  prompt: LibraryPrompt | null;
  audit: MutationAudit;
};

export type ExportFormat = "markdown" | "json";

export type ExportEntriesRequest = {
  format: ExportFormat;
  uuids?: string[];
  search?: SearchRequest;
  filters?: EntryFilters;
  fileName?: string | null;
};

export type ExportEntriesResponse = {
  path: string;
  format: ExportFormat;
  entryCount: number;
  createdAt: string;
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

export type EntryCreate = {
  text: string;
  contentFormat?: "plain" | "markdown";
  title?: string | null;
  summary?: string | null;
  mood?: string | null;
  tags?: string[];
  when?: string | null;
  starred?: boolean;
  pinned?: boolean;
  continueFromUuid?: string | null;
};

export type EntryUpdate = {
  text?: string;
  contentFormat?: "plain" | "markdown";
  title?: string | null;
  summary?: string | null;
  mood?: string | null;
  tags?: string[];
  starred?: boolean;
  pinned?: boolean;
  hidden?: boolean;
  continueFromUuid?: string | null;
};

export type MutationAudit = {
  backupPath: string;
  operation: string;
  completedAt: string;
};

export type EntryMutationResponse = {
  entry: Entry;
  audit: MutationAudit;
};

export type EntryHistoryItem = {
  id: number;
  timestamp: string;
  operationType: "EDIT_TEXT" | "EDIT_MOOD" | "EDIT_TAGS" | string;
  oldData: Record<string, unknown>;
  changedFields: string[];
};

export type EntryHistoryResponse = {
  entryId: number;
  current: Record<string, unknown>;
  history: EntryHistoryItem[];
  count: number;
};

export type SearchMode = "keyword" | "semantic" | "hybrid";

export type SearchRequest = {
  query: string;
  mode?: SearchMode;
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

export type StructuredTokenKind =
  | "keyword"
  | "tag"
  | "excludeTag"
  | "mood"
  | "excludeMood"
  | "before"
  | "after";

export type StructuredQueryToken = {
  kind: StructuredTokenKind;
  value: string;
};

export type SearchResponse = {
  entries: Entry[];
  total: number;
  limit: number;
  offset: number;
  mode: SearchMode;
  usedFts: boolean;
  parsedTokens: StructuredQueryToken[];
  warnings: string[];
};

export type ThreadGroup = {
  rootUuid: string;
  title: string | null;
  summary: string | null;
  latestActivity: string | null;
  entryCount: number;
  entries: Entry[];
};

export type ThreadListResponse = {
  threads: ThreadGroup[];
  total: number;
  limit: number;
  offset: number;
};

export type ThreadMetadataUpdate = {
  title?: string | null;
  summary?: string | null;
};

export type BulkThreadDetachRequest = {
  childUuids: string[];
};

export type BulkThreadLinkRequest = {
  parentUuid: string;
  childUuids: string[];
};

export type ThreadMutationResponse = {
  thread: ThreadGroup | null;
  affectedUuids: string[];
  audit: MutationAudit;
};
