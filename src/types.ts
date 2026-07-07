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

export type LocationConfigUpdateRequest = {
  autoCapture: boolean;
  useDefaultLocation: boolean;
  defaultLocationName?: string | null;
};

export type PathSettingsResponse = {
  databasePath: string;
  imageMediaRoot: string;
  coverWallRoot: string;
  backupDirectory: string;
  backupRetentionCount: number;
  syncPath: string | null;
  githubGistId: string | null;
  githubGistTokenConfigured: boolean;
  autoSyncEnabled: boolean;
  autoSyncIntervalMinutes: number;
  minimizeToTrayOnClose: boolean;
  debugMenuEnabled: boolean;
  settingsPath: string;
  warnings: string[];
};

export type PathSettingsUpdateRequest = {
  databasePath?: string | null;
  imageMediaRoot?: string | null;
  coverWallRoot?: string | null;
  backupDirectory?: string | null;
  backupRetentionCount?: number | null;
  syncPath?: string | null;
  githubGistId?: string | null;
  githubGistToken?: string | null;
  clearGithubGistToken?: boolean | null;
  autoSyncEnabled?: boolean | null;
  autoSyncIntervalMinutes?: number | null;
  minimizeToTrayOnClose?: boolean | null;
  debugMenuEnabled?: boolean | null;
};

export type AICloudProvider = "openai" | "gemini" | "openrouter";

export type AISettings = {
  cloudProvider: AICloudProvider;
  openaiModel: string;
  geminiModel: string;
  openrouterModel: string;
  defaultContextLimit: number | null;
  defaultSince: string | null;
  defaultUntil: string | null;
  warnings: string[];
};

export type AISettingsUpdateRequest = {
  cloudProvider: AICloudProvider;
  openaiModel: string;
  geminiModel: string;
  openrouterModel: string;
  defaultContextLimit: number | null;
  defaultSince: string | null;
  defaultUntil: string | null;
};

export type AIProviderStatus = {
  provider: AICloudProvider;
  label: string;
  configured: boolean;
  selectedModel: string;
  availableModels: string[];
  missingReason: string | null;
  keySource: string | null;
};

export type AIApiKeyUpdateRequest = {
  provider: AICloudProvider;
  apiKey: string;
};

export type AIApiKeyMutationResponse = {
  providerStatus: AIProviderStatus;
  completedAt: string;
};

export type DebugCheck = {
  label: string;
  status: "ok" | "warn" | "error" | string;
  detail: string;
  warnings: string[];
};

export type DebugDatabaseReport = {
  status: DatabaseStatus;
  integrityCheck: string | null;
  foreignKeyIssueCount: number | null;
  walSizeBytes: number | null;
  requiredTables: DebugCheck[];
  featureTables: DebugCheck[];
  warnings: string[];
};

export type DebugImageReport = {
  mediaRoot: string;
  rootExists: boolean;
  rootWritable: boolean;
  totalAssets: number;
  totalAttachments: number;
  attachmentsWithOriginals: number;
  attachmentsWithThumbnails: number;
  missingOriginals: number;
  missingThumbnails: number;
  sampleImages: ImageAttachment[];
  warnings: string[];
};

export type DebugAiReport = {
  cloudProvider: AICloudProvider | string;
  selectedModel: string;
  providerConfigured: boolean;
  providerStatuses: AIProviderStatus[];
  contextPreviewOk: boolean;
  contextPreviewEntries: number;
  warnings: string[];
};

export type DebugLogEntry = {
  timestamp: string;
  level: "info" | "warn" | "error" | string;
  message: string;
};

export type DebugLogRequest = {
  level?: "info" | "warn" | "error" | string | null;
  message: string;
};

export type DebugLogResponse = {
  entry: DebugLogEntry;
  recentLogs: DebugLogEntry[];
  logPath: string;
};

export type DebugDiagnosticsResponse = {
  generatedAt: string;
  appVersion: string;
  settingsPath: string;
  debugLogPath: string;
  bundleDirectory: string;
  database: DebugDatabaseReport;
  images: DebugImageReport;
  ai: DebugAiReport;
  recentLogs: DebugLogEntry[];
  warnings: string[];
};

export type DebugBundleResponse = {
  path: string;
  sizeBytes: number;
  createdAt: string;
  includedFiles: string[];
  warnings: string[];
};

export type AIChatScope = "search" | "entry" | "entries" | "thread";

export type AIChatMessageStatus = "complete" | "streaming" | "interrupted" | "error";

