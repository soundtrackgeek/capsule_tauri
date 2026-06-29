import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Archive,
  BarChart3,
  BookOpen,
  Bot,
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  CheckCircle2,
  Clock3,
  Cloud,
  Database,
  Download,
  Edit3,
  Eye,
  EyeOff,
  FileImage,
  FileArchive,
  FileText,
  Filter,
  FolderOpen,
  GitBranch,
  History,
  HardDrive,
  Image as ImageIcon,
  Images,
  Info,
  Link2,
  MapPin,
  Maximize2,
  Paperclip,
  Pin,
  PinOff,
  Plug,
  Plus,
  RefreshCw,
  Save,
  Search,
  Settings,
  ShieldCheck,
  Shuffle,
  Sparkles,
  Star,
  Tags,
  Trophy,
  TriangleAlert,
  Trash2,
  Unlink2,
  Upload,
  X,
} from "lucide-react";
import {
  attachImage,
  bulkDetachThreads,
  claimQuest,
  createPrompt,
  createEntry,
  createBackup,
  createTemplate,
  deleteCapsuleConfigValue,
  deleteMood,
  deletePrompt,
  deleteTag,
  deleteTemplate,
  disbandThread,
  exportEntries,
  getAnalytics,
  getAiOverview,
  getCapsuleConfig,
  getCoverDataUrl,
  getDatabaseStatus,
  getGamificationOverview,
  getEntry,
  getImageDataUrl,
  getPluginOverview,
  getRandomEntry,
  getSyncOverview,
  getWritingCalendar,
  hideEntry,
  listCoverWall,
  listEntryHistory,
  listEntryImages,
  listImagesForEntries,
  listBackups,
  listEntries,
  listLibraryItems,
  listMoods,
  listTags,
  listThreads,
  mergeTag,
  openBackupFolder,
  previewRestoreBackup,
  pinEntry,
  renameMood,
  renameTag,
  removeImage,
  restoreBackup,
  searchEntries,
  setCapsuleConfigValue,
  setPluginEnabled,
  starEntry,
  suggestAiMetadata,
  unhideEntry,
  unpinEntry,
  unstarEntry,
  updateThreadMetadata,
  updateEntry,
  updatePrompt,
  updateTemplate,
  uploadImage,
} from "./backend";
import { StatusPill } from "./components/StatusPill";
import { formatBytes, formatDateTime } from "./lib/format";
import type {
  BackupInfo,
  BackupRestorePreview,
  CapsuleConfigResponse,
  AiMetadataSuggestionResponse,
  AiOverviewResponse,
  AnalyticsPeriodRequest,
  AnalyticsResponse,
  CoverWallRequest,
  CoverWallResponse,
  DatabaseStatus,
  Entry,
  EntryCreate,
  EntryFilters,
  EntryHistoryResponse,
  EntryListResponse,
  EntryMutationResponse,
  EntryUpdate,
  ExportFormat,
  EntryCover,
  GamificationOverviewResponse,
  GamificationQuest,
  ImageAttachment,
  ImageEntryListResponse,
  ImageMutationResponse,
  ImageVariant,
  LibraryListResponse,
  MoodCatalogResponse,
  Phase6Capability,
  PluginOverviewResponse,
  SearchRequest,
  SearchResponse,
  SyncOverviewResponse,
  TagCatalogResponse,
  ThreadGroup,
  ThreadListResponse,
  ThreadMutationResponse,
  WritingCalendarResponse,
} from "./types";
import "./styles.css";

type ActiveView =
  | "dashboard"
  | "entries"
  | "threads"
  | "search"
  | "ai"
  | "sync"
  | "images"
  | "analytics"
  | "calendar"
  | "covers"
  | "plugins"
  | "gamification"
  | "composer"
  | "writer"
  | "backups"
  | "settings"
  | "about";

type EntryFilterForm = {
  text: string;
  tag: string;
  mood: string;
  location: string;
  since: string;
  until: string;
  includeHidden: boolean;
  hasImages: boolean;
  sort: "asc" | "desc";
};

type SearchForm = {
  query: string;
  mode: "keyword" | "semantic" | "hybrid";
  tag: string;
  excludeTag: string;
  mood: string;
  excludeMood: string;
  location: string;
  since: string;
  until: string;
  includeHidden: boolean;
  hasImages: boolean;
  sort: "asc" | "desc";
};

type ThreadMetadataDraft = {
  title: string;
  summary: string;
};

type DashboardCounts = {
  currentYear: number | null;
  currentMonth: number | null;
};

type ComposerMode = "create" | "edit";

type ComposerDraft = {
  text: string;
  title: string;
  summary: string;
  mood: string;
  tags: string;
  when: string;
  starred: boolean;
  pinned: boolean;
  continueFromUuid: string;
};

type WriterSettings = {
  background: string;
  color: string;
  fontFamily: string;
  fontSize: number;
  lineSpacing: number;
};

type UiSettings = {
  theme: "system" | "light" | "dark";
  sidebarMode: "comfortable" | "compact";
};

type ImageUploadDraft = {
  path: string;
  caption: string;
  altText: string;
};

type PeriodForm = {
  since: string;
  until: string;
};

type CoverWallFilters = {
  coverType: string;
  tag: string;
  mood: string;
  since: string;
  until: string;
};

const navItems: Array<{ id: ActiveView; label: string; icon: ReactNode }> = [
  { id: "dashboard", label: "Dashboard", icon: <Database size={18} /> },
  { id: "entries", label: "Entries", icon: <BookOpen size={18} /> },
  { id: "threads", label: "Threads", icon: <GitBranch size={18} /> },
  { id: "search", label: "Search", icon: <Search size={18} /> },
  { id: "ai", label: "AI", icon: <Bot size={18} /> },
  { id: "sync", label: "Sync", icon: <Cloud size={18} /> },
  { id: "images", label: "Images", icon: <Paperclip size={18} /> },
  { id: "analytics", label: "Analytics", icon: <BarChart3 size={18} /> },
  { id: "calendar", label: "Calendar", icon: <CalendarDays size={18} /> },
  { id: "covers", label: "Cover Wall", icon: <Images size={18} /> },
  { id: "plugins", label: "Plugins", icon: <Plug size={18} /> },
  { id: "gamification", label: "Profile", icon: <Trophy size={18} /> },
  { id: "composer", label: "New Entry", icon: <Plus size={18} /> },
  { id: "writer", label: "Writer", icon: <Sparkles size={18} /> },
  { id: "backups", label: "Backups", icon: <Archive size={18} /> },
  { id: "settings", label: "Settings", icon: <Settings size={18} /> },
  { id: "about", label: "About", icon: <Info size={18} /> },
];

const emptyComposerDraft: ComposerDraft = {
  text: "",
  title: "",
  summary: "",
  mood: "",
  tags: "",
  when: "",
  starred: false,
  pinned: false,
  continueFromUuid: "",
};

const defaultWriterSettings: WriterSettings = {
  background: "#f7f6f0",
  color: "#17201b",
  fontFamily: "Georgia, ui-serif, serif",
  fontSize: 21,
  lineSpacing: 1.75,
};

const draftStorageKey = "capsule-tauri-composer-draft-v1";
const uiSettingsStorageKey = "capsule-tauri-ui-settings-v1";

const defaultUiSettings: UiSettings = {
  theme: "system",
  sidebarMode: "comfortable",
};

const defaultEntryFilters: EntryFilterForm = {
  text: "",
  tag: "",
  mood: "",
  location: "",
  since: "",
  until: "",
  includeHidden: false,
  hasImages: false,
  sort: "desc",
};

const defaultSearchForm: SearchForm = {
  query: "",
  mode: "keyword",
  tag: "",
  excludeTag: "",
  mood: "",
  excludeMood: "",
  location: "",
  since: "",
  until: "",
  includeHidden: false,
  hasImages: false,
  sort: "desc",
};

const emptyThreadDraft: ThreadMetadataDraft = {
  title: "",
  summary: "",
};

function formatWeatherTemperature(location: NonNullable<Entry["location"]>) {
  if (location.weatherTempC !== null) {
    return `${location.weatherTempC.toFixed(1)} C`;
  }

  if (location.weatherTempF !== null) {
    return `${location.weatherTempF.toFixed(1)} F`;
  }

  return null;
}