export type AIChatContextFilters = {
  text?: string | null;
  since?: string | null;
  until?: string | null;
  tags?: string[] | null;
  excludeTags?: string[] | null;
  moods?: string[] | null;
  excludeMoods?: string[] | null;
  starred?: boolean | null;
  pinned?: boolean | null;
  includeHidden?: boolean | null;
  hasImages?: boolean | null;
  sort?: "asc" | "desc" | null;
  limit?: number | null;
};

export type AIChatContextPreviewRequest = {
  message?: string | null;
  scope: AIChatScope;
  scopeIdentifiers: string[];
  contextFilters?: AIChatContextFilters | null;
  contextLimit?: number | null;
  since?: string | null;
  until?: string | null;
  contextEntryUuids?: string[] | null;
};

export type AIChatContextPreviewEntry = {
  id: number;
  uuid: string;
  createdAt: string;
  title: string | null;
  summary: string | null;
  mood: string | null;
  tags: string[];
  hidden: boolean;
  attachmentCount: number;
  threadRootUuid: string | null;
  threadTitle: string | null;
  estimatedChars: number;
  textPreview: string;
};

export type AIChatContextPreviewResponse = {
  scope: AIChatScope;
  entries: AIChatContextPreviewEntry[];
  total: number;
  contextLimit: number | null;
  warnings: string[];
};

export type AIChatRequest = {
  message: string;
  conversationId?: number | null;
  cloudProvider?: AICloudProvider | null;
  model?: string | null;
  scope: AIChatScope;
  scopeIdentifiers: string[];
  contextFilters?: AIChatContextFilters | null;
  contextLimit?: number | null;
  since?: string | null;
  until?: string | null;
  contextEntryUuids?: string[] | null;
};

export type AIChatRetryRequest = {
  conversationId: number;
  cloudProvider?: AICloudProvider | null;
  model?: string | null;
  contextEntryUuids?: string[] | null;
};

export type AIChatStreamStartResponse = {
  streamId: string;
  conversationId: number;
  assistantMessageId: number;
  provider: AICloudProvider;
  model: string;
};

export type AIConversationMessage = {
  id: number;
  uuid: string;
  role: "user" | "assistant" | string;
  content: string;
  status: AIChatMessageStatus | string;
  createdAt: string;
  updatedAt: string;
};

export type AIConversationListResponse = {
  conversations: AiConversationSummary[];
  warnings: string[];
};

export type AIConversationDetail = AiConversationSummary & {
  scopeIdentifiers: string[];
  contextLimit: number | null;
  since: string | null;
  until: string | null;
  messages: AIConversationMessage[];
};

export type DeleteAIConversationResponse = {
  conversationId: number;
  conversationUuid: string;
  audit: MutationAudit;
};

export type AIChatStartedEvent = AIChatStreamStartResponse;

export type AIChatContextEvent = {
  streamId: string;
  conversationId: number;
  context: AIChatContextPreviewResponse;
};

export type AIChatChunkEvent = {
  streamId: string;
  conversationId: number;
  assistantMessageId: number;
  chunk: string;
  content: string;
};

export type AIChatCompleteEvent = {
  streamId: string;
  conversationId: number;
  assistantMessageId: number;
  content: string;
};

export type AIChatInterruptedEvent = {
  streamId: string;
  conversationId: number;
  assistantMessageId: number;
  content: string;
  reason: string;
};

export type AIChatErrorEvent = {
  streamId: string;
  conversationId: number;
  assistantMessageId: number;
  content: string;
  message: string;
  detail: string | null;
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

export type Phase6Capability = {
  key: string;
  label: string;
  available: boolean;
  configured: boolean;
  requiresCloud: boolean;
  readOnly: boolean;
  detail: string;
};

export type AiConversationSummary = {
  id: number;
  uuid: string;
  title: string;
  preview: string;
  cloudProvider: string;
  model: string | null;
  scope: string;
  messageCount: number;
  createdAt: string;
  lastMessageAt: string;
  updatedAt: string;
};

export type AiTimeCapsuleSummary = {
  id: number;
  triggerLabel: string;
  dueDate: string;
  status: string;
  sourceEntryCount: number;
  cloudProvider: string;
  llmModel: string;
  readAt: string | null;
  dismissedAt: string | null;
  errorMessage: string | null;
};

export type EmbeddingModelSummary = {
  id: number;
  name: string;
  dimensions: number;
  provider: string;
  isActive: boolean;
  entryCount: number;
};

export type AiOverviewResponse = {
  provider: string | null;
  model: string | null;
  capabilities: Phase6Capability[];
  conversations: AiConversationSummary[];
  timeCapsules: AiTimeCapsuleSummary[];
  embeddingModels: EmbeddingModelSummary[];
  conversationCount: number;
  messageCount: number;
  timeCapsuleCount: number;
  embeddedEntryCount: number;
  warnings: string[];
};

export type AiMetadataSuggestionRequest = {
  identifier: string;
};

export type AiMetadataSuggestionResponse = {
  entryUuid: string;
  source: string;
  suggestedTitle: string | null;
  suggestedSummary: string | null;
  suggestedMood: string | null;
  suggestedTags: string[];
  confidence: number;
  warnings: string[];
};

export type AiEntryMetadataSuggestionRequest = {
  text: string;
  contentFormat?: "plain" | "markdown" | string | null;
  cloudProvider?: AICloudProvider | null;
  model?: string | null;
};

export type AiEntryMetadataSuggestionResponse = {
  title: string | null;
  summary: string | null;
  cloudProvider: AICloudProvider;
  model: string;
  warnings: string[];
};

export type SyncStatusSummary = {
  lastSuccessfulSyncAt: string | null;
  lastSyncFilePath: string | null;
  lastSyncFileSizeBytes: number | null;
  lastSyncImported: number;
  lastSyncUpdated: number;
  lastSyncDeleted: number;
  lastSyncTotal: number;
  lastSyncSummary: string | null;
  lastConflictCount: number;
  lastConflictSummary: string | null;
  lastSyncError: string | null;
};

export type SyncHistoryItem = {
  id: number;
  timestamp: string;
  status: string;
  syncFilePath: string | null;
  importedCount: number;
  updatedCount: number;
  deletedCount: number;
  exportedCount: number;
  conflictCount: number;
  summary: string | null;
  error: string | null;
};

export type SyncTombstoneCount = {
  table: string;
  count: number;
};

export type SyncOverviewResponse = {
  configured: boolean;
  syncPath: string | null;
  syncFilePath: string | null;
  githubGistId: string | null;
  githubGistTokenConfigured: boolean;
  autoSyncEnabled: boolean;
  autoSyncIntervalMinutes: number;
  status: SyncStatusSummary | null;
  recentHistory: SyncHistoryItem[];
  tombstones: SyncTombstoneCount[];
  capabilities: Phase6Capability[];
  warnings: string[];
};

export type SyncRunRequest = {
  syncPath?: string | null;
};

export type SyncRunResponse = {
  syncPath: string;
  syncFilePath: string;
  githubGistPulled: boolean;
  githubGistPushed: boolean;
  importedCount: number;
  updatedCount: number;
  deletedCount: number;
  exportedCount: number;
  conflictCount: number;
  summary: string;
  completedAt: string;
};

export type PluginInfo = {
  key: string;
  label: string;
  enabled: boolean;
  installedVersion: string | null;
  source: string;
  updatedAt: string | null;
  implemented: boolean;
  tableName: string | null;
  rowCount: number;
};

export type PluginOverviewResponse = {
  plugins: PluginInfo[];
  capabilities: Phase6Capability[];
  warnings: string[];
};

export type PluginMutationRequest = {
  pluginName: string;
  enabled: boolean;
};

export type PluginMutationResponse = {
  plugin: PluginInfo;
  plugins: PluginInfo[];
  audit: MutationAudit;
};

export type GamificationProfileSummary = {
  heroSpritePath: string | null;
  updatedAt: string | null;
};

export type GamificationQuest = {
  instanceId: string;
  questKey: string;
  kind: string;
  title: string;
  description: string;
  enemySpritePath: string | null;
  targetValue: number;
  progressValue: number;
  rewardXp: number;
  status: string;
  periodKey: string;
  startsAt: string;
  expiresAt: string | null;
  completedAt: string | null;
  claimedAt: string | null;
  updatedAt: string;
};

export type GamificationXpEvent = {
  id: number;
  sourceType: string;
  sourceKey: string;
  amount: number;
  reason: string;
  createdAt: string;
};

export type GamificationBadge = {
  badgeKey: string;
  unlockedAt: string;
  updatedAt: string;
};

export type GamificationOverviewResponse = {
  profile: GamificationProfileSummary | null;
  totalXp: number;
  level: number;
  xpToNextLevel: number;
  eventCount: number;
  recentEvents: GamificationXpEvent[];
  quests: GamificationQuest[];
  badges: GamificationBadge[];
  capabilities: Phase6Capability[];
  warnings: string[];
};

export type QuestClaimRequest = {
  instanceId: string;
};

export type QuestClaimResponse = {
  quest: GamificationQuest;
  totalXp: number;
  level: number;
  xpToNextLevel: number;
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
  source: string | null;
  weatherCondition: string | null;
  weatherTempC: number | null;
  weatherTempF: number | null;
  weatherIcon: string | null;
  weatherHumidity: number | null;
  weatherWindKph: number | null;
  weatherFetchedAt: string | null;
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

export type DeleteEntryResponse = {
  entryId: number;
  entryUuid: string;
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

export type ImageAsset = {
  id: number;
  hash: string;
  mimeType: string;
  bytes: number;
  width: number;
  height: number;
  storageBackend: string;
  storageKey: string;
  createdAt: string;
  deletedAt: string | null;
};

export type ImageAttachment = {
  attachmentId: number;
  entryUuid: string;
  mediaId: number;
  position: number;
  caption: string | null;
  altText: string | null;
  createdAt: string;
  hash: string;
  mimeType: string;
  bytes: number;
  width: number;
  height: number;
  storageBackend: string;
  storageKey: string;
  deletedAt: string | null;
  thumbnailAvailable: boolean;
  originalAvailable: boolean;
};

export type ImageEntryListResponse = {
  entryUuid: string;
  images: ImageAttachment[];
  warnings: string[];
};

export type ImageEntriesListResponse = {
  entries: Array<{
    entryUuid: string;
    images: ImageAttachment[];
  }>;
  warnings: string[];
};

export type ImageAttachRequest = {
  identifier: string;
  mediaId: number;
  caption?: string | null;
  altText?: string | null;
  position?: number | null;
};

export type ImageUploadAttachRequest = {
  identifier: string;
  images: Array<{
    filePath: string;
    caption?: string | null;
    altText?: string | null;
  }>;
};

export type ImageUploadResponse = {
  asset: ImageAsset;
  audit: MutationAudit;
};

export type ImageMutationResponse = {
  entryUuid: string;
  images: ImageAttachment[];
  audit: MutationAudit;
};

export type ImageVariant = "thumb" | "full";

export type AnalyticsPeriodRequest = {
  since?: string | null;
  until?: string | null;
};

export type AnalyticsOverview = {
  totalEntries: number;
  totalWords: number;
  averageWords: number;
  averageMoodSentiment: number | null;
  moodSentimentCount: number;
  totalImages: number;
  entriesWithImages: number;
  entriesWithLocation: number;
  longestStreakDays: number;
  currentStreakDays: number;
};

export type AnalyticsTrendPoint = {
  period: string;
  entryCount: number;
  wordCount: number;
  averageMoodSentiment: number | null;
  moodSentimentCount: number;
};

export type AnalyticsBreakdownItem = {
  label: string;
  count: number;
};

export type WordCount = {
  word: string;
  count: number;
};

export type AnalyticsResponse = {
  overview: AnalyticsOverview;
  monthlyTrend: AnalyticsTrendPoint[];
  moodBreakdown: AnalyticsBreakdownItem[];
  tagBreakdown: AnalyticsBreakdownItem[];
  locationBreakdown: AnalyticsBreakdownItem[];
  weatherBreakdown: AnalyticsBreakdownItem[];
  topWords: WordCount[];
  warnings: string[];
};

export type WritingCalendarDay = {
  date: string;
  entryCount: number;
  wordCount: number;
  imageCount: number;
  moods: string[];
  averageMoodSentiment: number | null;
  moodSentimentCount: number;
};

export type WritingCalendarResponse = {
  year: number;
  days: WritingCalendarDay[];
  totalDays: number;
  activeDays: number;
  maxEntryCount: number;
  warnings: string[];
};

export type CoverWallRequest = {
  type?: string | null;
  since?: string | null;
  until?: string | null;
  tags?: string[];
  moods?: string[];
  limit?: number;
  offset?: number;
};

export type CoverEntrySummary = {
  id: number;
  uuid: string;
  createdAt: string;
  title: string | null;
  mood: string | null;
  tags: string[];
};

export type EntryCover = {
  filename: string;
  coverType: string;
  entryUuid: string;
  bytes: number;
  modifiedAt: string | null;
  entry: CoverEntrySummary;
};

export type CoverWallResponse = {
  covers: EntryCover[];
  total: number;
  limit: number;
  offset: number;
  availableTypes: string[];
  orphanedCoverCount: number;
  coversRoot: string;
};