function App() {
  const [activeView, setActiveView] = useState<ActiveView>("dashboard");
  const [status, setStatus] = useState<DatabaseStatus | null>(null);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupDirectory, setBackupDirectory] = useState<string>("");
  const [recentEntries, setRecentEntries] = useState<Entry[]>([]);
  const [pinnedEntries, setPinnedEntries] = useState<Entry[]>([]);
  const [randomEntry, setRandomEntry] = useState<Entry | null>(null);
  const [dashboardCounts, setDashboardCounts] = useState<DashboardCounts>({
    currentYear: null,
    currentMonth: null,
  });
  const [entryFilters, setEntryFilters] = useState<EntryFilterForm>(defaultEntryFilters);
  const [entryLimit, setEntryLimit] = useState(40);
  const [entryResponse, setEntryResponse] = useState<EntryListResponse | null>(null);
  const [selectedEntry, setSelectedEntry] = useState<Entry | null>(null);
  const [searchForm, setSearchForm] = useState<SearchForm>(defaultSearchForm);
  const [searchLimit, setSearchLimit] = useState(40);
  const [searchResponse, setSearchResponse] = useState<SearchResponse | null>(null);
  const [aiOverview, setAiOverview] = useState<AiOverviewResponse | null>(null);
  const [aiSuggestion, setAiSuggestion] = useState<AiMetadataSuggestionResponse | null>(null);
  const [aiSuggestionIdentifier, setAiSuggestionIdentifier] = useState("");
  const [syncOverview, setSyncOverview] = useState<SyncOverviewResponse | null>(null);
  const [pluginOverview, setPluginOverview] = useState<PluginOverviewResponse | null>(null);
  const [gamificationOverview, setGamificationOverview] =
    useState<GamificationOverviewResponse | null>(null);
  const [imageEntryResponse, setImageEntryResponse] = useState<EntryListResponse | null>(null);
  const [imageLimit, setImageLimit] = useState(40);
  const [selectedImageEntry, setSelectedImageEntry] = useState<Entry | null>(null);
  const [entryImages, setEntryImages] = useState<ImageEntryListResponse | null>(null);
  const [imageUploadDraft, setImageUploadDraft] = useState<ImageUploadDraft>({
    path: "",
    caption: "",
    altText: "",
  });
  const [analyticsPeriod, setAnalyticsPeriod] = useState<PeriodForm>({ since: "", until: "" });
  const [analytics, setAnalytics] = useState<AnalyticsResponse | null>(null);
  const [writingCalendarYear, setWritingCalendarYear] = useState(new Date().getFullYear());
  const [writingCalendar, setWritingCalendar] = useState<WritingCalendarResponse | null>(null);
  const [coverFilters, setCoverFilters] = useState<CoverWallFilters>({
    coverType: "",
    tag: "",
    mood: "",
    since: "",
    until: "",
  });
  const [coverLimit, setCoverLimit] = useState(60);
  const [coverWall, setCoverWall] = useState<CoverWallResponse | null>(null);
  const [selectedCover, setSelectedCover] = useState<EntryCover | null>(null);
  const [threadResponse, setThreadResponse] = useState<ThreadListResponse | null>(null);
  const [selectedThreadRoot, setSelectedThreadRoot] = useState<string | null>(null);
  const [threadDraft, setThreadDraft] = useState<ThreadMetadataDraft>(emptyThreadDraft);
  const [composerMode, setComposerMode] = useState<ComposerMode>("create");
  const [editingEntry, setEditingEntry] = useState<Entry | null>(null);
  const [composerDraft, setComposerDraft] = useState<ComposerDraft>(emptyComposerDraft);
  const [draftRecovered, setDraftRecovered] = useState(false);
  const [writerSettings, setWriterSettings] = useState<WriterSettings>(defaultWriterSettings);
  const [uiSettings, setUiSettings] = useState<UiSettings>(defaultUiSettings);
  const [entryHistory, setEntryHistory] = useState<EntryHistoryResponse | null>(null);
  const [restorePreview, setRestorePreview] = useState<BackupRestorePreview | null>(null);
  const [capsuleConfig, setCapsuleConfig] = useState<CapsuleConfigResponse | null>(null);
  const [tagCatalog, setTagCatalog] = useState<TagCatalogResponse | null>(null);
  const [moodCatalog, setMoodCatalog] = useState<MoodCatalogResponse | null>(null);
  const [library, setLibrary] = useState<LibraryListResponse | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [loading, setLoading] = useState(true);
  const [entriesLoading, setEntriesLoading] = useState(false);
  const [searchLoading, setSearchLoading] = useState(false);
  const [aiLoading, setAiLoading] = useState(false);
  const [aiSuggesting, setAiSuggesting] = useState(false);
  const [syncLoading, setSyncLoading] = useState(false);
  const [pluginsLoading, setPluginsLoading] = useState(false);
  const [pluginMutating, setPluginMutating] = useState(false);
  const [gamificationLoading, setGamificationLoading] = useState(false);
  const [questMutating, setQuestMutating] = useState(false);
  const [imagesLoading, setImagesLoading] = useState(false);
  const [imageDetailLoading, setImageDetailLoading] = useState(false);
  const [imageMutating, setImageMutating] = useState(false);
  const [analyticsLoading, setAnalyticsLoading] = useState(false);
  const [calendarLoading, setCalendarLoading] = useState(false);
  const [coverLoading, setCoverLoading] = useState(false);
  const [threadsLoading, setThreadsLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [creatingBackup, setCreatingBackup] = useState(false);
  const [restoringBackup, setRestoringBackup] = useState(false);
  const [dataToolsLoading, setDataToolsLoading] = useState(false);
  const [dataToolMutating, setDataToolMutating] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [savingEntry, setSavingEntry] = useState(false);
  const [savingThread, setSavingThread] = useState(false);
  const [mutatingEntryUuid, setMutatingEntryUuid] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const statusTone = useMemo(() => {
    if (!status || !status.dbExists || !status.readable) {
      return "warn";
    }

    return status.warnings.length > 0 ? "neutral" : "good";
  }, [status]);

  const builtEntryFilters = useMemo<EntryFilters>(() => {
    const tags = splitFilter(entryFilters.tag);
    const moods = splitFilter(entryFilters.mood);
    return {
      text: entryFilters.text || undefined,
      location: entryFilters.location || undefined,
      tags: tags.length ? tags : undefined,
      moods: moods.length ? moods : undefined,
      since: entryFilters.since || undefined,
      until: entryFilters.until || undefined,
      includeHidden: entryFilters.includeHidden,
      hasImages: entryFilters.hasImages ? true : null,
      limit: entryLimit,
      offset: 0,
      sort: entryFilters.sort,
    };
  }, [entryFilters, entryLimit]);

  const builtSearchRequest = useMemo<SearchRequest>(() => {
    const tags = splitFilter(searchForm.tag);
    const excludeTags = splitFilter(searchForm.excludeTag);
    const moods = splitFilter(searchForm.mood);
    const excludeMoods = splitFilter(searchForm.excludeMood);
    return {
      query: searchForm.query,
      mode: searchForm.mode,
      tags: tags.length ? tags : undefined,
      excludeTags: excludeTags.length ? excludeTags : undefined,
      moods: moods.length ? moods : undefined,
      excludeMoods: excludeMoods.length ? excludeMoods : undefined,
      location: searchForm.location || undefined,
      since: searchForm.since || undefined,
      until: searchForm.until || undefined,
      includeHidden: searchForm.includeHidden,
      hasImages: searchForm.hasImages ? true : null,
      limit: searchLimit,
      offset: 0,
      sort: searchForm.sort,
    };
  }, [searchForm, searchLimit]);

  const builtAnalyticsPeriod = useMemo<AnalyticsPeriodRequest>(
    () => ({
      since: analyticsPeriod.since || undefined,
      until: analyticsPeriod.until || undefined,
    }),
    [analyticsPeriod],
  );

  const builtCoverRequest = useMemo<CoverWallRequest>(() => {
    const tags = splitFilter(coverFilters.tag);
    const moods = splitFilter(coverFilters.mood);
    return {
      type: coverFilters.coverType || undefined,
      since: coverFilters.since || undefined,
      until: coverFilters.until || undefined,
      tags: tags.length ? tags : undefined,
      moods: moods.length ? moods : undefined,
      limit: coverLimit,
      offset: 0,
    };
  }, [coverFilters, coverLimit]);

  const selectedThread = useMemo(
    () => threadResponse?.threads.find((thread) => thread.rootUuid === selectedThreadRoot) ?? null,
    [selectedThreadRoot, threadResponse],
  );

  const loadEntryList = useCallback(async () => {
    if (!status?.readable) {
      setEntryResponse(null);
      setSelectedEntry(null);
      return;
    }

    setEntriesLoading(true);
    setError(null);
    try {
      const response = await listEntries(builtEntryFilters);
      setEntryResponse(response);
      setSelectedEntry((current) => {
        if (!current) {
          return response.entries[0] ?? null;
        }
        return response.entries.find((entry) => entry.uuid === current.uuid) ?? response.entries[0] ?? null;
      });
    } catch (listError) {
      setError(listError instanceof Error ? listError.message : "Unable to load entries");
    } finally {
      setEntriesLoading(false);
    }
  }, [builtEntryFilters, status?.readable]);

  const loadSearchResults = useCallback(async () => {
    if (!status?.readable) {
      setSearchResponse(null);
      return;
    }

    setSearchLoading(true);
    setError(null);
    try {
      const response = await searchEntries(builtSearchRequest);
      setSearchResponse(response);
      setSelectedEntry((current) => {
        if (!current) {
          return response.entries[0] ?? null;
        }
        return response.entries.find((entry) => entry.uuid === current.uuid) ?? response.entries[0] ?? null;
      });
    } catch (searchError) {
      setError(searchError instanceof Error ? searchError.message : "Unable to search entries");
    } finally {
      setSearchLoading(false);
    }
  }, [builtSearchRequest, status?.readable]);

  const loadAiOverview = useCallback(async () => {
    if (!status?.readable) {
      setAiOverview(null);
      setAiSuggestion(null);
      return;
    }

    setAiLoading(true);
    setError(null);
    try {
      setAiOverview(await getAiOverview());
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : "Unable to load AI overview");
    } finally {
      setAiLoading(false);
    }
  }, [status?.readable]);

  const loadSyncOverview = useCallback(async () => {
    if (!status?.readable) {
      setSyncOverview(null);
      return;
    }

    setSyncLoading(true);
    setError(null);
    try {
      setSyncOverview(await getSyncOverview());
    } catch (syncError) {
      setError(syncError instanceof Error ? syncError.message : "Unable to load sync overview");
    } finally {
      setSyncLoading(false);
    }
  }, [status?.readable]);

  const loadPluginOverview = useCallback(async () => {
    if (!status?.readable) {
      setPluginOverview(null);
      return;
    }

    setPluginsLoading(true);
    setError(null);
    try {
      setPluginOverview(await getPluginOverview());
    } catch (pluginError) {
      setError(
        pluginError instanceof Error ? pluginError.message : "Unable to load plugin overview",
      );
    } finally {
      setPluginsLoading(false);
    }
  }, [status?.readable]);

  const loadGamificationOverview = useCallback(async () => {
    if (!status?.readable) {
      setGamificationOverview(null);
      return;
    }

    setGamificationLoading(true);
    setError(null);
    try {
      setGamificationOverview(await getGamificationOverview());
    } catch (gameError) {
      setError(
        gameError instanceof Error ? gameError.message : "Unable to load gamification overview",
      );
    } finally {
      setGamificationLoading(false);
    }
  }, [status?.readable]);

  const loadImageEntries = useCallback(async () => {
    if (!status?.readable) {
      setImageEntryResponse(null);
      setSelectedImageEntry(null);
      setEntryImages(null);
      return;
    }

    setImagesLoading(true);
    setError(null);
    try {
      const response = await listEntries({ hasImages: true, limit: imageLimit, sort: "desc" });
      await listImagesForEntries(response.entries.map((entry) => entry.uuid));
      setImageEntryResponse(response);
      const nextSelected =
        response.entries.find((entry) => entry.uuid === selectedImageEntry?.uuid) ??
        response.entries[0] ??
        null;
      setSelectedImageEntry(nextSelected);
      if (nextSelected) {
        setEntryImages(await listEntryImages(nextSelected.uuid));
      } else {
        setEntryImages(null);
      }
    } catch (imageError) {
      setError(imageError instanceof Error ? imageError.message : "Unable to load images");
    } finally {
      setImagesLoading(false);
    }
  }, [imageLimit, selectedImageEntry?.uuid, status?.readable]);

  const loadEntryImages = useCallback(async (entry: Entry) => {
    setSelectedImageEntry(entry);
    setImageDetailLoading(true);
    setError(null);
    try {
      setEntryImages(await listEntryImages(entry.uuid));
    } catch (imageError) {
      setError(imageError instanceof Error ? imageError.message : "Unable to load entry images");
    } finally {
      setImageDetailLoading(false);
    }
  }, []);

  const loadAnalytics = useCallback(async () => {
    if (!status?.readable) {
      setAnalytics(null);
      return;
    }

    setAnalyticsLoading(true);
    setError(null);
    try {
      setAnalytics(await getAnalytics(builtAnalyticsPeriod));
    } catch (analyticsError) {
      setError(
        analyticsError instanceof Error ? analyticsError.message : "Unable to load analytics",
      );
    } finally {
      setAnalyticsLoading(false);
    }
  }, [builtAnalyticsPeriod, status?.readable]);

  const loadWritingCalendar = useCallback(async () => {
    if (!status?.readable) {
      setWritingCalendar(null);
      return;
    }

    setCalendarLoading(true);
    setError(null);
    try {
      setWritingCalendar(await getWritingCalendar(writingCalendarYear));
    } catch (calendarError) {
      setError(
        calendarError instanceof Error ? calendarError.message : "Unable to load writing calendar",
      );
    } finally {
      setCalendarLoading(false);
    }
  }, [status?.readable, writingCalendarYear]);

  const loadCoverWall = useCallback(async () => {
    if (!status?.readable) {
      setCoverWall(null);
      setSelectedCover(null);
      return;
    }

    setCoverLoading(true);
    setError(null);
    try {
      const response = await listCoverWall(builtCoverRequest);
      setCoverWall(response);
      setSelectedCover((current) => {
        if (current && response.covers.some((cover) => cover.filename === current.filename)) {
          return current;
        }
        return response.covers[0] ?? null;
      });
    } catch (coverError) {
      setError(coverError instanceof Error ? coverError.message : "Unable to load cover wall");
    } finally {
      setCoverLoading(false);
    }
  }, [builtCoverRequest, status?.readable]);

  const loadThreads = useCallback(async () => {
    if (!status?.readable) {
      setThreadResponse(null);
      setSelectedThreadRoot(null);
      return;
    }

    setThreadsLoading(true);
    setError(null);
    try {
      const response = await listThreads(50, 0);
      setThreadResponse(response);
      setSelectedThreadRoot((current) => {
        if (current && response.threads.some((thread) => thread.rootUuid === current)) {
          return current;
        }
        return response.threads[0]?.rootUuid ?? null;
      });
    } catch (threadError) {
      setError(threadError instanceof Error ? threadError.message : "Unable to load threads");
    } finally {
      setThreadsLoading(false);
    }
  }, [status?.readable]);

  const loadDataTools = useCallback(async () => {
    setDataToolsLoading(true);
    try {
      const [nextConfig, nextTags, nextMoods, nextLibrary] = await Promise.all([
        getCapsuleConfig(),
        status?.readable ? listTags() : Promise.resolve<TagCatalogResponse | null>(null),
        status?.readable ? listMoods() : Promise.resolve<MoodCatalogResponse | null>(null),
        status?.readable ? listLibraryItems() : Promise.resolve<LibraryListResponse | null>(null),
      ]);
      setCapsuleConfig(nextConfig);
      setTagCatalog(nextTags);
      setMoodCatalog(nextMoods);
      setLibrary(nextLibrary);
    } catch (toolError) {
      setError(toolError instanceof Error ? toolError.message : "Unable to load settings tools");
    } finally {
      setDataToolsLoading(false);
    }
  }, [status?.readable]);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [nextStatus, nextBackups] = await Promise.all([getDatabaseStatus(), listBackups()]);
      setStatus(nextStatus);
      setBackups(nextBackups.backups);
      setBackupDirectory(nextBackups.backupDirectory);

      if (nextStatus.readable) {
        const now = new Date();
        const yearStart = `${now.getFullYear()}-01-01`;
        const monthStart = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-01`;
        const [recent, pinned, random, yearEntries, monthEntries] = await Promise.all([
          listEntries({ limit: 6, sort: "desc" }),
          listEntries({ pinned: true, limit: 6, sort: "desc" }),
          getRandomEntry({ includeHidden: false }),
          listEntries({ since: yearStart, limit: 1 }),
          listEntries({ since: monthStart, limit: 1 }),
        ]);

        setRecentEntries(recent.entries);
        setPinnedEntries(pinned.entries);
        setRandomEntry(random);
        setDashboardCounts({
          currentYear: yearEntries.total,
          currentMonth: monthEntries.total,
        });
      } else {
        setRecentEntries([]);
        setPinnedEntries([]);
        setRandomEntry(null);
        setDashboardCounts({ currentYear: null, currentMonth: null });
      }
    } catch (refreshError) {
      setError(refreshError instanceof Error ? refreshError.message : "Unable to refresh");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const rawDraft = window.localStorage.getItem(draftStorageKey);
    if (!rawDraft) {
      return;
    }

    try {
      const parsed = JSON.parse(rawDraft) as ComposerDraft;
      if (draftHasContent(parsed)) {
        setComposerDraft({ ...emptyComposerDraft, ...parsed });
        setDraftRecovered(true);
      }
    } catch {
      window.localStorage.removeItem(draftStorageKey);
    }
  }, []);

  useEffect(() => {
    if (composerMode !== "create") {
      return;
    }

    if (draftHasContent(composerDraft)) {
      window.localStorage.setItem(draftStorageKey, JSON.stringify(composerDraft));
    } else {
      window.localStorage.removeItem(draftStorageKey);
    }
  }, [composerDraft, composerMode]);

  useEffect(() => {
    const rawSettings = window.localStorage.getItem(uiSettingsStorageKey);
    if (!rawSettings) {
      return;
    }

    try {
      setUiSettings({ ...defaultUiSettings, ...JSON.parse(rawSettings) });
    } catch {
      window.localStorage.removeItem(uiSettingsStorageKey);
    }
  }, []);

  useEffect(() => {
    window.localStorage.setItem(uiSettingsStorageKey, JSON.stringify(uiSettings));
    document.documentElement.dataset.theme = uiSettings.theme;
  }, [uiSettings]);

  useEffect(() => {
    if (activeView === "entries") {
      void loadEntryList();
    }
  }, [activeView, loadEntryList]);

  useEffect(() => {
    if (activeView === "search") {
      void loadSearchResults();
    }
  }, [activeView, loadSearchResults]);

  useEffect(() => {
    if (activeView === "ai") {
      void loadAiOverview();
    }
  }, [activeView, loadAiOverview]);

  useEffect(() => {
    if (activeView === "sync") {
      void loadSyncOverview();
    }
  }, [activeView, loadSyncOverview]);

  useEffect(() => {
    if (activeView === "plugins") {
      void loadPluginOverview();
    }
  }, [activeView, loadPluginOverview]);

  useEffect(() => {
    if (activeView === "gamification") {
      void loadGamificationOverview();
    }
  }, [activeView, loadGamificationOverview]);

  useEffect(() => {
    if (activeView === "images") {
      void loadImageEntries();
    }
  }, [activeView, loadImageEntries]);

  useEffect(() => {
    if (activeView === "analytics") {
      void loadAnalytics();
    }
  }, [activeView, loadAnalytics]);

  useEffect(() => {
    if (activeView === "calendar") {
      void loadWritingCalendar();
    }
  }, [activeView, loadWritingCalendar]);

  useEffect(() => {
    if (activeView === "covers") {
      void loadCoverWall();
    }
  }, [activeView, loadCoverWall]);

  useEffect(() => {
    if (activeView === "threads") {
      void loadThreads();
    }
  }, [activeView, loadThreads]);

  useEffect(() => {
    if (activeView === "settings") {
      void loadDataTools();
    }
  }, [activeView, loadDataTools]);

  useEffect(() => {
    setThreadDraft({
      title: selectedThread?.title ?? "",
      summary: selectedThread?.summary ?? "",
    });
  }, [selectedThread?.rootUuid, selectedThread?.summary, selectedThread?.title]);

  const handleCreateBackup = useCallback(async () => {
    setCreatingBackup(true);
    setError(null);
    setNotice(null);

    try {
      const response = await createBackup({ operation: "manual" });
      setNotice(`Created backup: ${response.backup.path}`);
      await refresh();
    } catch (backupError) {
      setError(backupError instanceof Error ? backupError.message : "Backup failed");
    } finally {
      setCreatingBackup(false);
    }
  }, [refresh]);

  const handlePreviewRestore = useCallback(async (backup: BackupInfo) => {
    setError(null);
    setNotice(null);
    try {
      setRestorePreview(await previewRestoreBackup({ backupPath: backup.path }));
    } catch (previewError) {
      setError(previewError instanceof Error ? previewError.message : "Unable to preview backup");
    }
  }, []);

  const handleRestoreBackup = useCallback(
    async (backup: BackupInfo) => {
      if (
        !window.confirm(
          "Restore this backup into the live Capsule database? A fresh safety backup will be created first.",
        )
      ) {
        return;
      }

      setRestoringBackup(true);
      setError(null);
      setNotice(null);
      try {
        const response = await restoreBackup({
          backupPath: backup.path,
          confirmation: "RESTORE",
        });
        setStatus(response.status);
        setNotice(`Restored backup. Safety backup: ${response.safetyBackup.path}`);
        setRestorePreview(null);
        await refresh();
      } catch (restoreError) {
        setError(restoreError instanceof Error ? restoreError.message : "Restore failed");
      } finally {
        setRestoringBackup(false);
      }
    },
    [refresh],
  );

  const handleOpenBackupFolder = useCallback(async () => {
    setError(null);
    try {
      await openBackupFolder();
    } catch (folderError) {
      setError(folderError instanceof Error ? folderError.message : "Unable to open backup folder");
    }
  }, []);

  const handleExportEntry = useCallback(async (entry: Entry, format: ExportFormat) => {
    setExporting(true);
    setError(null);
    setNotice(null);
    try {
      const response = await exportEntries({
        format,
        uuids: [entry.uuid],
        fileName: `${entry.uuid}.${format}`,
      });
      setNotice(`Exported ${response.entryCount} entry to ${response.path}`);
    } catch (exportError) {
      setError(exportError instanceof Error ? exportError.message : "Export failed");
    } finally {
      setExporting(false);
    }
  }, []);

  const handleExportSearch = useCallback(
    async (format: ExportFormat) => {
      setExporting(true);
      setError(null);
      setNotice(null);
      try {
        const response = await exportEntries({
          format,
          search: builtSearchRequest,
          fileName: `search-results.${format}`,
        });
        setNotice(`Exported ${response.entryCount} search results to ${response.path}`);
      } catch (exportError) {
        setError(exportError instanceof Error ? exportError.message : "Export failed");
      } finally {
        setExporting(false);
      }
    },
    [builtSearchRequest],
  );

  const runDataToolMutation = useCallback(
    async (mutation: () => Promise<string>) => {
      setDataToolMutating(true);
      setError(null);
      setNotice(null);
      try {
        const message = await mutation();
        setNotice(message);
        await refresh();
        await loadDataTools();
      } catch (toolError) {
        setError(toolError instanceof Error ? toolError.message : "Settings update failed");
      } finally {
        setDataToolMutating(false);
      }
    },
    [loadDataTools, refresh],
  );

  const handleSelectEntry = useCallback(async (entry: Entry) => {
    setSelectedEntry(entry);
    setDetailLoading(true);
    setError(null);

    try {
      const detail = await getEntry(entry.uuid);
      setSelectedEntry(detail);
    } catch (detailError) {
      setError(detailError instanceof Error ? detailError.message : "Unable to open entry");
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const handleRandomRefresh = useCallback(async () => {
    setError(null);
    try {
      setRandomEntry(await getRandomEntry({ includeHidden: false }));
    } catch (randomError) {
      setError(randomError instanceof Error ? randomError.message : "Unable to load random entry");
    }
  }, []);

  const openNewEntry = useCallback(() => {
    setComposerMode("create");
    setEditingEntry(null);
    setComposerDraft((current) =>
      composerMode === "create" && draftHasContent(current) ? current : emptyComposerDraft,
    );
    setActiveView("composer");
  }, [composerMode]);

  const openEditEntry = useCallback((entry: Entry) => {
    setComposerMode("edit");
    setEditingEntry(entry);
    setComposerDraft(draftFromEntry(entry));
    setDraftRecovered(false);
    setActiveView("composer");
  }, []);

  const openContinueEntry = useCallback((entry: Entry) => {
    setComposerMode("create");
    setEditingEntry(null);
    setComposerDraft({
      ...emptyComposerDraft,
      continueFromUuid: entry.uuid,
      tags: entry.tags.map((tag) => tag.name).join(", "),
      mood: entry.mood ?? "",
    });
    setDraftRecovered(false);
    setActiveView("composer");
  }, []);

  const applyMutationResponse = useCallback(
    async (response: EntryMutationResponse) => {
      setNotice(`Saved with backup: ${response.audit.backupPath}`);
      setSelectedEntry(response.entry);
      setEntryHistory(null);
      await refresh();
      if (activeView === "entries") {
        await loadEntryList();
      } else if (activeView === "search") {
        await loadSearchResults();
      } else if (activeView === "threads") {
        await loadThreads();
      } else if (activeView === "images") {
        await loadImageEntries();
      } else if (activeView === "analytics") {
        await loadAnalytics();
      } else if (activeView === "calendar") {
        await loadWritingCalendar();
      } else if (activeView === "covers") {
        await loadCoverWall();
      }
    },
    [
      activeView,
      loadAnalytics,
      loadCoverWall,
      loadEntryList,
      loadImageEntries,
      loadSearchResults,
      loadThreads,
      loadWritingCalendar,
      refresh,
    ],
  );

  const handleSuggestAiMetadata = useCallback(async () => {
    const identifier =
      aiSuggestionIdentifier.trim() || selectedEntry?.uuid || recentEntries[0]?.uuid || "";
    if (!identifier) {
      setError("Choose an entry UUID or select an entry first.");
      return;
    }

    setAiSuggesting(true);
    setError(null);
    setNotice(null);
    try {
      const response = await suggestAiMetadata({ identifier });
      setAiSuggestion(response);
      setAiSuggestionIdentifier(response.entryUuid);
    } catch (suggestError) {
      setError(
        suggestError instanceof Error ? suggestError.message : "Unable to suggest metadata",
      );
    } finally {
      setAiSuggesting(false);
    }
  }, [aiSuggestionIdentifier, recentEntries, selectedEntry?.uuid]);

  const handleSetPluginEnabled = useCallback(
    async (pluginName: string, enabled: boolean) => {
      setPluginMutating(true);
      setError(null);
      setNotice(null);
      try {
        const response = await setPluginEnabled({ pluginName, enabled });
        setPluginOverview((current) =>
          current
            ? { ...current, plugins: response.plugins }
            : { plugins: response.plugins, capabilities: [], warnings: [] },
        );
        setNotice(
          `${response.plugin.label} ${enabled ? "enabled" : "disabled"} with backup: ${response.audit.backupPath}`,
        );
        await refresh();
      } catch (pluginError) {
        setError(
          pluginError instanceof Error ? pluginError.message : "Unable to update plugin state",
        );
      } finally {
        setPluginMutating(false);
      }
    },
    [refresh],
  );

  const handleClaimQuest = useCallback(
    async (quest: GamificationQuest) => {
      setQuestMutating(true);
      setError(null);
      setNotice(null);
      try {
        const response = await claimQuest(quest.instanceId);
        setNotice(`Claimed ${response.quest.title} with backup: ${response.audit.backupPath}`);
        await Promise.all([loadGamificationOverview(), refresh()]);
      } catch (questError) {
        setError(questError instanceof Error ? questError.message : "Unable to claim quest");
      } finally {
        setQuestMutating(false);
      }
    },
    [loadGamificationOverview, refresh],
  );

  const handleSaveEntry = useCallback(async () => {
    if (!composerDraft.text.trim()) {
      setError("Entry text is required.");
      return;
    }

    setSavingEntry(true);
    setError(null);
    setNotice(null);
    try {
      if (composerMode === "edit" && editingEntry) {
        const input: EntryUpdate = {
          text: composerDraft.text,
          contentFormat: "markdown",
          title: nullableFromText(composerDraft.title),
          summary: nullableFromText(composerDraft.summary),
          mood: nullableFromText(composerDraft.mood),
          tags: splitFilter(composerDraft.tags),
          starred: composerDraft.starred,
          pinned: composerDraft.pinned,
          continueFromUuid: nullableFromText(composerDraft.continueFromUuid),
        };
        const response = await updateEntry(editingEntry.uuid, input);
        await applyMutationResponse(response);
      } else {
        const input: EntryCreate = {
          text: composerDraft.text,
          contentFormat: "markdown",
          title: nullableFromText(composerDraft.title),
          summary: nullableFromText(composerDraft.summary),
          mood: nullableFromText(composerDraft.mood),
          tags: splitFilter(composerDraft.tags),
          when: nullableFromText(composerDraft.when),
          starred: composerDraft.starred,
          pinned: composerDraft.pinned,
          continueFromUuid: nullableFromText(composerDraft.continueFromUuid),
        };
        const response = await createEntry(input);
        window.localStorage.removeItem(draftStorageKey);
        setComposerDraft(emptyComposerDraft);
        setDraftRecovered(false);
        await applyMutationResponse(response);
      }
      setActiveView("entries");
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : "Unable to save entry");
    } finally {
      setSavingEntry(false);
    }
  }, [applyMutationResponse, composerDraft, composerMode, editingEntry]);

  const handleEntryAction = useCallback(
    async (entry: Entry, action: "star" | "pin" | "hide" | "unhide") => {
      setMutatingEntryUuid(entry.uuid);
      setError(null);
      setNotice(null);
      try {
        const response =
          action === "star"
            ? entry.starred
              ? await unstarEntry(entry.uuid)
              : await starEntry(entry.uuid)
            : action === "pin"
              ? entry.pinned
                ? await unpinEntry(entry.uuid)
                : await pinEntry(entry.uuid)
              : action === "hide"
                ? await hideEntry(entry.uuid)
                : await unhideEntry(entry.uuid);
        await applyMutationResponse(response);
      } catch (actionError) {
        setError(actionError instanceof Error ? actionError.message : "Entry action failed");
      } finally {
        setMutatingEntryUuid(null);
      }
    },
    [applyMutationResponse],
  );

  const applyThreadMutationResponse = useCallback(
    async (response: ThreadMutationResponse) => {
      setNotice(`Thread saved with backup: ${response.audit.backupPath}`);
      setThreadResponse((current) => {
        if (!current || !response.thread) {
          return current;
        }
        return {
          ...current,
          threads: current.threads.map((thread) =>
            thread.rootUuid === response.thread?.rootUuid ? response.thread : thread,
          ),
        };
      });
      await refresh();
      await loadThreads();
      if (activeView === "search") {
        await loadSearchResults();
      }
    },
    [activeView, loadSearchResults, loadThreads, refresh],
  );

  const handleSaveThreadMetadata = useCallback(
    async (thread: ThreadGroup) => {
      setSavingThread(true);
      setError(null);
      setNotice(null);
      try {
        const response = await updateThreadMetadata(thread.rootUuid, {
          title: nullableFromText(threadDraft.title),
          summary: nullableFromText(threadDraft.summary),
        });
        await applyThreadMutationResponse(response);
      } catch (threadError) {
        setError(threadError instanceof Error ? threadError.message : "Unable to save thread");
      } finally {
        setSavingThread(false);
      }
    },
    [applyThreadMutationResponse, threadDraft],
  );

  const handleDetachThreadEntry = useCallback(
    async (entry: Entry) => {
      if (!window.confirm("Detach this entry from its thread?")) {
        return;
      }
      setMutatingEntryUuid(entry.uuid);
      setError(null);
      setNotice(null);
      try {
        const response = await bulkDetachThreads({ childUuids: [entry.uuid] });
        await applyThreadMutationResponse(response);
      } catch (threadError) {
        setError(threadError instanceof Error ? threadError.message : "Unable to detach entry");
      } finally {
        setMutatingEntryUuid(null);
      }
    },
    [applyThreadMutationResponse],
  );

  const handleDisbandThread = useCallback(
    async (thread: ThreadGroup) => {
      if (!window.confirm("Disband this thread and remove all continuation links?")) {
        return;
      }
      setSavingThread(true);
      setError(null);
      setNotice(null);
      try {
        const response = await disbandThread(thread.rootUuid);
        await applyThreadMutationResponse(response);
        setSelectedThreadRoot(null);
      } catch (threadError) {
        setError(threadError instanceof Error ? threadError.message : "Unable to disband thread");
      } finally {
        setSavingThread(false);
      }
    },
    [applyThreadMutationResponse],
  );

  const applyImageMutationResponse = useCallback(
    async (response: ImageMutationResponse) => {
      setEntryImages({
        entryUuid: response.entryUuid,
        images: response.images,
        warnings: [],
      });
      await refresh();
      await loadImageEntries();
      if (activeView === "entries") {
        await loadEntryList();
      } else if (activeView === "search") {
        await loadSearchResults();
      } else if (activeView === "analytics") {
        await loadAnalytics();
      } else if (activeView === "calendar") {
        await loadWritingCalendar();
      }
    },
    [
      activeView,
      loadAnalytics,
      loadEntryList,
      loadImageEntries,
      loadSearchResults,
      loadWritingCalendar,
      refresh,
    ],
  );

  const handleUploadAttachImage = useCallback(async () => {
    if (!selectedImageEntry) {
      setError("Select an entry before attaching an image.");
      return;
    }
    if (!imageUploadDraft.path.trim()) {
      setError("Enter a local image path.");
      return;
    }

    setImageMutating(true);
    setError(null);
    setNotice(null);
    try {
      const upload = await uploadImage(imageUploadDraft.path.trim());
      const response = await attachImage({
        identifier: selectedImageEntry.uuid,
        mediaId: upload.asset.id,
        caption: nullableFromText(imageUploadDraft.caption),
        altText: nullableFromText(imageUploadDraft.altText),
      });
      setImageUploadDraft({ path: "", caption: "", altText: "" });
      setNotice(
        `Attached image. Upload backup: ${upload.audit.backupPath}; attach backup: ${response.audit.backupPath}`,
      );
      await applyImageMutationResponse(response);
    } catch (imageError) {
      setError(imageError instanceof Error ? imageError.message : "Unable to attach image");
    } finally {
      setImageMutating(false);
    }
  }, [applyImageMutationResponse, imageUploadDraft, selectedImageEntry]);

  const handleRemoveImage = useCallback(
    async (attachment: ImageAttachment) => {
      if (!selectedImageEntry || !window.confirm("Remove this image attachment from the entry?")) {
        return;
      }

      setImageMutating(true);
      setError(null);
      setNotice(null);
      try {
        const response = await removeImage(attachment.attachmentId, selectedImageEntry.uuid);
        setNotice(`Removed image with backup: ${response.audit.backupPath}`);
        await applyImageMutationResponse(response);
      } catch (imageError) {
        setError(imageError instanceof Error ? imageError.message : "Unable to remove image");
      } finally {
        setImageMutating(false);
      }
    },
    [applyImageMutationResponse, selectedImageEntry],
  );

  const handleLoadHistory = useCallback(async (entry: Entry) => {
    setHistoryLoading(true);
    setError(null);
    try {
      setEntryHistory(await listEntryHistory(entry.uuid));
    } catch (historyError) {
      setError(historyError instanceof Error ? historyError.message : "Unable to load entry history");
    } finally {
      setHistoryLoading(false);
    }
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const savingShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s";
      const writerShortcut =
        (event.ctrlKey || event.metaKey) && event.shiftKey && event.key === ".";

      if (savingShortcut && (activeView === "composer" || activeView === "writer")) {
        event.preventDefault();
        void handleSaveEntry();
      }

      if (writerShortcut && (activeView === "composer" || activeView === "writer")) {
        event.preventDefault();
        setActiveView((current) => (current === "writer" ? "composer" : "writer"));
      }

      if (event.key === "Escape" && activeView === "writer") {
        event.preventDefault();
        setActiveView("composer");
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeView, handleSaveEntry]);

  const title = {
    dashboard: "Write-Safe Journal",
    entries: "Entries",
    threads: "Threads",
    search: "Search",
    ai: "AI",
    sync: "Sync",
    images: "Images",
    analytics: "Analytics",
    calendar: "Writing Calendar",
    covers: "Cover Wall",
    plugins: "Plugins",
    gamification: "Profile",
    composer: composerMode === "edit" ? "Edit Entry" : "New Entry",
    writer: "Writer Mode",
    backups: "Backups",
    settings: "Settings",
    about: "About",
  }[activeView];

  if (activeView === "writer") {
    return (
      <WriterModeView
        draft={composerDraft}
        error={error}
        mode={composerMode}
        notice={notice}
        onChange={setComposerDraft}
        onExit={() => setActiveView("composer")}
        onSave={handleSaveEntry}
        saving={savingEntry}
        settings={writerSettings}
        setSettings={setWriterSettings}
      />
    );
  }

  return (
    <div
      className={
        uiSettings.sidebarMode === "compact"
          ? "app-shell app-shell--compact"
          : "app-shell"
      }
    >
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            C
          </div>
          <div>
            <h1>Capsule</h1>
            <p>Local journal desktop</p>
          </div>
        </div>

        <nav className="sidebar-nav" aria-label="Primary">
          {navItems.map((item) => (
            <button
              className={activeView === item.id ? "nav-item nav-item--active" : "nav-item"}
              key={item.id}
              onClick={() => {
                if (item.id === "composer") {
                  openNewEntry();
                } else {
                  setActiveView(item.id);
                }
              }}
              type="button"
            >
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
        </nav>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Phase 6</p>
            <h2>{title}</h2>
          </div>

          <div className="topbar-actions">
            {status && (
              <StatusPill tone={statusTone}>
                {status.readable ? "Database readable" : "Database needs attention"}
              </StatusPill>
            )}
            <button
              aria-label="Refresh"
              className="icon-button"
              disabled={loading}
              onClick={refresh}
              title="Refresh"
              type="button"
            >
              <RefreshCw size={18} />
            </button>
            <button className="secondary-button" onClick={openNewEntry} type="button">
              <Plus size={18} />
              New
            </button>
            <button
              className="primary-button"
              disabled={creatingBackup || !status?.dbExists}
              onClick={handleCreateBackup}
              title="Create a verified SQLite backup"
              type="button"
            >
              <FileArchive size={18} />
              {creatingBackup ? "Creating" : "Backup"}
            </button>
          </div>
        </header>

        {error && (
          <div className="banner banner--error" role="alert">
            <TriangleAlert size={18} />
            <span>{error}</span>
          </div>
        )}

        {notice && (
          <div className="banner banner--success" role="status">
            <CheckCircle2 size={18} />
            <span>{notice}</span>
          </div>
        )}

        {draftRecovered && activeView === "composer" && (
          <div className="banner banner--neutral" role="status">
            <Clock3 size={18} />
            <span>Recovered an unsaved local draft.</span>
            <button
              className="text-button"
              onClick={() => {
                setComposerDraft(emptyComposerDraft);
                setDraftRecovered(false);
                window.localStorage.removeItem(draftStorageKey);
              }}
              type="button"
            >
              Discard
            </button>
          </div>
        )}

        {activeView === "dashboard" && (
          <DashboardView
            backups={backups}
            backupDirectory={backupDirectory}
            counts={dashboardCounts}
            loading={loading}
            onRandomRefresh={handleRandomRefresh}
            pinnedEntries={pinnedEntries}
            randomEntry={randomEntry}
            recentEntries={recentEntries}
            status={status}
            statusTone={statusTone}
          />
        )}

        {activeView === "entries" && (
          <EntriesView
            entryHistory={entryHistory}
            detailLoading={detailLoading}
            entryFilters={entryFilters}
            entryResponse={entryResponse}
            historyLoading={historyLoading}
            loading={entriesLoading}
            mutatingEntryUuid={mutatingEntryUuid}
            onContinueEntry={openContinueEntry}
            onEditEntry={openEditEntry}
            onEntryAction={handleEntryAction}
            onExportEntry={handleExportEntry}
            onLoadHistory={handleLoadHistory}
            onLoadMore={() => setEntryLimit((current) => current + 40)}
            onResetFilters={() => {
              setEntryLimit(40);
              setEntryFilters(defaultEntryFilters);
            }}
            onSelectEntry={handleSelectEntry}
            selectedEntry={selectedEntry}
            setEntryFilters={(next) => {
              setEntryLimit(40);
              setEntryFilters(next);
            }}
            status={status}
          />
        )}

        {activeView === "search" && (
          <SearchView
            detailLoading={detailLoading}
            entryHistory={entryHistory}
            historyLoading={historyLoading}
            loading={searchLoading}
            mutatingEntryUuid={mutatingEntryUuid}
            onContinueEntry={openContinueEntry}
            onEditEntry={openEditEntry}
            onEntryAction={handleEntryAction}
            onExportEntry={handleExportEntry}
            onExportSearch={handleExportSearch}
            onLoadHistory={handleLoadHistory}
            onLoadMore={() => setSearchLimit((current) => current + 40)}
            onResetSearch={() => {
              setSearchLimit(40);
              setSearchForm(defaultSearchForm);
            }}
            onSelectEntry={handleSelectEntry}
            searchForm={searchForm}
            searchResponse={searchResponse}
            selectedEntry={selectedEntry}
            setSearchForm={(next) => {
              setSearchLimit(40);
              setSearchForm(next);
            }}
            status={status}
          />
        )}

        {activeView === "ai" && (
          <AiView
            loading={aiLoading}
            onRefresh={loadAiOverview}
            onSuggest={handleSuggestAiMetadata}
            overview={aiOverview}
            selectedIdentifier={aiSuggestionIdentifier || selectedEntry?.uuid || recentEntries[0]?.uuid || ""}
            setSelectedIdentifier={setAiSuggestionIdentifier}
            status={status}
            suggesting={aiSuggesting}
            suggestion={aiSuggestion}
          />
        )}

        {activeView === "sync" && (
          <SyncView
            loading={syncLoading}
            onRefresh={loadSyncOverview}
            overview={syncOverview}
            status={status}
          />
        )}

        {activeView === "images" && (
          <ImagesView
            detailLoading={imageDetailLoading}
            draft={imageUploadDraft}
            entryImages={entryImages}
            entryResponse={imageEntryResponse}
            loading={imagesLoading}
            mutating={imageMutating}
            onAttach={handleUploadAttachImage}
            onChangeDraft={setImageUploadDraft}
            onLoadMore={() => setImageLimit((current) => current + 40)}
            onRemoveImage={handleRemoveImage}
            onSelectEntry={loadEntryImages}
            selectedEntry={selectedImageEntry}
            status={status}
          />
        )}

        {activeView === "analytics" && (
          <AnalyticsView
            analytics={analytics}
            loading={analyticsLoading}
            onRefresh={loadAnalytics}
            period={analyticsPeriod}
            setPeriod={setAnalyticsPeriod}
            status={status}
          />
        )}

        {activeView === "calendar" && (
          <WritingCalendarView
            calendar={writingCalendar}
            loading={calendarLoading}
            onRefresh={loadWritingCalendar}
            setYear={setWritingCalendarYear}
            status={status}
            year={writingCalendarYear}
          />
        )}

        {activeView === "covers" && (
          <CoverWallView
            coverWall={coverWall}
            filters={coverFilters}
            limit={coverLimit}
            loading={coverLoading}
            onLoadMore={() => setCoverLimit((current) => current + 60)}
            onRefresh={loadCoverWall}
            onSelectCover={setSelectedCover}
            selectedCover={selectedCover}
            setFilters={(next) => {
              setCoverLimit(60);
              setCoverFilters(next);
            }}
            status={status}
          />
        )}

        {activeView === "plugins" && (
          <PluginsView
            loading={pluginsLoading}
            mutating={pluginMutating}
            onRefresh={loadPluginOverview}
            onToggle={handleSetPluginEnabled}
            overview={pluginOverview}
            status={status}
          />
        )}

        {activeView === "gamification" && (
          <GamificationView
            loading={gamificationLoading}
            mutating={questMutating}
            onClaimQuest={handleClaimQuest}
            onRefresh={loadGamificationOverview}
            overview={gamificationOverview}
            status={status}
          />
        )}

        {activeView === "threads" && (
          <ThreadsView
            draft={threadDraft}
            loading={threadsLoading}
            mutatingEntryUuid={mutatingEntryUuid}
            onContinueEntry={openContinueEntry}
            onDetachEntry={handleDetachThreadEntry}
            onDisbandThread={handleDisbandThread}
            onEditEntry={openEditEntry}
            onSaveMetadata={handleSaveThreadMetadata}
            onSelectThread={setSelectedThreadRoot}
            response={threadResponse}
            saving={savingThread}
            selectedThread={selectedThread}
            setDraft={setThreadDraft}
            status={status}
          />
        )}

        {activeView === "composer" && (
          <ComposerView
            draft={composerDraft}
            editingEntry={editingEntry}
            mode={composerMode}
            onCancel={() => setActiveView("entries")}
            onChange={setComposerDraft}
            onOpenWriter={() => setActiveView("writer")}
            onSave={handleSaveEntry}
            saving={savingEntry}
            status={status}
          />
        )}

        {activeView === "backups" && (
          <BackupsView
            backupDirectory={backupDirectory}
            backups={backups}
            creatingBackup={creatingBackup}
            onOpenFolder={handleOpenBackupFolder}
            onCreateBackup={handleCreateBackup}
            onPreviewRestore={handlePreviewRestore}
            onRestoreBackup={handleRestoreBackup}
            restorePreview={restorePreview}
            restoringBackup={restoringBackup}
            status={status}
          />
        )}

        {activeView === "settings" && (
          <SettingsView
            backupDirectory={backupDirectory}
            config={capsuleConfig}
            dataToolMutating={dataToolMutating}
            library={library}
            loading={dataToolsLoading}
            moodCatalog={moodCatalog}
            onRefresh={loadDataTools}
            onRunMutation={runDataToolMutation}
            status={status}
            statusTone={statusTone}
            tagCatalog={tagCatalog}
            uiSettings={uiSettings}
            setUiSettings={setUiSettings}
          />
        )}

        {activeView === "about" && <AboutView />}
      </main>
    </div>
  );
}

type DashboardViewProps = {
  status: DatabaseStatus | null;
  statusTone: "good" | "warn" | "neutral";
  backups: BackupInfo[];
  backupDirectory: string;
  recentEntries: Entry[];
  pinnedEntries: Entry[];
  randomEntry: Entry | null;
  counts: DashboardCounts;
  loading: boolean;
  onRandomRefresh: () => void;
};

function DashboardView({
  status,
  statusTone,
  backups,
  backupDirectory,
  recentEntries,
  pinnedEntries,
  randomEntry,
  counts,
  loading,
  onRandomRefresh,
}: DashboardViewProps) {
  return (
    <section className="dashboard" aria-label="Journal dashboard">
      <div className="metric-strip">
        <Metric label="Entries" value={status?.entryCount ?? "Unknown"} />
        <Metric label="Tags" value={status?.tagCount ?? "Unknown"} />
        <Metric label="This year" value={counts.currentYear ?? "Unknown"} />
        <Metric label="This month" value={counts.currentMonth ?? "Unknown"} />
      </div>

      <div className="dashboard-grid">
        <Panel
          action={<StatusPill tone={statusTone}>{status?.security.mode ?? "unknown"}</StatusPill>}
          icon={<HardDrive size={20} />}
          title="Database"
        >
          <dl className="detail-list">
            <Detail label="Path" value={status?.dbPath ?? "Loading"} />
            <Detail label="Exists" value={status?.dbExists ? "Yes" : "No"} />
            <Detail label="Readable" value={status?.readable ? "Yes" : "No"} />
            <Detail label="Size" value={formatBytes(status?.dbSizeBytes)} />
            <Detail label="Modified" value={formatDateTime(status?.dbModifiedAt)} />
          </dl>
        </Panel>

        <Panel icon={<ShieldCheck size={20} />} title="Backup Safety">
          <dl className="detail-list">
            <Detail label="Directory" value={backupDirectory || "Not available"} />
            <Detail label="Backups" value={status?.backupCount ?? backups.length} />
            <Detail label="Last backup" value={status?.lastBackupPath ?? "No backups found"} />
          </dl>
        </Panel>

        <Panel icon={<BookOpen size={20} />} title="Recent Entries">
          <EntryStack entries={recentEntries} loading={loading} />
        </Panel>

        <Panel icon={<Archive size={20} />} title="Pinned Entries">
          <EntryStack entries={pinnedEntries} emptyText="No pinned entries found." loading={loading} />
        </Panel>

        <Panel
          action={
            <button
              aria-label="Refresh random entry"
              className="icon-button icon-button--small"
              onClick={onRandomRefresh}
              title="Refresh random entry"
              type="button"
            >
              <Shuffle size={16} />
            </button>
          }
          icon={<Shuffle size={20} />}
          title="Random Entry"
        >
          {randomEntry ? (
            <EntryMini entry={randomEntry} />
          ) : (
            <div className="empty-state">No random entry available.</div>
          )}
        </Panel>

        <Panel icon={<TriangleAlert size={20} />} title="Warnings">
          {status?.warnings.length ? (
            <ul className="warning-list">
              {status.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          ) : (
            <p className="muted">No safety warnings for the current read-only status check.</p>
          )}
        </Panel>
      </div>
    </section>
  );
}

type EntriesViewProps = {
  status: DatabaseStatus | null;
  entryFilters: EntryFilterForm;
  setEntryFilters: (next: EntryFilterForm) => void;
  entryResponse: EntryListResponse | null;
  selectedEntry: Entry | null;
  entryHistory: EntryHistoryResponse | null;
  loading: boolean;
  detailLoading: boolean;
  historyLoading: boolean;
  mutatingEntryUuid: string | null;
  onSelectEntry: (entry: Entry) => void;
  onEditEntry: (entry: Entry) => void;
  onContinueEntry: (entry: Entry) => void;
  onEntryAction: (entry: Entry, action: "star" | "pin" | "hide" | "unhide") => void;
  onExportEntry: (entry: Entry, format: ExportFormat) => void;
  onLoadHistory: (entry: Entry) => void;
  onLoadMore: () => void;
  onResetFilters: () => void;
};

function EntriesView({
  status,
  entryFilters,
  setEntryFilters,
  entryResponse,
  selectedEntry,
  entryHistory,
  loading,
  detailLoading,
  historyLoading,
  mutatingEntryUuid,
  onSelectEntry,
  onEditEntry,
  onContinueEntry,
  onEntryAction,
  onExportEntry,
  onLoadHistory,
  onLoadMore,
  onResetFilters,
}: EntriesViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Database is not readable</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  const entries = entryResponse?.entries ?? [];

  return (
    <section className="entries-workspace" aria-label="Entries">
      <aside className="filters-panel">
        <div className="panel-title">
          <Filter size={18} />
          <h3>Filters</h3>
        </div>
        <label className="field">
          <span>Text</span>
          <input
            onChange={(event) => setEntryFilters({ ...entryFilters, text: event.target.value })}
            placeholder="keyword"
            type="search"
            value={entryFilters.text}
          />
        </label>
        <label className="field">
          <span>Tag</span>
          <input
            onChange={(event) => setEntryFilters({ ...entryFilters, tag: event.target.value })}
            placeholder="work, capsule"
            type="text"
            value={entryFilters.tag}
          />
        </label>
        <label className="field">
          <span>Mood</span>
          <input
            onChange={(event) => setEntryFilters({ ...entryFilters, mood: event.target.value })}
            placeholder="happy"
            type="text"
            value={entryFilters.mood}
          />
        </label>
        <label className="field">
          <span>Location</span>
          <input
            onChange={(event) => setEntryFilters({ ...entryFilters, location: event.target.value })}
            placeholder="Oslo, trail, rain"
            type="text"
            value={entryFilters.location}
          />
        </label>
        <div className="field-grid">
          <label className="field">
            <span>Since</span>
            <input
              onChange={(event) => setEntryFilters({ ...entryFilters, since: event.target.value })}
              type="date"
              value={entryFilters.since}
            />
          </label>
          <label className="field">
            <span>Until</span>
            <input
              onChange={(event) => setEntryFilters({ ...entryFilters, until: event.target.value })}
              type="date"
              value={entryFilters.until}
            />
          </label>
        </div>
        <label className="field">
          <span>Sort</span>
          <select
            onChange={(event) =>
              setEntryFilters({ ...entryFilters, sort: event.target.value as "asc" | "desc" })
            }
            value={entryFilters.sort}
          >
            <option value="desc">Newest first</option>
            <option value="asc">Oldest first</option>
          </select>
        </label>
        <label className="check-row">
          <input
            checked={entryFilters.includeHidden}
            onChange={(event) =>
              setEntryFilters({ ...entryFilters, includeHidden: event.target.checked })
            }
            type="checkbox"
          />
          <span>Include hidden</span>
        </label>
        <label className="check-row">
          <input
            checked={entryFilters.hasImages}
            onChange={(event) =>
              setEntryFilters({ ...entryFilters, hasImages: event.target.checked })
            }
            type="checkbox"
          />
          <span>Has images</span>
        </label>
        <button className="secondary-button secondary-button--full" onClick={onResetFilters} type="button">
          Reset filters
        </button>
      </aside>

      <div className="entry-list-panel">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Browse</p>
            <h3>{entryResponse ? `${entryResponse.total} entries` : "Loading entries"}</h3>
          </div>
          <StatusPill tone="neutral">{loading ? "Loading" : "Write-safe"}</StatusPill>
        </div>

        <div className="entry-list">
          {loading && <SkeletonList />}
          {!loading && entries.length === 0 && (
            <div className="empty-state">No entries match the current filters.</div>
          )}
          {!loading &&
            entries.map((entry) => (
              <button
                className={
                  selectedEntry?.uuid === entry.uuid
                    ? "entry-card entry-card--active"
                    : "entry-card"
                }
                key={entry.uuid}
                onClick={() => onSelectEntry(entry)}
                type="button"
              >
                <EntryCardContent entry={entry} />
              </button>
            ))}
        </div>

        {entryResponse && entryResponse.entries.length < entryResponse.total && (
          <button className="secondary-button secondary-button--full" onClick={onLoadMore} type="button">
            Load more
          </button>
        )}
      </div>

      <EntryDetail
        entry={selectedEntry}
        entryHistory={entryHistory}
        historyLoading={historyLoading}
        loading={detailLoading}
        mutating={Boolean(selectedEntry && mutatingEntryUuid === selectedEntry.uuid)}
        onContinue={onContinueEntry}
        onEdit={onEditEntry}
        onEntryAction={onEntryAction}
        onExport={onExportEntry}
        onLoadHistory={onLoadHistory}
      />
    </section>
  );
}

type SearchViewProps = {
  status: DatabaseStatus | null;
  searchForm: SearchForm;
  setSearchForm: (next: SearchForm) => void;
  searchResponse: SearchResponse | null;
  selectedEntry: Entry | null;
  entryHistory: EntryHistoryResponse | null;
  loading: boolean;
  detailLoading: boolean;
  historyLoading: boolean;
  mutatingEntryUuid: string | null;
  onSelectEntry: (entry: Entry) => void;
  onEditEntry: (entry: Entry) => void;
  onContinueEntry: (entry: Entry) => void;
  onEntryAction: (entry: Entry, action: "star" | "pin" | "hide" | "unhide") => void;
  onExportEntry: (entry: Entry, format: ExportFormat) => void;
  onExportSearch: (format: ExportFormat) => void;
  onLoadHistory: (entry: Entry) => void;
  onLoadMore: () => void;
  onResetSearch: () => void;
};

function SearchView({
  status,
  searchForm,
  setSearchForm,
  searchResponse,
  selectedEntry,
  entryHistory,
  loading,
  detailLoading,
  historyLoading,
  mutatingEntryUuid,
  onSelectEntry,
  onEditEntry,
  onContinueEntry,
  onEntryAction,
  onExportEntry,
  onExportSearch,
  onLoadHistory,
  onLoadMore,
  onResetSearch,
}: SearchViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Database is not searchable</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  const entries = searchResponse?.entries ?? [];

  return (
    <section className="entries-workspace" aria-label="Search">
      <aside className="filters-panel">
        <div className="panel-title">
          <Search size={18} />
          <h3>Search</h3>
        </div>
        <label className="field">
          <span>Query</span>
          <input
            onChange={(event) => setSearchForm({ ...searchForm, query: event.target.value })}
            placeholder="keyword tag:work NOT tag:archive"
            type="search"
            value={searchForm.query}
          />
        </label>
        <label className="field">
          <span>Mode</span>
          <select
            onChange={(event) =>
              setSearchForm({ ...searchForm, mode: event.target.value as SearchForm["mode"] })
            }
            value={searchForm.mode}
          >
            <option value="keyword">Keyword</option>
            <option disabled value="semantic">
              Semantic later
            </option>
            <option disabled value="hybrid">
              Hybrid later
            </option>
          </select>
        </label>
        <label className="field">
          <span>Include tags</span>
          <input
            onChange={(event) => setSearchForm({ ...searchForm, tag: event.target.value })}
            placeholder="work, capsule"
            type="text"
            value={searchForm.tag}
          />
        </label>
        <label className="field">
          <span>Exclude tags</span>
          <input
            onChange={(event) => setSearchForm({ ...searchForm, excludeTag: event.target.value })}
            placeholder="archive"
            type="text"
            value={searchForm.excludeTag}
          />
        </label>
        <div className="field-grid">
          <label className="field">
            <span>Mood</span>
            <input
              onChange={(event) => setSearchForm({ ...searchForm, mood: event.target.value })}
              placeholder="focused"
              type="text"
              value={searchForm.mood}
            />
          </label>
          <label className="field">
            <span>Not mood</span>
            <input
              onChange={(event) =>
                setSearchForm({ ...searchForm, excludeMood: event.target.value })
              }
              placeholder="sad"
              type="text"
              value={searchForm.excludeMood}
            />
          </label>
        </div>
        <label className="field">
          <span>Location</span>
          <input
            onChange={(event) => setSearchForm({ ...searchForm, location: event.target.value })}
            placeholder="Oslo, cafe, sunny"
            type="text"
            value={searchForm.location}
          />
        </label>
        <div className="field-grid">
          <label className="field">
            <span>After</span>
            <input
              onChange={(event) => setSearchForm({ ...searchForm, since: event.target.value })}
              type="date"
              value={searchForm.since}
            />
          </label>
          <label className="field">
            <span>Before</span>
            <input
              onChange={(event) => setSearchForm({ ...searchForm, until: event.target.value })}
              type="date"
              value={searchForm.until}
            />
          </label>
        </div>
        <label className="field">
          <span>Sort</span>
          <select
            onChange={(event) =>
              setSearchForm({ ...searchForm, sort: event.target.value as "asc" | "desc" })
            }
            value={searchForm.sort}
          >
            <option value="desc">Newest first</option>
            <option value="asc">Oldest first</option>
          </select>
        </label>
        <label className="check-row">
          <input
            checked={searchForm.includeHidden}
            onChange={(event) =>
              setSearchForm({ ...searchForm, includeHidden: event.target.checked })
            }
            type="checkbox"
          />
          <span>Include hidden</span>
        </label>
        <label className="check-row">
          <input
            checked={searchForm.hasImages}
            onChange={(event) => setSearchForm({ ...searchForm, hasImages: event.target.checked })}
            type="checkbox"
          />
          <span>Has images</span>
        </label>
        <button className="secondary-button secondary-button--full" onClick={onResetSearch} type="button">
          Reset search
        </button>
      </aside>

      <div className="entry-list-panel">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Results</p>
            <h3>{searchResponse ? `${searchResponse.total} entries` : "Loading search"}</h3>
          </div>
          <div className="topbar-actions">
            <button
              className="icon-button icon-button--small"
              disabled={!searchResponse || searchResponse.total === 0}
              onClick={() => onExportSearch("markdown")}
              title="Export search results as Markdown"
              type="button"
            >
              <Download size={15} />
            </button>
            <button
              className="secondary-button secondary-button--small"
              disabled={!searchResponse || searchResponse.total === 0}
              onClick={() => onExportSearch("json")}
              type="button"
            >
              JSON
            </button>
            <StatusPill tone={searchResponse?.usedFts ? "good" : "neutral"}>
              {searchResponse?.usedFts ? "FTS" : "Keyword"}
            </StatusPill>
          </div>
        </div>

        {searchResponse?.parsedTokens.length ? (
          <div className="token-row">
            {searchResponse.parsedTokens.map((token, index) => (
              <span className="tag-chip" key={`${token.kind}-${token.value}-${index}`}>
                {token.kind}: {token.value}
              </span>
            ))}
          </div>
        ) : null}

        {searchResponse?.warnings.map((warning) => (
          <div className="inline-warning" key={warning}>
            <TriangleAlert size={15} />
            {warning}
          </div>
        ))}

        <div className="entry-list">
          {loading && <SkeletonList />}
          {!loading && entries.length === 0 && (
            <div className="empty-state">No entries match the current search.</div>
          )}
          {!loading &&
            entries.map((entry) => (
              <button
                className={
                  selectedEntry?.uuid === entry.uuid ? "entry-card entry-card--active" : "entry-card"
                }
                key={entry.uuid}
                onClick={() => onSelectEntry(entry)}
                type="button"
              >
                <EntryCardContent entry={entry} />
              </button>
            ))}
        </div>

        {searchResponse && searchResponse.entries.length < searchResponse.total && (
          <button className="secondary-button secondary-button--full" onClick={onLoadMore} type="button">
            Load more
          </button>
        )}
      </div>

      <EntryDetail
        entry={selectedEntry}
        entryHistory={entryHistory}
        historyLoading={historyLoading}
        loading={detailLoading}
        mutating={Boolean(selectedEntry && mutatingEntryUuid === selectedEntry.uuid)}
        onContinue={onContinueEntry}
        onEdit={onEditEntry}
        onEntryAction={onEntryAction}
        onExport={onExportEntry}
        onLoadHistory={onLoadHistory}
      />
    </section>
  );
}

type ImagesViewProps = {
  status: DatabaseStatus | null;
  entryResponse: EntryListResponse | null;
  selectedEntry: Entry | null;
  entryImages: ImageEntryListResponse | null;
  draft: ImageUploadDraft;
  loading: boolean;
  detailLoading: boolean;
  mutating: boolean;
  onSelectEntry: (entry: Entry) => void;
  onLoadMore: () => void;
  onChangeDraft: (next: ImageUploadDraft) => void;
  onAttach: () => void;
  onRemoveImage: (attachment: ImageAttachment) => void;
};

function ImagesView({
  status,
  entryResponse,
  selectedEntry,
  entryImages,
  draft,
  loading,
  detailLoading,
  mutating,
  onSelectEntry,
  onLoadMore,
  onChangeDraft,
  onAttach,
  onRemoveImage,
}: ImagesViewProps) {
  const [expandedImage, setExpandedImage] = useState<ImageAttachment | null>(null);

  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Images are not available</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  const entries = entryResponse?.entries ?? [];
  const images = entryImages?.images ?? [];

  return (
    <section className="images-workspace" aria-label="Images">
      <aside className="filters-panel">
        <div className="panel-title">
          <FileImage size={18} />
          <h3>Image Entries</h3>
        </div>
        <div className="image-entry-list">
          {loading && <SkeletonList compact />}
          {!loading && entries.length === 0 && (
            <div className="empty-state">No image attachments found.</div>
          )}
          {!loading &&
            entries.map((entry) => (
              <button
                className={
                  selectedEntry?.uuid === entry.uuid
                    ? "entry-card entry-card--active"
                    : "entry-card"
                }
                key={entry.uuid}
                onClick={() => onSelectEntry(entry)}
                type="button"
              >
                <EntryCardContent entry={entry} />
              </button>
            ))}
        </div>
        {entryResponse && entryResponse.entries.length < entryResponse.total && (
          <button className="secondary-button secondary-button--full" onClick={onLoadMore} type="button">
            Load more
          </button>
        )}
      </aside>

      <div className="media-panel">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Attachments</p>
            <h3>
              {selectedEntry
                ? selectedEntry.title || selectedEntry.textPlain.slice(0, 72) || selectedEntry.uuid
                : "Select an entry"}
            </h3>
          </div>
          <StatusPill tone="neutral">{detailLoading ? "Loading" : `${images.length} images`}</StatusPill>
        </div>

        {selectedEntry ? (
          <>
            <div className="upload-strip">
              <label className="field field--wide">
                <span>Local image path</span>
                <input
                  onChange={(event) => onChangeDraft({ ...draft, path: event.target.value })}
                  placeholder="C:\\Users\\jtill\\OneDrive\\_capsule\\images\\photo.jpg"
                  value={draft.path}
                />
              </label>
              <label className="field">
                <span>Caption</span>
                <input
                  onChange={(event) => onChangeDraft({ ...draft, caption: event.target.value })}
                  value={draft.caption}
                />
              </label>
              <label className="field">
                <span>Alt text</span>
                <input
                  onChange={(event) => onChangeDraft({ ...draft, altText: event.target.value })}
                  value={draft.altText}
                />
              </label>
              <button
                className="primary-button"
                disabled={mutating || !draft.path.trim()}
                onClick={onAttach}
                type="button"
              >
                <Upload size={17} />
                Attach
              </button>
            </div>

            {entryImages?.warnings.map((warning) => (
              <div className="inline-warning" key={warning}>
                <TriangleAlert size={15} />
                {warning}
              </div>
            ))}

            {detailLoading && <SkeletonList />}
            {!detailLoading && images.length === 0 && (
              <div className="empty-state">This entry does not have attached images yet.</div>
            )}
            {!detailLoading && images.length > 0 && (
              <div className="gallery-grid">
                {images.map((attachment) => (
                  <article className="media-tile" key={attachment.attachmentId}>
                    <button
                      className="media-tile-preview"
                      onClick={() => setExpandedImage(attachment)}
                      type="button"
                    >
                      <DataUrlImage
                        attachment={attachment}
                        className="media-thumb"
                        variant="thumb"
                      />
                    </button>
                    <div className="media-tile-body">
                      <h4>{attachment.caption || attachment.altText || attachment.hash.slice(0, 12)}</h4>
                      <p>
                        {attachment.width} x {attachment.height} / {formatBytes(attachment.bytes)}
                      </p>
                      <div className="backup-actions">
                        <button
                          className="icon-button icon-button--small"
                          onClick={() => setExpandedImage(attachment)}
                          title="View full image"
                          type="button"
                        >
                          <Maximize2 size={15} />
                        </button>
                        <button
                          className="icon-button icon-button--small"
                          disabled={mutating}
                          onClick={() => onRemoveImage(attachment)}
                          title="Remove attachment"
                          type="button"
                        >
                          <Trash2 size={15} />
                        </button>
                      </div>
                    </div>
                  </article>
                ))}
              </div>
            )}
          </>
        ) : (
          <div className="empty-state">Choose an entry with images to inspect attachments.</div>
        )}
      </div>

      {expandedImage && (
        <ImageLightbox
          attachment={expandedImage}
          onClose={() => setExpandedImage(null)}
        />
      )}
    </section>
  );
}

type AnalyticsViewProps = {
  status: DatabaseStatus | null;
  analytics: AnalyticsResponse | null;
  period: PeriodForm;
  setPeriod: (next: PeriodForm) => void;
  loading: boolean;
  onRefresh: () => void;
};

function AnalyticsView({
  status,
  analytics,
  period,
  setPeriod,
  loading,
  onRefresh,
}: AnalyticsViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Analytics are not available</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  return (
    <section className="analytics-workspace" aria-label="Analytics">
      <div className="toolbar-panel">
        <div className="panel-title">
          <BarChart3 size={18} />
          <h3>Period</h3>
        </div>
        <div className="inline-form">
          <label className="field">
            <span>Since</span>
            <input
              onChange={(event) => setPeriod({ ...period, since: event.target.value })}
              type="date"
              value={period.since}
            />
          </label>
          <label className="field">
            <span>Until</span>
            <input
              onChange={(event) => setPeriod({ ...period, until: event.target.value })}
              type="date"
              value={period.until}
            />
          </label>
          <button className="secondary-button" disabled={loading} onClick={onRefresh} type="button">
            <RefreshCw size={17} />
            Refresh
          </button>
        </div>
      </div>

      {loading && <SkeletonList />}
      {!loading && !analytics && <div className="empty-state">Analytics will appear after loading.</div>}
      {analytics && (
        <>
          <div className="metric-strip metric-strip--six">
            <Metric label="Entries" value={analytics.overview.totalEntries} />
            <Metric label="Words" value={analytics.overview.totalWords} />
            <Metric label="Avg words" value={analytics.overview.averageWords} />
            <Metric label="Images" value={analytics.overview.totalImages} />
            <Metric label="Location" value={analytics.overview.entriesWithLocation} />
            <Metric label="Streak" value={`${analytics.overview.currentStreakDays}d`} />
          </div>

          {analytics.warnings.map((warning) => (
            <div className="inline-warning" key={warning}>
              <TriangleAlert size={15} />
              {warning}
            </div>
          ))}

          <div className="analytics-grid">
            <Panel icon={<BarChart3 size={20} />} title="Monthly Trend">
              <TrendBars trend={analytics.monthlyTrend} />
            </Panel>
            <Panel icon={<Tags size={20} />} title="Tags">
              <BreakdownList items={analytics.tagBreakdown} />
            </Panel>
            <Panel icon={<Sparkles size={20} />} title="Moods">
              <BreakdownList items={analytics.moodBreakdown} />
            </Panel>
            <Panel icon={<MapPin size={20} />} title="Locations">
              <BreakdownList items={analytics.locationBreakdown} emptyText="No locations in this period." />
            </Panel>
            <Panel icon={<Cloud size={20} />} title="Weather">
              <BreakdownList items={analytics.weatherBreakdown} emptyText="No weather metadata in this period." />
            </Panel>
            <Panel icon={<FileText size={20} />} title="Top Words">
              <div className="word-cloud">
                {analytics.topWords.length === 0 && <p className="muted">No words counted.</p>}
                {analytics.topWords.slice(0, 18).map((item) => (
                  <span className="tag-chip" key={item.word}>
                    {item.word}
                    <strong>{item.count}</strong>
                  </span>
                ))}
              </div>
            </Panel>
          </div>
        </>
      )}
    </section>
  );
}

type WritingCalendarViewProps = {
  status: DatabaseStatus | null;
  calendar: WritingCalendarResponse | null;
  year: number;
  setYear: (year: number) => void;
  loading: boolean;
  onRefresh: () => void;
};

function WritingCalendarView({
  status,
  calendar,
  year,
  setYear,
  loading,
  onRefresh,
}: WritingCalendarViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Writing calendar is not available</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  const months = buildCalendarMonths(calendar?.year ?? year, calendar?.days ?? []);

  return (
    <section className="calendar-workspace" aria-label="Writing calendar">
      <div className="toolbar-panel">
        <div className="panel-title">
          <CalendarDays size={18} />
          <h3>{calendar?.year ?? year}</h3>
        </div>
        <div className="inline-form inline-form--compact">
          <button className="icon-button" onClick={() => setYear(year - 1)} title="Previous year" type="button">
            <ChevronLeft size={18} />
          </button>
          <label className="field field--year">
            <span>Year</span>
            <input
              max={9999}
              min={1}
              onChange={(event) => setYear(Number(event.target.value) || new Date().getFullYear())}
              type="number"
              value={year}
            />
          </label>
          <button className="icon-button" onClick={() => setYear(year + 1)} title="Next year" type="button">
            <ChevronRight size={18} />
          </button>
          <button className="secondary-button" disabled={loading} onClick={onRefresh} type="button">
            <RefreshCw size={17} />
            Refresh
          </button>
        </div>
      </div>

      {loading && <SkeletonList />}
      {calendar && (
        <>
          <div className="metric-strip">
            <Metric label="Active days" value={calendar.activeDays} />
            <Metric label="Total days" value={calendar.totalDays} />
            <Metric label="Max entries" value={calendar.maxEntryCount} />
            <Metric label="Coverage" value={`${Math.round((calendar.activeDays / Math.max(calendar.totalDays, 1)) * 100)}%`} />
          </div>
          {calendar.warnings.map((warning) => (
            <div className="inline-warning" key={warning}>
              <TriangleAlert size={15} />
              {warning}
            </div>
          ))}
          <div className="calendar-grid">
            {months.map((month) => (
              <article className="calendar-month" key={month.label}>
                <h4>{month.label}</h4>
                <div className="calendar-weekdays" aria-hidden="true">
                  {["S", "M", "T", "W", "T", "F", "S"].map((day, index) => (
                    <span key={`${day}-${index}`}>{day}</span>
                  ))}
                </div>
                <div className="calendar-days">
                  {Array.from({ length: month.blanks }).map((_, index) => (
                    <span className="calendar-day calendar-day--empty" key={`blank-${index}`} />
                  ))}
                  {month.days.map((day) => {
                    const level = calendarLevel(day.data, calendar.maxEntryCount);
                    return (
                      <span
                        className={`calendar-day calendar-day--level-${level}`}
                        key={day.date}
                        title={calendarDayTitle(day.date, day.data)}
                      >
                        {day.day}
                      </span>
                    );
                  })}
                </div>
              </article>
            ))}
          </div>
        </>
      )}
      {!loading && !calendar && <div className="empty-state">Calendar data will appear after loading.</div>}
    </section>
  );
}

type CoverWallViewProps = {
  status: DatabaseStatus | null;
  coverWall: CoverWallResponse | null;
  selectedCover: EntryCover | null;
  filters: CoverWallFilters;
  limit: number;
  loading: boolean;
  setFilters: (next: CoverWallFilters) => void;
  onSelectCover: (cover: EntryCover) => void;
  onLoadMore: () => void;
  onRefresh: () => void;
};

function CoverWallView({
  status,
  coverWall,
  selectedCover,
  filters,
  limit,
  loading,
  setFilters,
  onSelectCover,
  onLoadMore,
  onRefresh,
}: CoverWallViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Cover wall is not available</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  const covers = coverWall?.covers ?? [];

  return (
    <section className="covers-workspace" aria-label="Cover wall">
      <aside className="filters-panel">
        <div className="panel-title">
          <Images size={18} />
          <h3>Cover Wall</h3>
        </div>
        <label className="field">
          <span>Type</span>
          <select
            onChange={(event) => setFilters({ ...filters, coverType: event.target.value })}
            value={filters.coverType}
          >
            <option value="">All types</option>
            {(coverWall?.availableTypes ?? []).map((coverType) => (
              <option key={coverType} value={coverType}>
                {coverType}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>Tags</span>
          <input
            onChange={(event) => setFilters({ ...filters, tag: event.target.value })}
            placeholder="work, capsule"
            value={filters.tag}
          />
        </label>
        <label className="field">
          <span>Moods</span>
          <input
            onChange={(event) => setFilters({ ...filters, mood: event.target.value })}
            placeholder="focused"
            value={filters.mood}
          />
        </label>
        <div className="field-grid">
          <label className="field">
            <span>Since</span>
            <input
              onChange={(event) => setFilters({ ...filters, since: event.target.value })}
              type="date"
              value={filters.since}
            />
          </label>
          <label className="field">
            <span>Until</span>
            <input
              onChange={(event) => setFilters({ ...filters, until: event.target.value })}
              type="date"
              value={filters.until}
            />
          </label>
        </div>
        <button className="secondary-button secondary-button--full" disabled={loading} onClick={onRefresh} type="button">
          <RefreshCw size={17} />
          Refresh
        </button>
        {coverWall && (
          <dl className="detail-list detail-list--compact">
            <Detail label="Root" value={coverWall.coversRoot} />
            <Detail label="Orphans" value={coverWall.orphanedCoverCount} />
            <Detail label="Showing" value={`${covers.length} / ${coverWall.total}`} />
          </dl>
        )}
      </aside>

      <div className="cover-wall-panel">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Visual index</p>
            <h3>{coverWall ? `${coverWall.total} linked covers` : "Loading covers"}</h3>
          </div>
          <StatusPill tone="neutral">{loading ? "Loading" : `${limit} limit`}</StatusPill>
        </div>

        {loading && <SkeletonList />}
        {!loading && covers.length === 0 && (
          <div className="empty-state">No linked covers match these filters.</div>
        )}
        {!loading && covers.length > 0 && (
          <div className="cover-grid">
            {covers.map((cover) => (
              <button
                className={
                  selectedCover?.filename === cover.filename
                    ? "cover-tile cover-tile--active"
                    : "cover-tile"
                }
                key={cover.filename}
                onClick={() => onSelectCover(cover)}
                type="button"
              >
                <CoverImage className="cover-thumb" filename={cover.filename} variant="thumb" />
                <span>{cover.entry.title || cover.entry.uuid}</span>
              </button>
            ))}
          </div>
        )}
        {coverWall && coverWall.covers.length < coverWall.total && (
          <button className="secondary-button secondary-button--full" onClick={onLoadMore} type="button">
            Load more
          </button>
        )}
      </div>

      <aside className="detail-panel cover-detail-panel">
        {selectedCover ? (
          <>
            <div className="entry-detail-heading">
              <p className="eyebrow">{selectedCover.coverType}</p>
              <h3>{selectedCover.entry.title || selectedCover.entry.uuid}</h3>
            </div>
            <CoverImage className="cover-full-preview" filename={selectedCover.filename} variant="full" />
            <dl className="detail-list detail-list--compact">
              <Detail label="Entry" value={selectedCover.entry.uuid} />
              <Detail label="Date" value={formatDateTime(selectedCover.entry.createdAt)} />
              <Detail label="File" value={selectedCover.filename} />
              <Detail label="Size" value={formatBytes(selectedCover.bytes)} />
              <Detail label="Modified" value={formatDateTime(selectedCover.modifiedAt)} />
            </dl>
            <div className="tag-row">
              {selectedCover.entry.mood && <span className="mood-chip">{selectedCover.entry.mood}</span>}
              {selectedCover.entry.tags.map((tag) => (
                <span className="tag-chip" key={tag}>
                  <Tags size={12} />
                  {tag}
                </span>
              ))}
            </div>
          </>
        ) : (
          <div className="detail-panel--empty">
            <Images size={22} />
            <h3>No cover selected</h3>
            <p>Select a cover to inspect its linked entry.</p>
          </div>
        )}
      </aside>
    </section>
  );
}

type ThreadsViewProps = {
  status: DatabaseStatus | null;
  response: ThreadListResponse | null;
  selectedThread: ThreadGroup | null;
  draft: ThreadMetadataDraft;
  setDraft: (next: ThreadMetadataDraft) => void;
  loading: boolean;
  saving: boolean;
  mutatingEntryUuid: string | null;
  onSelectThread: (rootUuid: string) => void;
  onSaveMetadata: (thread: ThreadGroup) => void;
  onDisbandThread: (thread: ThreadGroup) => void;
  onEditEntry: (entry: Entry) => void;
  onContinueEntry: (entry: Entry) => void;
  onDetachEntry: (entry: Entry) => void;
};

function ThreadsView({
  status,
  response,
  selectedThread,
  draft,
  setDraft,
  loading,
  saving,
  mutatingEntryUuid,
  onSelectThread,
  onSaveMetadata,
  onDisbandThread,
  onEditEntry,
  onContinueEntry,
  onDetachEntry,
}: ThreadsViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Database is not readable</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  const threads = response?.threads ?? [];

  return (
    <section className="threads-workspace" aria-label="Threads">
      <div className="thread-list-panel">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Continuation groups</p>
            <h3>{response ? `${response.total} threads` : "Loading threads"}</h3>
          </div>
          <StatusPill tone="neutral">{loading ? "Loading" : "UUID identity"}</StatusPill>
        </div>

        <div className="thread-list">
          {loading && <SkeletonList />}
          {!loading && threads.length === 0 && (
            <div className="empty-state">No continuation threads found.</div>
          )}
          {!loading &&
            threads.map((thread) => (
              <button
                className={
                  selectedThread?.rootUuid === thread.rootUuid
                    ? "thread-card thread-card--active"
                    : "thread-card"
                }
                key={thread.rootUuid}
                onClick={() => onSelectThread(thread.rootUuid)}
                type="button"
              >
                <div className="thread-card-heading">
                  <GitBranch size={17} />
                  <div>
                    <h4>{thread.title || thread.rootUuid}</h4>
                    <p>{thread.summary || thread.entries[0]?.textPlain.slice(0, 120)}</p>
                  </div>
                </div>
                <div className="entry-meta">
                  <span className="tag-chip">{thread.entryCount} entries</span>
                  <span className="tag-chip">{formatDateTime(thread.latestActivity)}</span>
                </div>
              </button>
            ))}
        </div>
      </div>

      <aside className="thread-detail-panel">
        {!selectedThread ? (
          <div className="detail-panel--empty">
            <GitBranch size={22} />
            <h3>No thread selected</h3>
            <p>Select a thread to inspect continuation order.</p>
          </div>
        ) : (
          <>
            <div className="section-heading">
              <div>
                <p className="eyebrow">{selectedThread.rootUuid}</p>
                <h3>{selectedThread.title || "Untitled thread"}</h3>
              </div>
              <StatusPill tone="good">{selectedThread.entryCount} entries</StatusPill>
            </div>

            <div className="thread-metadata-editor">
              <label className="field">
                <span>Title</span>
                <input
                  onChange={(event) => setDraft({ ...draft, title: event.target.value })}
                  placeholder="Thread title"
                  type="text"
                  value={draft.title}
                />
              </label>
              <label className="field">
                <span>Summary</span>
                <textarea
                  className="compact-textarea"
                  onChange={(event) => setDraft({ ...draft, summary: event.target.value })}
                  placeholder="Thread summary"
                  value={draft.summary}
                />
              </label>
              <div className="entry-action-bar">
                <button
                  className="primary-button"
                  disabled={saving}
                  onClick={() => onSaveMetadata(selectedThread)}
                  type="button"
                >
                  <Save size={17} />
                  {saving ? "Saving" : "Save"}
                </button>
                <button
                  className="secondary-button"
                  disabled={saving}
                  onClick={() => onDisbandThread(selectedThread)}
                  type="button"
                >
                  <Unlink2 size={17} />
                  Disband
                </button>
              </div>
            </div>

            <div className="thread-entry-list">
              {selectedThread.entries.map((entry, index) => {
                const canDetach =
                  !entry.thread?.isRoot && isThreadLeaf(selectedThread, entry);
                return (
                  <article className="thread-entry-row" key={entry.uuid}>
                    <div className="thread-entry-index">{index + 1}</div>
                    <div className="thread-entry-main">
                      <h4>{entry.title || entry.textPlain.slice(0, 86) || "Untitled entry"}</h4>
                      <p>{entry.textPlain.slice(0, 180)}</p>
                      <EntryMeta entry={entry} />
                    </div>
                    <div className="thread-entry-actions">
                      <button
                        className="icon-button icon-button--small"
                        onClick={() => onContinueEntry(entry)}
                        title="Continue from entry"
                        type="button"
                      >
                        <Link2 size={15} />
                      </button>
                      <button
                        className="icon-button icon-button--small"
                        onClick={() => onEditEntry(entry)}
                        title="Edit entry"
                        type="button"
                      >
                        <Edit3 size={15} />
                      </button>
                      <button
                        className="icon-button icon-button--small"
                        disabled={!canDetach || mutatingEntryUuid === entry.uuid}
                        onClick={() => onDetachEntry(entry)}
                        title={canDetach ? "Detach leaf entry" : "Only leaf continuations can detach here"}
                        type="button"
                      >
                        <Unlink2 size={15} />
                      </button>
                    </div>
                  </article>
                );
              })}
            </div>
          </>
        )}
      </aside>
    </section>
  );
}

type ComposerViewProps = {
  status: DatabaseStatus | null;
  mode: ComposerMode;
  editingEntry: Entry | null;
  draft: ComposerDraft;
  onChange: (next: ComposerDraft) => void;
  onSave: () => void;
  onCancel: () => void;
  onOpenWriter: () => void;
  saving: boolean;
};

function ComposerView({
  status,
  mode,
  editingEntry,
  draft,
  onChange,
  onSave,
  onCancel,
  onOpenWriter,
  saving,
}: ComposerViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <TriangleAlert size={22} />
        <h3>Database is not writable</h3>
        <p>{status.security.message ?? "Confirm the active database before writing."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  const stats = writingStats(draft.text);

  return (
    <section className="composer-view" aria-label={mode === "edit" ? "Edit entry" : "New entry"}>
      <div className="composer-main">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{mode === "edit" ? editingEntry?.uuid : "Markdown"}</p>
            <h3>{mode === "edit" ? "Edit Entry" : "New Entry"}</h3>
          </div>
          <div className="topbar-actions">
            <button className="secondary-button" onClick={onOpenWriter} type="button">
              <Maximize2 size={17} />
              Writer
            </button>
            <button className="secondary-button" onClick={onCancel} type="button">
              <X size={17} />
              Cancel
            </button>
            <button
              className="primary-button"
              disabled={saving || !draft.text.trim()}
              onClick={onSave}
              type="button"
            >
              <Save size={17} />
              {saving ? "Saving" : "Save"}
            </button>
          </div>
        </div>

        <label className="field composer-title-field">
          <span>Title</span>
          <input
            onChange={(event) => onChange({ ...draft, title: event.target.value })}
            placeholder="Optional title"
            type="text"
            value={draft.title}
          />
        </label>

        <label className="field composer-text-field">
          <span>Entry</span>
          <textarea
            autoFocus
            onChange={(event) => onChange({ ...draft, text: event.target.value })}
            placeholder="Write the entry"
            value={draft.text}
          />
        </label>
      </div>

      <aside className="composer-side">
        <Panel icon={<FileText size={20} />} title="Metadata">
          <div className="composer-meta-grid">
            <label className="field">
              <span>Summary</span>
              <textarea
                className="compact-textarea"
                onChange={(event) => onChange({ ...draft, summary: event.target.value })}
                placeholder="Optional summary"
                value={draft.summary}
              />
            </label>
            <label className="field">
              <span>Mood</span>
              <input
                onChange={(event) => onChange({ ...draft, mood: event.target.value })}
                placeholder="focused"
                type="text"
                value={draft.mood}
              />
            </label>
            <label className="field">
              <span>Tags</span>
              <input
                onChange={(event) => onChange({ ...draft, tags: event.target.value })}
                placeholder="work, capsule"
                type="text"
                value={draft.tags}
              />
            </label>
            {mode === "create" && (
              <label className="field">
                <span>When</span>
                <input
                  onChange={(event) => onChange({ ...draft, when: event.target.value })}
                  type="datetime-local"
                  value={draft.when}
                />
              </label>
            )}
            <label className="field">
              <span>Continue from UUID</span>
              <input
                onChange={(event) => onChange({ ...draft, continueFromUuid: event.target.value })}
                placeholder="entry_xxxxxxxx"
                type="text"
                value={draft.continueFromUuid}
              />
            </label>
            <label className="check-row">
              <input
                checked={draft.starred}
                onChange={(event) => onChange({ ...draft, starred: event.target.checked })}
                type="checkbox"
              />
              <span>Starred</span>
            </label>
            <label className="check-row">
              <input
                checked={draft.pinned}
                onChange={(event) => onChange({ ...draft, pinned: event.target.checked })}
                type="checkbox"
              />
              <span>Pinned</span>
            </label>
          </div>
        </Panel>

        <Panel icon={<Clock3 size={20} />} title="Writing Stats">
          <div className="mini-metrics">
            <Metric label="Words" value={stats.words} />
            <Metric label="Characters" value={stats.characters} />
            <Metric label="Reading" value={`${stats.readingMinutes} min`} />
          </div>
        </Panel>
      </aside>
    </section>
  );
}

type WriterModeViewProps = {
  mode: ComposerMode;
  draft: ComposerDraft;
  onChange: (next: ComposerDraft) => void;
  onSave: () => void;
  onExit: () => void;
  saving: boolean;
  settings: WriterSettings;
  setSettings: (next: WriterSettings) => void;
  error: string | null;
  notice: string | null;
};

function WriterModeView({
  mode,
  draft,
  onChange,
  onSave,
  onExit,
  saving,
  settings,
  setSettings,
  error,
  notice,
}: WriterModeViewProps) {
  const stats = writingStats(draft.text);
  return (
    <main
      className="writer-mode"
      style={{
        background: settings.background,
        color: settings.color,
        fontFamily: settings.fontFamily,
      }}
    >
      <div className="writer-toolbar">
        <div>
          <p className="eyebrow">{mode === "edit" ? "Edit" : "New"}</p>
          <h1>{draft.title || "Untitled"}</h1>
        </div>
        <div className="writer-controls">
          <label title="Background color">
            <input
              onChange={(event) => setSettings({ ...settings, background: event.target.value })}
              type="color"
              value={settings.background}
            />
          </label>
          <label title="Text color">
            <input
              onChange={(event) => setSettings({ ...settings, color: event.target.value })}
              type="color"
              value={settings.color}
            />
          </label>
          <select
            onChange={(event) => setSettings({ ...settings, fontFamily: event.target.value })}
            value={settings.fontFamily}
          >
            <option value="Georgia, ui-serif, serif">Serif</option>
            <option value="Inter, Segoe UI, ui-sans-serif, sans-serif">Sans</option>
            <option value="Cascadia Code, ui-monospace, monospace">Mono</option>
          </select>
          <input
            max={28}
            min={16}
            onChange={(event) => setSettings({ ...settings, fontSize: Number(event.target.value) })}
            title="Font size"
            type="range"
            value={settings.fontSize}
          />
          <input
            max={2.2}
            min={1.3}
            onChange={(event) =>
              setSettings({ ...settings, lineSpacing: Number(event.target.value) })
            }
            step={0.05}
            title="Line spacing"
            type="range"
            value={settings.lineSpacing}
          />
          <button className="secondary-button" onClick={onExit} type="button">
            <X size={17} />
            Exit
          </button>
          <button className="primary-button" disabled={saving || !draft.text.trim()} onClick={onSave} type="button">
            <Save size={17} />
            {saving ? "Saving" : "Save"}
          </button>
        </div>
      </div>

      {error && (
        <div className="writer-banner writer-banner--error">
          <TriangleAlert size={18} />
          {error}
        </div>
      )}
      {notice && (
        <div className="writer-banner writer-banner--success">
          <CheckCircle2 size={18} />
          {notice}
        </div>
      )}

      <div className="writer-canvas">
        <input
          className="writer-title-input"
          onChange={(event) => onChange({ ...draft, title: event.target.value })}
          placeholder="Title"
          style={{ color: settings.color }}
          value={draft.title}
        />
        <textarea
          autoFocus
          className="writer-textarea"
          onChange={(event) => onChange({ ...draft, text: event.target.value })}
          placeholder="Write"
          style={{
            color: settings.color,
            fontFamily: settings.fontFamily,
            fontSize: settings.fontSize,
            lineHeight: settings.lineSpacing,
          }}
          value={draft.text}
        />
      </div>

      <div className="writer-footer">
        <span>{stats.words} words</span>
        <span>{stats.characters} characters</span>
        <span>{stats.readingMinutes} min</span>
      </div>
    </main>
  );
}

type BackupsViewProps = {
  backupDirectory: string;
  backups: BackupInfo[];
  status: DatabaseStatus | null;
  creatingBackup: boolean;
  restoringBackup: boolean;
  restorePreview: BackupRestorePreview | null;
  onCreateBackup: () => void;
  onOpenFolder: () => void;
  onPreviewRestore: (backup: BackupInfo) => void;
  onRestoreBackup: (backup: BackupInfo) => void;
};

function BackupsView({
  backupDirectory,
  backups,
  status,
  creatingBackup,
  restoringBackup,
  restorePreview,
  onCreateBackup,
  onOpenFolder,
  onPreviewRestore,
  onRestoreBackup,
}: BackupsViewProps) {
  return (
    <section className="backup-view" aria-label="Backup list">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Backup directory</p>
          <h3>{backupDirectory || "Not available"}</h3>
        </div>
        <div className="topbar-actions">
          <button className="secondary-button" onClick={onOpenFolder} type="button">
            <FolderOpen size={18} />
            Open folder
          </button>
          <button
            className="secondary-button"
            disabled={creatingBackup || !status?.dbExists}
            onClick={onCreateBackup}
            type="button"
          >
            <FileArchive size={18} />
            {creatingBackup ? "Creating" : "Create backup"}
          </button>
        </div>
      </div>

      {restorePreview && (
        <Panel
          action={<StatusPill tone={restorePreview.backup.verified ? "good" : "warn"}>{restorePreview.backup.verified ? "Verified" : "Check"}</StatusPill>}
          icon={<ShieldCheck size={20} />}
          title="Restore Preview"
        >
          <dl className="detail-list">
            <Detail label="Backup" value={restorePreview.backup.path} />
            <Detail label="Entries" value={restorePreview.entryCount ?? "Unknown"} />
            <Detail label="Tags" value={restorePreview.tagCount ?? "Unknown"} />
            <Detail label="Size" value={formatBytes(restorePreview.dbSizeBytes)} />
            <Detail label="Modified" value={formatDateTime(restorePreview.dbModifiedAt)} />
          </dl>
          {restorePreview.warnings.length > 0 && (
            <ul className="warning-list">
              {restorePreview.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          )}
        </Panel>
      )}

      <div className="backup-list">
        {backups.length === 0 && <div className="empty-state">No Capsule backups found yet.</div>}
        {backups.map((backup) => (
          <article className="backup-row" key={backup.path}>
            <div>
              <h4>{backup.path}</h4>
              <p>
                {formatBytes(backup.sizeBytes)} / {formatDateTime(backup.createdAt)} /{" "}
                {backup.operation ?? "unknown operation"}
              </p>
            </div>
            <div className="backup-actions">
              <button
                className="secondary-button secondary-button--small"
                onClick={() => onPreviewRestore(backup)}
                type="button"
              >
                Preview
              </button>
              <button
                className="secondary-button secondary-button--small"
                disabled={restoringBackup || !backup.verified}
                onClick={() => onRestoreBackup(backup)}
                type="button"
              >
                Restore
              </button>
              <StatusPill tone={backup.verified ? "good" : "warn"}>
                {backup.verified ? "Verified" : "Needs check"}
              </StatusPill>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

type SettingsViewProps = {
  status: DatabaseStatus | null;
  backupDirectory: string;
  statusTone: "good" | "warn" | "neutral";
  config: CapsuleConfigResponse | null;
  tagCatalog: TagCatalogResponse | null;
  moodCatalog: MoodCatalogResponse | null;
  library: LibraryListResponse | null;
  uiSettings: UiSettings;
  setUiSettings: (next: UiSettings) => void;
  loading: boolean;
  dataToolMutating: boolean;
  onRefresh: () => void;
  onRunMutation: (mutation: () => Promise<string>) => Promise<void>;
};

function SettingsView({
  status,
  backupDirectory,
  statusTone,
  config,
  tagCatalog,
  moodCatalog,
  library,
  uiSettings,
  setUiSettings,
  loading,
  dataToolMutating,
  onRefresh,
  onRunMutation,
}: SettingsViewProps) {
  const [configDraft, setConfigDraft] = useState({ key: "", value: "" });
  const [tagDraft, setTagDraft] = useState({ from: "", to: "", source: "", target: "", deleteName: "" });
  const [moodDraft, setMoodDraft] = useState({ from: "", to: "", deleteName: "" });
  const [templateDraft, setTemplateDraft] = useState({
    slug: "",
    name: "",
    description: "",
    introText: "",
    sections: "",
  });
  const [promptDraft, setPromptDraft] = useState({
    slug: "",
    promptText: "",
    category: "general",
    tags: "",
  });

  return (
    <section className="settings-grid" aria-label="Settings">
      <Panel action={<StatusPill tone={statusTone}>{status?.security.mode ?? "unknown"}</StatusPill>} icon={<Database size={20} />} title="Database">
        <dl className="detail-list">
          <Detail label="Path" value={status?.dbPath ?? "Loading"} />
          <Detail label="Readable" value={status?.readable ? "Yes" : "No"} />
          <Detail label="Security" value={status?.security.message ?? status?.security.mode ?? "Unknown"} />
          <Detail label="Backups" value={backupDirectory || "Not available"} />
        </dl>
      </Panel>

      <Panel
        action={
          <button className="secondary-button secondary-button--small" disabled={loading} onClick={onRefresh} type="button">
            <RefreshCw size={15} />
            Refresh
          </button>
        }
        icon={<Info size={20} />}
        title="Application"
      >
        <dl className="detail-list">
          <Detail label="Version" value="0.5.0" />
          <Detail label="Mode" value="Backups and data tools" />
          <Detail label="Writes" value="Backup guarded" />
        </dl>
      </Panel>

      <Panel icon={<Settings size={20} />} title="Interface">
        <div className="settings-form-grid">
          <label className="field">
            <span>Theme</span>
            <select
              onChange={(event) =>
                setUiSettings({ ...uiSettings, theme: event.target.value as UiSettings["theme"] })
              }
              value={uiSettings.theme}
            >
              <option value="system">System</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </label>
          <label className="field">
            <span>Sidebar</span>
            <select
              onChange={(event) =>
                setUiSettings({
                  ...uiSettings,
                  sidebarMode: event.target.value as UiSettings["sidebarMode"],
                })
              }
              value={uiSettings.sidebarMode}
            >
              <option value="comfortable">Comfortable</option>
              <option value="compact">Compact</option>
            </select>
          </label>
        </div>
      </Panel>

      <Panel icon={<FileText size={20} />} title="Capsule Config">
        <dl className="detail-list detail-list--compact">
          <Detail label="Path" value={config?.configPath ?? "Loading"} />
          <Detail label="Exists" value={config?.exists ? "Yes" : "No"} />
        </dl>
        {config?.warnings.map((warning) => (
          <div className="inline-warning" key={warning}>
            <TriangleAlert size={15} />
            {warning}
          </div>
        ))}
        <div className="config-list">
          {(config?.values ?? []).slice(0, 12).map((item) => (
            <div className="data-row" key={item.key}>
              <div>
                <h4>{item.key}</h4>
                <p>{item.value}</p>
              </div>
              <button
                className="icon-button icon-button--small"
                disabled={dataToolMutating}
                onClick={() =>
                  onRunMutation(async () => {
                    const response = await deleteCapsuleConfigValue(item.key);
                    return `Deleted config value. Backup: ${response.backupPath ?? "new config"}`;
                  })
                }
                title="Delete config value"
                type="button"
              >
                <Trash2 size={15} />
              </button>
            </div>
          ))}
        </div>
        <div className="settings-form-grid">
          <label className="field">
            <span>Key</span>
            <input
              onChange={(event) => setConfigDraft({ ...configDraft, key: event.target.value })}
              value={configDraft.key}
            />
          </label>
          <label className="field">
            <span>Value</span>
            <input
              onChange={(event) => setConfigDraft({ ...configDraft, value: event.target.value })}
              value={configDraft.value}
            />
          </label>
          <button
            className="primary-button"
            disabled={dataToolMutating || !configDraft.key.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await setCapsuleConfigValue(configDraft.key, configDraft.value);
                setConfigDraft({ key: "", value: "" });
                return `Saved config value. Backup: ${response.backupPath ?? "new config"}`;
              })
            }
            type="button"
          >
            <Save size={17} />
            Save
          </button>
        </div>
      </Panel>

      <Panel icon={<Tags size={20} />} title="Tags">
        <div className="catalog-cloud">
          {(tagCatalog?.tags ?? []).slice(0, 28).map((tag) => (
            <span className="tag-chip" key={tag.id}>
              {tag.name} ({tag.entryCount})
            </span>
          ))}
        </div>
        <div className="settings-form-grid">
          <label className="field">
            <span>Rename from</span>
            <input onChange={(event) => setTagDraft({ ...tagDraft, from: event.target.value })} value={tagDraft.from} />
          </label>
          <label className="field">
            <span>Rename to</span>
            <input onChange={(event) => setTagDraft({ ...tagDraft, to: event.target.value })} value={tagDraft.to} />
          </label>
          <button
            className="secondary-button"
            disabled={dataToolMutating || !tagDraft.from.trim() || !tagDraft.to.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await renameTag({ from: tagDraft.from, to: tagDraft.to });
                setTagDraft({ ...tagDraft, from: "", to: "" });
                return `Renamed tag with backup: ${response.audit.backupPath}`;
              })
            }
            type="button"
          >
            Rename
          </button>
        </div>
        <div className="settings-form-grid">
          <label className="field">
            <span>Merge source</span>
            <input onChange={(event) => setTagDraft({ ...tagDraft, source: event.target.value })} value={tagDraft.source} />
          </label>
          <label className="field">
            <span>Merge target</span>
            <input onChange={(event) => setTagDraft({ ...tagDraft, target: event.target.value })} value={tagDraft.target} />
          </label>
          <button
            className="secondary-button"
            disabled={dataToolMutating || !tagDraft.source.trim() || !tagDraft.target.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await mergeTag({ source: tagDraft.source, target: tagDraft.target });
                setTagDraft({ ...tagDraft, source: "", target: "" });
                return `Merged tag with backup: ${response.audit.backupPath}`;
              })
            }
            type="button"
          >
            Merge
          </button>
        </div>
        <div className="settings-form-grid settings-form-grid--delete">
          <label className="field">
            <span>Delete tag</span>
            <input onChange={(event) => setTagDraft({ ...tagDraft, deleteName: event.target.value })} value={tagDraft.deleteName} />
          </label>
          <button
            className="secondary-button"
            disabled={dataToolMutating || !tagDraft.deleteName.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await deleteTag({ name: tagDraft.deleteName });
                setTagDraft({ ...tagDraft, deleteName: "" });
                return `Deleted tag with backup: ${response.audit.backupPath}`;
              })
            }
            type="button"
          >
            <Trash2 size={17} />
            Delete
          </button>
        </div>
      </Panel>

      <Panel icon={<Sparkles size={20} />} title="Moods">
        <div className="catalog-cloud">
          {(moodCatalog?.moods ?? []).slice(0, 28).map((mood) => (
            <span className="mood-chip" key={mood.name}>
              {mood.label} ({mood.entryCount})
            </span>
          ))}
        </div>
        <div className="settings-form-grid">
          <label className="field">
            <span>Rename from</span>
            <input onChange={(event) => setMoodDraft({ ...moodDraft, from: event.target.value })} value={moodDraft.from} />
          </label>
          <label className="field">
            <span>Rename to</span>
            <input onChange={(event) => setMoodDraft({ ...moodDraft, to: event.target.value })} value={moodDraft.to} />
          </label>
          <button
            className="secondary-button"
            disabled={dataToolMutating || !moodDraft.from.trim() || !moodDraft.to.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await renameMood({ from: moodDraft.from, to: moodDraft.to });
                setMoodDraft({ ...moodDraft, from: "", to: "" });
                return `Renamed mood with backup: ${response.audit.backupPath}`;
              })
            }
            type="button"
          >
            Rename
          </button>
        </div>
        <div className="settings-form-grid settings-form-grid--delete">
          <label className="field">
            <span>Clear mood</span>
            <input onChange={(event) => setMoodDraft({ ...moodDraft, deleteName: event.target.value })} value={moodDraft.deleteName} />
          </label>
          <button
            className="secondary-button"
            disabled={dataToolMutating || !moodDraft.deleteName.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await deleteMood({ name: moodDraft.deleteName });
                setMoodDraft({ ...moodDraft, deleteName: "" });
                return `Cleared mood with backup: ${response.audit.backupPath}`;
              })
            }
            type="button"
          >
            <Trash2 size={17} />
            Clear
          </button>
        </div>
      </Panel>

      <Panel icon={<BookOpen size={20} />} title="Template Library">
        <div className="library-list">
          {(library?.templates ?? []).slice(0, 10).map((template) => (
            <article className="data-row data-row--stacked" key={template.slug}>
              <div>
                <h4>{template.name}</h4>
                <p>{template.slug} / {template.isBuiltin ? "built-in" : "custom"} / {template.isActive ? "active" : "inactive"}</p>
              </div>
              <div className="backup-actions">
                <button
                  className="secondary-button secondary-button--small"
                  disabled={dataToolMutating}
                  onClick={() =>
                    onRunMutation(async () => {
                      const response = await updateTemplate(template.slug, {
                        isActive: !template.isActive,
                      });
                      return `Updated template with backup: ${response.audit.backupPath}`;
                    })
                  }
                  type="button"
                >
                  {template.isActive ? "Disable" : "Enable"}
                </button>
                {!template.isBuiltin && (
                  <button
                    className="icon-button icon-button--small"
                    disabled={dataToolMutating}
                    onClick={() =>
                      onRunMutation(async () => {
                        const response = await deleteTemplate(template.slug);
                        return `Deleted template with backup: ${response.audit.backupPath}`;
                      })
                    }
                    title="Delete template"
                    type="button"
                  >
                    <Trash2 size={15} />
                  </button>
                )}
              </div>
            </article>
          ))}
        </div>
        <div className="settings-form-grid">
          <label className="field">
            <span>Slug</span>
            <input onChange={(event) => setTemplateDraft({ ...templateDraft, slug: event.target.value })} value={templateDraft.slug} />
          </label>
          <label className="field">
            <span>Name</span>
            <input onChange={(event) => setTemplateDraft({ ...templateDraft, name: event.target.value })} value={templateDraft.name} />
          </label>
          <label className="field">
            <span>Description</span>
            <input onChange={(event) => setTemplateDraft({ ...templateDraft, description: event.target.value })} value={templateDraft.description} />
          </label>
          <label className="field">
            <span>Intro</span>
            <input onChange={(event) => setTemplateDraft({ ...templateDraft, introText: event.target.value })} value={templateDraft.introText} />
          </label>
          <label className="field field--wide">
            <span>Sections</span>
            <input onChange={(event) => setTemplateDraft({ ...templateDraft, sections: event.target.value })} placeholder="## Wins, ## Next" value={templateDraft.sections} />
          </label>
          <button
            className="primary-button"
            disabled={dataToolMutating || !templateDraft.slug.trim() || !templateDraft.name.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await createTemplate({
                  slug: templateDraft.slug,
                  name: templateDraft.name,
                  description: nullableFromText(templateDraft.description),
                  introText: templateDraft.introText,
                  sections: splitFilter(templateDraft.sections),
                  isActive: true,
                });
                setTemplateDraft({ slug: "", name: "", description: "", introText: "", sections: "" });
                return `Created template with backup: ${response.audit.backupPath}`;
              })
            }
            type="button"
          >
            <Plus size={17} />
            Add
          </button>
        </div>
      </Panel>

      <Panel icon={<FileText size={20} />} title="Prompt Library">
        <div className="library-list">
          {(library?.prompts ?? []).slice(0, 10).map((prompt) => (
            <article className="data-row data-row--stacked" key={prompt.slug}>
              <div>
                <h4>{prompt.slug}</h4>
                <p>{prompt.promptText}</p>
                <div className="entry-meta">
                  <span className="tag-chip">{prompt.category}</span>
                  <span className="tag-chip">{prompt.isBuiltin ? "built-in" : "custom"}</span>
                  <span className="tag-chip">{prompt.isActive ? "active" : "inactive"}</span>
                </div>
              </div>
              <div className="backup-actions">
                <button
                  className="secondary-button secondary-button--small"
                  disabled={dataToolMutating}
                  onClick={() =>
                    onRunMutation(async () => {
                      const response = await updatePrompt(prompt.slug, {
                        isActive: !prompt.isActive,
                      });
                      return `Updated prompt with backup: ${response.audit.backupPath}`;
                    })
                  }
                  type="button"
                >
                  {prompt.isActive ? "Disable" : "Enable"}
                </button>
                {!prompt.isBuiltin && (
                  <button
                    className="icon-button icon-button--small"
                    disabled={dataToolMutating}
                    onClick={() =>
                      onRunMutation(async () => {
                        const response = await deletePrompt(prompt.slug);
                        return `Deleted prompt with backup: ${response.audit.backupPath}`;
                      })
                    }
                    title="Delete prompt"
                    type="button"
                  >
                    <Trash2 size={15} />
                  </button>
                )}
              </div>
            </article>
          ))}
        </div>
        <div className="settings-form-grid">
          <label className="field">
            <span>Slug</span>
            <input onChange={(event) => setPromptDraft({ ...promptDraft, slug: event.target.value })} value={promptDraft.slug} />
          </label>
          <label className="field">
            <span>Category</span>
            <input onChange={(event) => setPromptDraft({ ...promptDraft, category: event.target.value })} value={promptDraft.category} />
          </label>
          <label className="field">
            <span>Tags</span>
            <input onChange={(event) => setPromptDraft({ ...promptDraft, tags: event.target.value })} value={promptDraft.tags} />
          </label>
          <label className="field field--wide">
            <span>Prompt</span>
            <textarea
              className="compact-textarea"
              onChange={(event) => setPromptDraft({ ...promptDraft, promptText: event.target.value })}
              value={promptDraft.promptText}
            />
          </label>
          <button
            className="primary-button"
            disabled={dataToolMutating || !promptDraft.slug.trim() || !promptDraft.promptText.trim()}
            onClick={() =>
              onRunMutation(async () => {
                const response = await createPrompt({
                  slug: promptDraft.slug,
                  promptText: promptDraft.promptText,
                  category: promptDraft.category,
                  tags: splitFilter(promptDraft.tags),
                  isActive: true,
                });
                setPromptDraft({ slug: "", promptText: "", category: "general", tags: "" });
                return `Created prompt with backup: ${response.audit.backupPath}`;
              })
            }
            type="button"
          >
            <Plus size={17} />
            Add
          </button>
        </div>
      </Panel>
    </section>
  );
}

type AiViewProps = {
  status: DatabaseStatus | null;
  overview: AiOverviewResponse | null;
  suggestion: AiMetadataSuggestionResponse | null;
  selectedIdentifier: string;
  setSelectedIdentifier: (value: string) => void;
  loading: boolean;
  suggesting: boolean;
  onRefresh: () => void;
  onSuggest: () => void;
};

function AiView({
  status,
  overview,
  suggestion,
  selectedIdentifier,
  setSelectedIdentifier,
  loading,
  suggesting,
  onRefresh,
  onSuggest,
}: AiViewProps) {
  if (!status?.readable) {
    return <UnavailableState icon={<Bot size={24} />} label="AI needs a readable database." status={status} />;
  }

  return (
    <section className="phase6-workspace">
      <div className="metric-strip">
        <Metric label="Conversations" value={overview?.conversationCount ?? 0} />
        <Metric label="Messages" value={overview?.messageCount ?? 0} />
        <Metric label="Embedded Entries" value={overview?.embeddedEntryCount ?? 0} />
        <Metric label="Time Capsules" value={overview?.timeCapsuleCount ?? 0} />
      </div>

      <div className="phase6-grid">
        <Panel
          icon={<Bot size={20} />}
          title="AI Capability Layer"
          action={
            <button className="secondary-button secondary-button--small" onClick={onRefresh} type="button">
              <RefreshCw size={15} />
              {loading ? "Loading" : "Refresh"}
            </button>
          }
        >
          <dl className="detail-list">
            <Detail label="Provider" value={overview?.provider ?? "Not configured"} />
            <Detail label="Model" value={overview?.model ?? "Not configured"} />
          </dl>
          <CapabilityGrid capabilities={overview?.capabilities ?? []} />
          <WarningList warnings={overview?.warnings ?? []} />
        </Panel>

        <Panel icon={<Sparkles size={20} />} title="Metadata Suggestions">
          <div className="inline-form">
            <label className="field">
              <span>Entry UUID or ID</span>
              <input
                onChange={(event) => setSelectedIdentifier(event.target.value)}
                placeholder="entry_..."
                value={selectedIdentifier}
              />
            </label>
            <button
              className="primary-button"
              disabled={suggesting || !selectedIdentifier.trim()}
              onClick={onSuggest}
              type="button"
            >
              <Sparkles size={17} />
              {suggesting ? "Suggesting" : "Suggest"}
            </button>
          </div>
          {suggestion ? (
            <div className="suggestion-card">
              <div className="metadata-heading-row">
                <h4>{suggestion.suggestedTitle ?? "Untitled suggestion"}</h4>
                <span className="status-pill status-pill--neutral">
                  {Math.round(suggestion.confidence * 100)}%
                </span>
              </div>
              <p>{suggestion.suggestedSummary ?? "No summary suggestion."}</p>
              <div className="tag-row">
                {suggestion.suggestedMood && <span className="mood-chip">{suggestion.suggestedMood}</span>}
                {suggestion.suggestedTags.map((tag) => (
                  <span className="tag-chip" key={tag}>
                    {tag}
                  </span>
                ))}
              </div>
              <WarningList warnings={suggestion.warnings} />
            </div>
          ) : (
            <p className="muted">Suggestions run locally unless an explicit provider bridge is later configured.</p>
          )}
        </Panel>
      </div>

      <div className="phase6-grid phase6-grid--three">
        <Panel icon={<FileText size={20} />} title="Recent AI Chats">
          <DataList
            emptyText="No persisted AI conversations found."
            items={(overview?.conversations ?? []).map((conversation) => ({
              key: String(conversation.id),
              title: conversation.title,
              meta: `${conversation.cloudProvider} / ${conversation.scope} / ${conversation.messageCount} messages`,
              body: conversation.preview || formatDateTime(conversation.lastMessageAt),
            }))}
          />
        </Panel>

        <Panel icon={<Clock3 size={20} />} title="Time Capsules">
          <DataList
            emptyText="No AI Time Capsules found."
            items={(overview?.timeCapsules ?? []).map((capsule) => ({
              key: String(capsule.id),
              title: capsule.triggerLabel,
              meta: `${capsule.status} / ${capsule.sourceEntryCount} entries`,
              body: `${formatDateTime(capsule.dueDate)} / ${capsule.cloudProvider || "provider unknown"}`,
            }))}
          />
        </Panel>

        <Panel icon={<Database size={20} />} title="Embeddings">
          <DataList
            emptyText="No embedding models found."
            items={(overview?.embeddingModels ?? []).map((model) => ({
              key: String(model.id),
              title: model.name,
              meta: `${model.provider} / ${model.dimensions} dimensions`,
              body: `${model.entryCount} embedded entries${model.isActive ? " / active" : ""}`,
            }))}
          />
        </Panel>
      </div>
    </section>
  );
}

type SyncViewProps = {
  status: DatabaseStatus | null;
  overview: SyncOverviewResponse | null;
  loading: boolean;
  onRefresh: () => void;
};

function SyncView({ status, overview, loading, onRefresh }: SyncViewProps) {
  if (!status?.readable) {
    return <UnavailableState icon={<Cloud size={24} />} label="Sync needs a readable database." status={status} />;
  }

  const syncStatus = overview?.status;
  return (
    <section className="phase6-workspace">
      <div className="metric-strip">
        <Metric label="Imported" value={syncStatus?.lastSyncImported ?? 0} />
        <Metric label="Updated" value={syncStatus?.lastSyncUpdated ?? 0} />
        <Metric label="Deleted" value={syncStatus?.lastSyncDeleted ?? 0} />
        <Metric label="Conflicts" value={syncStatus?.lastConflictCount ?? 0} />
      </div>

      <div className="phase6-grid">
        <Panel
          icon={<Cloud size={20} />}
          title="Sync Status"
          action={
            <button className="secondary-button secondary-button--small" onClick={onRefresh} type="button">
              <RefreshCw size={15} />
              {loading ? "Loading" : "Refresh"}
            </button>
          }
        >
          <dl className="detail-list">
            <Detail label="Last success" value={formatDateTime(syncStatus?.lastSuccessfulSyncAt)} />
            <Detail label="File" value={syncStatus?.lastSyncFilePath ?? "None"} />
            <Detail label="Size" value={syncStatus?.lastSyncFileSizeBytes ? formatBytes(syncStatus.lastSyncFileSizeBytes) : "None"} />
            <Detail label="Summary" value={syncStatus?.lastSyncSummary ?? syncStatus?.lastSyncError ?? "No sync status row."} />
          </dl>
          <CapabilityGrid capabilities={overview?.capabilities ?? []} />
          <WarningList warnings={overview?.warnings ?? []} />
        </Panel>

        <Panel icon={<History size={20} />} title="Recent Sync History">
          <DataList
            emptyText="No sync history rows found."
            items={(overview?.recentHistory ?? []).map((item) => ({
              key: String(item.id),
              title: `${item.status} / ${formatDateTime(item.timestamp)}`,
              meta: `${item.importedCount} imported / ${item.updatedCount} updated / ${item.conflictCount} conflicts`,
              body: item.summary ?? item.error ?? item.syncFilePath ?? "No details",
            }))}
          />
        </Panel>
      </div>

      <Panel icon={<Trash2 size={20} />} title="Sync Tombstones">
        <div className="catalog-cloud">
          {(overview?.tombstones ?? []).map((item) => (
            <span className="tag-chip" key={item.table}>
              {item.table}: {item.count}
            </span>
          ))}
          {(overview?.tombstones ?? []).length === 0 && <span className="muted">No tombstone tables found.</span>}
        </div>
      </Panel>
    </section>
  );
}

type PluginsViewProps = {
  status: DatabaseStatus | null;
  overview: PluginOverviewResponse | null;
  loading: boolean;
  mutating: boolean;
  onRefresh: () => void;
  onToggle: (pluginName: string, enabled: boolean) => void;
};

function PluginsView({ status, overview, loading, mutating, onRefresh, onToggle }: PluginsViewProps) {
  if (!status?.readable) {
    return <UnavailableState icon={<Plug size={24} />} label="Plugins need a readable database." status={status} />;
  }

  return (
    <section className="phase6-workspace">
      <Panel
        icon={<Plug size={20} />}
        title="Plugin Registry"
        action={
          <button className="secondary-button secondary-button--small" onClick={onRefresh} type="button">
            <RefreshCw size={15} />
            {loading ? "Loading" : "Refresh"}
          </button>
        }
      >
        <CapabilityGrid capabilities={overview?.capabilities ?? []} />
        <WarningList warnings={overview?.warnings ?? []} />
      </Panel>

      <div className="plugin-grid">
        {(overview?.plugins ?? []).map((plugin) => (
          <article className="data-row data-row--stacked plugin-card" key={plugin.key}>
            <div className="metadata-heading-row">
              <div>
                <h4>{plugin.label}</h4>
                <p>{plugin.tableName ?? "No module table"} / {plugin.source}</p>
              </div>
              <StatusPill tone={plugin.enabled ? "good" : "neutral"}>
                {plugin.enabled ? "Enabled" : "Disabled"}
              </StatusPill>
            </div>
            <dl className="detail-list detail-list--compact">
              <Detail label="Rows" value={plugin.rowCount} />
              <Detail label="Version" value={plugin.installedVersion ?? "None"} />
              <Detail label="Updated" value={formatDateTime(plugin.updatedAt)} />
              <Detail label="Screen" value={plugin.implemented ? "Implemented" : "State only"} />
            </dl>
            <button
              className={plugin.enabled ? "secondary-button" : "primary-button"}
              disabled={mutating}
              onClick={() => onToggle(plugin.key, !plugin.enabled)}
              type="button"
            >
              {plugin.enabled ? "Disable" : "Enable"}
            </button>
          </article>
        ))}
      </div>
    </section>
  );
}

type GamificationViewProps = {
  status: DatabaseStatus | null;
  overview: GamificationOverviewResponse | null;
  loading: boolean;
  mutating: boolean;
  onRefresh: () => void;
  onClaimQuest: (quest: GamificationQuest) => void;
};

function GamificationView({
  status,
  overview,
  loading,
  mutating,
  onRefresh,
  onClaimQuest,
}: GamificationViewProps) {
  if (!status?.readable) {
    return <UnavailableState icon={<Trophy size={24} />} label="Profile needs a readable database." status={status} />;
  }

  return (
    <section className="phase6-workspace">
      <div className="metric-strip">
        <Metric label="Total XP" value={overview?.totalXp ?? 0} />
        <Metric label="Level" value={overview?.level ?? 1} />
        <Metric label="To Next" value={overview?.xpToNextLevel ?? 0} />
        <Metric label="XP Events" value={overview?.eventCount ?? 0} />
      </div>

      <div className="phase6-grid">
        <Panel
          icon={<Trophy size={20} />}
          title="Profile And Capabilities"
          action={
            <button className="secondary-button secondary-button--small" onClick={onRefresh} type="button">
              <RefreshCw size={15} />
              {loading ? "Loading" : "Refresh"}
            </button>
          }
        >
          <dl className="detail-list">
            <Detail label="Hero" value={overview?.profile?.heroSpritePath ?? "Default"} />
            <Detail label="Updated" value={formatDateTime(overview?.profile?.updatedAt)} />
          </dl>
          <CapabilityGrid capabilities={overview?.capabilities ?? []} />
          <WarningList warnings={overview?.warnings ?? []} />
        </Panel>

        <Panel icon={<Star size={20} />} title="Quests">
          <div className="quest-list">
            {(overview?.quests ?? []).map((quest) => {
              const complete = quest.progressValue >= quest.targetValue;
              const claimed = Boolean(quest.claimedAt) || quest.status === "claimed";
              return (
                <article className="quest-card" key={quest.instanceId}>
                  <div className="metadata-heading-row">
                    <div>
                      <h4>{quest.title}</h4>
                      <p>{quest.description}</p>
                    </div>
                    <span className="status-pill status-pill--neutral">{quest.rewardXp} XP</span>
                  </div>
                  <div className="progress-track" title={`${quest.progressValue} / ${quest.targetValue}`}>
                    <div style={{ width: `${Math.min(100, (quest.progressValue / Math.max(quest.targetValue, 1)) * 100)}%` }} />
                  </div>
                  <div className="quest-footer">
                    <span>{quest.progressValue} / {quest.targetValue}</span>
                    <button
                      className="secondary-button secondary-button--small"
                      disabled={mutating || !complete || claimed}
                      onClick={() => onClaimQuest(quest)}
                      type="button"
                    >
                      {claimed ? "Claimed" : "Claim"}
                    </button>
                  </div>
                </article>
              );
            })}
            {(overview?.quests ?? []).length === 0 && <p className="muted">No quest rows found.</p>}
          </div>
        </Panel>
      </div>

      <div className="phase6-grid">
        <Panel icon={<History size={20} />} title="Recent XP">
          <DataList
            emptyText="No XP events found."
            items={(overview?.recentEvents ?? []).map((event) => ({
              key: String(event.id),
              title: `${event.amount} XP / ${event.reason}`,
              meta: `${event.sourceType}:${event.sourceKey}`,
              body: formatDateTime(event.createdAt),
            }))}
          />
        </Panel>

        <Panel icon={<ShieldCheck size={20} />} title="Badges">
          <DataList
            emptyText="No badges unlocked yet."
            items={(overview?.badges ?? []).map((badge) => ({
              key: badge.badgeKey,
              title: badge.badgeKey.replace(/_/g, " "),
              meta: formatDateTime(badge.unlockedAt),
              body: `Updated ${formatDateTime(badge.updatedAt)}`,
            }))}
          />
        </Panel>
      </div>
    </section>
  );
}

function CapabilityGrid({ capabilities }: { capabilities: Phase6Capability[] }) {
  if (capabilities.length === 0) {
    return <p className="muted">No capability rows available.</p>;
  }

  return (
    <div className="capability-grid">
      {capabilities.map((capability) => (
        <article className="capability-card" key={capability.key}>
          <div className="metadata-heading-row">
            <h4>{capability.label}</h4>
            <StatusPill tone={capability.available && capability.configured ? "good" : "neutral"}>
              {capability.available ? (capability.configured ? "Ready" : "Gated") : "Missing"}
            </StatusPill>
          </div>
          <p>{capability.detail}</p>
          <div className="tag-row">
            {capability.requiresCloud && <span className="tag-chip">cloud</span>}
            {capability.readOnly && <span className="tag-chip">read-only</span>}
          </div>
        </article>
      ))}
    </div>
  );
}

type DataListItem = {
  key: string;
  title: string;
  meta: string;
  body: string;
};

function DataList({ items, emptyText }: { items: DataListItem[]; emptyText: string }) {
  if (items.length === 0) {
    return <p className="muted">{emptyText}</p>;
  }

  return (
    <div className="data-list">
      {items.map((item) => (
        <article className="data-row data-row--stacked" key={item.key}>
          <h4>{item.title}</h4>
          <p>{item.meta}</p>
          <p>{item.body}</p>
        </article>
      ))}
    </div>
  );
}

function WarningList({ warnings }: { warnings: string[] }) {
  if (warnings.length === 0) {
    return null;
  }

  return (
    <ul className="warning-list phase6-warning-list">
      {warnings.map((warning) => (
        <li key={warning}>{warning}</li>
      ))}
    </ul>
  );
}

function UnavailableState({
  icon,
  label,
  status,
}: {
  icon: ReactNode;
  label: string;
  status: DatabaseStatus | null;
}) {
  return (
    <section className="state-panel">
      {icon}
      <h3>{label}</h3>
      <p>{status?.dbPath ?? "No database path resolved."}</p>
    </section>
  );
}

function AboutView() {
  return (
    <section className="about-panel">
      <h3>Capsule Tauri</h3>
      <p>
        Phase 6 adds capability-gated AI, sync, plugin, and gamification
        surfaces over the local Capsule database. Cloud and bridge-driven paths
        stay explicit, and plugin/quest writes keep the backup guard.
      </p>
      <p>Hard delete remains reserved until the legacy resequencing behavior is fully matched and tested.</p>
    </section>
  );
}

type EntryDetailProps = {
  entry: Entry | null;
  entryHistory: EntryHistoryResponse | null;
  historyLoading: boolean;
  loading: boolean;
  mutating: boolean;
  onEdit: (entry: Entry) => void;
  onContinue: (entry: Entry) => void;
  onEntryAction: (entry: Entry, action: "star" | "pin" | "hide" | "unhide") => void;
  onExport: (entry: Entry, format: ExportFormat) => void;
  onLoadHistory: (entry: Entry) => void;
};

function EntryDetail({
  entry,
  entryHistory,
  historyLoading,
  loading,
  mutating,
  onEdit,
  onContinue,
  onEntryAction,
  onExport,
  onLoadHistory,
}: EntryDetailProps) {
  if (loading) {
    return (
      <aside className="detail-panel">
        <div className="skeleton skeleton-title" />
        <div className="skeleton skeleton-line" />
        <div className="skeleton skeleton-block" />
      </aside>
    );
  }

  if (!entry) {
    return (
      <aside className="detail-panel detail-panel--empty">
        <Search size={22} />
        <h3>No entry selected</h3>
        <p>Select an entry to inspect it.</p>
      </aside>
    );
  }

  const weatherTemperature = entry.location ? formatWeatherTemperature(entry.location) : null;

  return (
    <aside className="detail-panel">
      <div className="entry-detail-heading">
        <p className="eyebrow">{formatDateTime(entry.createdAt)}</p>
        <h3>{entry.title || entry.textPlain.slice(0, 72) || "Untitled entry"}</h3>
      </div>

      <div className="entry-action-bar">
        <button
          className={entry.starred ? "icon-button icon-button--active" : "icon-button"}
          disabled={mutating}
          onClick={() => onEntryAction(entry, "star")}
          title={entry.starred ? "Unstar" : "Star"}
          type="button"
        >
          <Star size={17} />
        </button>
        <button
          className={entry.pinned ? "icon-button icon-button--active" : "icon-button"}
          disabled={mutating}
          onClick={() => onEntryAction(entry, "pin")}
          title={entry.pinned ? "Unpin" : "Pin"}
          type="button"
        >
          {entry.pinned ? <PinOff size={17} /> : <Pin size={17} />}
        </button>
        <button
          className="icon-button"
          disabled={mutating}
          onClick={() => onEntryAction(entry, entry.hidden ? "unhide" : "hide")}
          title={entry.hidden ? "Unhide" : "Hide"}
          type="button"
        >
          {entry.hidden ? <Eye size={17} /> : <EyeOff size={17} />}
        </button>
        <button className="secondary-button" onClick={() => onEdit(entry)} type="button">
          <Edit3 size={17} />
          Edit
        </button>
        <button className="secondary-button" onClick={() => onContinue(entry)} type="button">
          <FileText size={17} />
          Continue
        </button>
        <button className="secondary-button" onClick={() => onExport(entry, "markdown")} type="button">
          <Download size={17} />
          MD
        </button>
        <button className="secondary-button" onClick={() => onExport(entry, "json")} type="button">
          JSON
        </button>
      </div>

      <div className="tag-row">
        {entry.moodInfo.label && <span className="mood-chip">{entry.moodInfo.label}</span>}
        {entry.tags.map((tag) => (
          <span className="tag-chip" key={tag.id}>
            {tag.name}
          </span>
        ))}
      </div>

      <article className="entry-body">{entry.textPlain || entry.text}</article>

      <dl className="detail-list detail-list--compact">
        <Detail label="UUID" value={entry.uuid} />
        <Detail label="Format" value={entry.contentFormat} />
        <Detail label="Updated" value={formatDateTime(entry.updatedAt)} />
        <Detail label="Images" value={entry.attachmentCount} />
      </dl>

      {entry.summary && (
        <div className="metadata-block">
          <h4>Summary</h4>
          <p>{entry.summary}</p>
        </div>
      )}

      {entry.location && (
        <div className="metadata-block">
          <h4>
            <MapPin size={16} />
            Location
          </h4>
          <p>{entry.location.placeName ?? `${entry.location.latitude}, ${entry.location.longitude}`}</p>
          {(entry.location.weatherCondition || weatherTemperature) && (
            <p>
              {entry.location.weatherCondition ?? "Weather"} /{" "}
              {weatherTemperature ?? "No temperature"}
            </p>
          )}
          {(entry.location.weatherHumidity !== null || entry.location.weatherWindKph !== null) && (
            <p>
              {entry.location.weatherHumidity !== null ? `Humidity ${entry.location.weatherHumidity}%` : "Humidity n/a"}
              {" / "}
              {entry.location.weatherWindKph !== null
                ? `Wind ${entry.location.weatherWindKph.toFixed(1)} kph`
                : "Wind n/a"}
            </p>
          )}
          {(entry.location.weatherIcon || entry.location.weatherFetchedAt || entry.location.source) && (
            <p>
              {[entry.location.weatherIcon, entry.location.weatherFetchedAt, entry.location.source]
                .filter(Boolean)
                .join(" / ")}
            </p>
          )}
        </div>
      )}

      {entry.thread && (
        <div className="metadata-block">
          <h4>Thread</h4>
          <p>{entry.thread.title ?? entry.thread.rootUuid}</p>
          {entry.thread.summary && <p>{entry.thread.summary}</p>}
          <p>
            {entry.thread.entryCount} entries / {entry.thread.isRoot ? "Root" : "Continuation"}
          </p>
        </div>
      )}

      <div className="metadata-block">
        <div className="metadata-heading-row">
          <h4>
            <History size={16} />
            History
          </h4>
          <button
            className="secondary-button secondary-button--small"
            disabled={historyLoading}
            onClick={() => onLoadHistory(entry)}
            type="button"
          >
            {historyLoading ? "Loading" : "Load"}
          </button>
        </div>
        {entryHistory?.entryId === entry.id ? (
          entryHistory.history.length > 0 ? (
            <div className="history-list">
              {entryHistory.history.map((item) => (
                <article className="history-row" key={item.id}>
                  <div>
                    <h5>{item.operationType.replace("EDIT_", "").toLowerCase()}</h5>
                    <p>{formatDateTime(item.timestamp)}</p>
                  </div>
                  <span>{item.changedFields.join(", ") || "metadata"}</span>
                </article>
              ))}
            </div>
          ) : (
            <p>No edit history for this entry.</p>
          )
        ) : (
          <p>Version snapshots appear here after loading.</p>
        )}
      </div>
    </aside>
  );
}

type EntryStackProps = {
  entries: Entry[];
  loading: boolean;
  emptyText?: string;
};

function EntryStack({ entries, loading, emptyText = "No entries found." }: EntryStackProps) {
  if (loading) {
    return <SkeletonList compact />;
  }

  if (entries.length === 0) {
    return <div className="empty-state">{emptyText}</div>;
  }

  return (
    <div className="entry-stack">
      {entries.map((entry) => (
        <EntryMini entry={entry} key={entry.uuid} />
      ))}
    </div>
  );
}

function EntryMini({ entry }: { entry: Entry }) {
  return (
    <article className="entry-mini">
      <div>
        <h4>{entry.title || entry.textPlain.slice(0, 82) || "Untitled entry"}</h4>
        <p>{entry.textPlain.slice(0, 140)}</p>
      </div>
      <EntryMeta entry={entry} />
    </article>
  );
}

function EntryCardContent({ entry }: { entry: Entry }) {
  return (
    <>
      <div className="entry-card-heading">
        <div>
          <p className="eyebrow">{formatDateTime(entry.createdAt)}</p>
          <h4>{entry.title || entry.textPlain.slice(0, 84) || "Untitled entry"}</h4>
        </div>
        {entry.attachmentCount > 0 && (
          <span className="icon-stat" title="Image attachments">
            <ImageIcon size={15} />
            {entry.attachmentCount}
          </span>
        )}
      </div>
      <p>{entry.summary || entry.textPlain.slice(0, 180)}</p>
      <EntryMeta entry={entry} />
    </>
  );
}

function EntryMeta({ entry }: { entry: Entry }) {
  const weatherTemperature = entry.location ? formatWeatherTemperature(entry.location) : null;

  return (
    <div className="entry-meta">
      {entry.moodInfo.label && <span className="mood-chip">{entry.moodInfo.label}</span>}
      {entry.tags.slice(0, 4).map((tag) => (
        <span className="tag-chip" key={tag.id}>
          <Tags size={12} />
          {tag.name}
        </span>
      ))}
      {entry.location && (
        <span className="tag-chip">
          <MapPin size={12} />
          {entry.location.placeName ?? "Location"}
        </span>
      )}
      {entry.location?.weatherCondition && (
        <span className="tag-chip">
          {weatherTemperature
            ? `${entry.location.weatherCondition}, ${weatherTemperature}`
            : entry.location.weatherCondition}
        </span>
      )}
      {entry.thread && <span className="tag-chip">{entry.thread.entryCount} in thread</span>}
    </div>
  );
}

type DataUrlImageProps = {
  attachment: ImageAttachment;
  variant: ImageVariant;
  className: string;
};

function DataUrlImage({ attachment, variant, className }: DataUrlImageProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setSrc(null);
    setFailed(false);
    getImageDataUrl(attachment.attachmentId, variant)
      .then((dataUrl) => {
        if (!cancelled) {
          setSrc(dataUrl);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFailed(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [attachment.attachmentId, variant]);

  if (failed) {
    return (
      <div className={`${className} media-placeholder`}>
        <FileImage size={24} />
      </div>
    );
  }

  if (!src) {
    return <div className={`${className} media-placeholder skeleton`} />;
  }

  return <img alt={attachment.altText ?? attachment.caption ?? "Entry image"} className={className} src={src} />;
}

type CoverImageProps = {
  filename: string;
  variant: ImageVariant;
  className: string;
};

function CoverImage({ filename, variant, className }: CoverImageProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setSrc(null);
    setFailed(false);
    getCoverDataUrl(filename, variant)
      .then((dataUrl) => {
        if (!cancelled) {
          setSrc(dataUrl);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFailed(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [filename, variant]);

  if (failed) {
    return (
      <div className={`${className} media-placeholder`}>
        <Images size={24} />
      </div>
    );
  }

  if (!src) {
    return <div className={`${className} media-placeholder skeleton`} />;
  }

  return <img alt={filename} className={className} src={src} />;
}

function ImageLightbox({
  attachment,
  onClose,
}: {
  attachment: ImageAttachment;
  onClose: () => void;
}) {
  return (
    <div className="lightbox" role="dialog" aria-modal="true">
      <div className="lightbox-toolbar">
        <div>
          <p className="eyebrow">{attachment.mimeType}</p>
          <h3>{attachment.caption || attachment.altText || attachment.hash}</h3>
        </div>
        <button className="icon-button" onClick={onClose} title="Close" type="button">
          <X size={18} />
        </button>
      </div>
      <DataUrlImage attachment={attachment} className="lightbox-image" variant="full" />
    </div>
  );
}

type PanelProps = {
  icon: ReactNode;
  title: string;
  action?: ReactNode;
  children: ReactNode;
};

function Panel({ icon, title, action, children }: PanelProps) {
  return (
    <article className="panel">
      <div className="panel-header">
        <div className="panel-title">
          {icon}
          <h3>{title}</h3>
        </div>
        {action}
      </div>
      {children}
    </article>
  );
}

type DetailProps = {
  label: string;
  value: ReactNode;
};

function Detail({ label, value }: DetailProps) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </>
  );
}

function Metric({ label, value }: DetailProps) {
  return (
    <div className="metric">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function TrendBars({ trend }: { trend: AnalyticsResponse["monthlyTrend"] }) {
  if (trend.length === 0) {
    return <p className="muted">No monthly activity in this period.</p>;
  }

  const maxEntries = Math.max(1, ...trend.map((point) => point.entryCount));
  return (
    <div className="bar-list">
      {trend.map((point) => (
        <div className="bar-row" key={point.period}>
          <span>{point.period}</span>
          <div className="bar-track">
            <div style={{ width: `${(point.entryCount / maxEntries) * 100}%` }} />
          </div>
          <strong>{point.entryCount}</strong>
          <em>{point.wordCount} words</em>
        </div>
      ))}
    </div>
  );
}

function BreakdownList({
  items,
  emptyText = "No data in this period.",
}: {
  items: AnalyticsResponse["tagBreakdown"];
  emptyText?: string;
}) {
  if (items.length === 0) {
    return <p className="muted">{emptyText}</p>;
  }

  const maxCount = Math.max(1, ...items.map((item) => item.count));
  return (
    <div className="bar-list">
      {items.slice(0, 10).map((item) => (
        <div className="bar-row" key={item.label}>
          <span>{item.label}</span>
          <div className="bar-track">
            <div style={{ width: `${(item.count / maxCount) * 100}%` }} />
          </div>
          <strong>{item.count}</strong>
        </div>
      ))}
    </div>
  );
}

type CalendarDayCell = {
  date: string;
  day: number;
  data?: WritingCalendarResponse["days"][number];
};

type CalendarMonth = {
  label: string;
  blanks: number;
  days: CalendarDayCell[];
};

function buildCalendarMonths(
  year: number,
  days: WritingCalendarResponse["days"],
): CalendarMonth[] {
  const daysByDate = new Map(days.map((day) => [day.date, day]));
  return Array.from({ length: 12 }).map((_, monthIndex) => {
    const firstDay = new Date(year, monthIndex, 1).getDay();
    const daysInMonth = new Date(year, monthIndex + 1, 0).getDate();
    const label = new Date(year, monthIndex, 1).toLocaleDateString(undefined, {
      month: "long",
    });
    return {
      label,
      blanks: firstDay,
      days: Array.from({ length: daysInMonth }).map((__, dayIndex) => {
        const day = dayIndex + 1;
        const date = `${year}-${String(monthIndex + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
        return {
          date,
          day,
          data: daysByDate.get(date),
        };
      }),
    };
  });
}

function calendarLevel(day: CalendarDayCell["data"], maxEntryCount: number) {
  if (!day || day.entryCount <= 0) {
    return 0;
  }
  return Math.max(1, Math.ceil((day.entryCount / Math.max(maxEntryCount, 1)) * 4));
}

function calendarDayTitle(date: string, day: CalendarDayCell["data"]) {
  if (!day) {
    return `${date}: no entries`;
  }
  const moodText = day.moods.length ? ` / ${day.moods.join(", ")}` : "";
  return `${date}: ${day.entryCount} entries, ${day.wordCount} words, ${day.imageCount} images${moodText}`;
}

function SkeletonList({ compact = false }: { compact?: boolean }) {
  return (
    <div className={compact ? "skeleton-list skeleton-list--compact" : "skeleton-list"}>
      {Array.from({ length: compact ? 3 : 5 }).map((_, index) => (
        <div className="skeleton-card" key={index}>
          <div className="skeleton skeleton-title" />
          <div className="skeleton skeleton-line" />
          <div className="skeleton skeleton-line skeleton-line--short" />
        </div>
      ))}
    </div>
  );
}

function draftHasContent(draft: ComposerDraft) {
  return Boolean(
    draft.text.trim() ||
      draft.title.trim() ||
      draft.summary.trim() ||
      draft.mood.trim() ||
      draft.tags.trim() ||
      draft.when.trim() ||
      draft.continueFromUuid.trim() ||
      draft.starred ||
      draft.pinned,
  );
}

function draftFromEntry(entry: Entry): ComposerDraft {
  return {
    text: entry.text,
    title: entry.title ?? "",
    summary: entry.summary ?? "",
    mood: entry.mood ?? "",
    tags: entry.tags.map((tag) => tag.name).join(", "),
    when: "",
    starred: entry.starred,
    pinned: entry.pinned,
    continueFromUuid: entry.thread?.parentUuid ?? "",
  };
}

function nullableFromText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function writingStats(text: string) {
  const words = text.trim().split(/\s+/).filter(Boolean).length;
  return {
    words,
    characters: text.length,
    readingMinutes: words === 0 ? 0 : Math.max(1, Math.ceil(words / 220)),
  };
}

function splitFilter(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function isThreadLeaf(thread: ThreadGroup, entry: Entry) {
  return !thread.entries.some((item) => item.thread?.parentUuid === entry.uuid);
}

export default App;
