import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Archive,
  BookOpen,
  CheckCircle2,
  Database,
  FileArchive,
  Filter,
  HardDrive,
  Image as ImageIcon,
  Info,
  MapPin,
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
  Shuffle,
  Tags,
  TriangleAlert,
} from "lucide-react";
import {
  createBackup,
  getDatabaseStatus,
  getEntry,
  getRandomEntry,
  listBackups,
  listEntries,
} from "./backend";
import { StatusPill } from "./components/StatusPill";
import { formatBytes, formatDateTime } from "./lib/format";
import type {
  BackupInfo,
  DatabaseStatus,
  Entry,
  EntryFilters,
  EntryListResponse,
} from "./types";
import "./styles.css";

type ActiveView = "dashboard" | "entries" | "backups" | "settings" | "about";

type EntryFilterForm = {
  text: string;
  tag: string;
  mood: string;
  since: string;
  until: string;
  includeHidden: boolean;
  hasImages: boolean;
  sort: "asc" | "desc";
};

type DashboardCounts = {
  currentYear: number | null;
  currentMonth: number | null;
};

const navItems: Array<{ id: ActiveView; label: string; icon: ReactNode }> = [
  { id: "dashboard", label: "Dashboard", icon: <Database size={18} /> },
  { id: "entries", label: "Entries", icon: <BookOpen size={18} /> },
  { id: "backups", label: "Backups", icon: <Archive size={18} /> },
  { id: "settings", label: "Settings", icon: <Settings size={18} /> },
  { id: "about", label: "About", icon: <Info size={18} /> },
];

const defaultEntryFilters: EntryFilterForm = {
  text: "",
  tag: "",
  mood: "",
  since: "",
  until: "",
  includeHidden: false,
  hasImages: false,
  sort: "desc",
};

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
  const [loading, setLoading] = useState(true);
  const [entriesLoading, setEntriesLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [creatingBackup, setCreatingBackup] = useState(false);
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
    if (activeView === "entries") {
      void loadEntryList();
    }
  }, [activeView, loadEntryList]);

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

  const title = {
    dashboard: "Read-Only Journal",
    entries: "Entries",
    backups: "Backups",
    settings: "Settings",
    about: "About",
  }[activeView];

  return (
    <div className="app-shell">
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
              onClick={() => setActiveView(item.id)}
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
            <p className="eyebrow">Phase 1</p>
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
            detailLoading={detailLoading}
            entryFilters={entryFilters}
            entryResponse={entryResponse}
            loading={entriesLoading}
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

        {activeView === "backups" && (
          <BackupsView
            backupDirectory={backupDirectory}
            backups={backups}
            creatingBackup={creatingBackup}
            onCreateBackup={handleCreateBackup}
            status={status}
          />
        )}

        {activeView === "settings" && (
          <SettingsView
            backupDirectory={backupDirectory}
            status={status}
            statusTone={statusTone}
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
  loading: boolean;
  detailLoading: boolean;
  onSelectEntry: (entry: Entry) => void;
  onLoadMore: () => void;
  onResetFilters: () => void;
};

function EntriesView({
  status,
  entryFilters,
  setEntryFilters,
  entryResponse,
  selectedEntry,
  loading,
  detailLoading,
  onSelectEntry,
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
          <StatusPill tone="neutral">{loading ? "Loading" : "Read-only"}</StatusPill>
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

      <EntryDetail entry={selectedEntry} loading={detailLoading} />
    </section>
  );
}

type BackupsViewProps = {
  backupDirectory: string;
  backups: BackupInfo[];
  status: DatabaseStatus | null;
  creatingBackup: boolean;
  onCreateBackup: () => void;
};

function BackupsView({
  backupDirectory,
  backups,
  status,
  creatingBackup,
  onCreateBackup,
}: BackupsViewProps) {
  return (
    <section className="backup-view" aria-label="Backup list">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Backup directory</p>
          <h3>{backupDirectory || "Not available"}</h3>
        </div>
        <button
          className="secondary-button"
          disabled={creatingBackup || !status?.dbExists}
          onClick={onCreateBackup}
          type="button"
        >
          <FileArchive size={18} />
          Create backup
        </button>
      </div>

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
            <StatusPill tone={backup.verified ? "good" : "warn"}>
              {backup.verified ? "Verified" : "Needs check"}
            </StatusPill>
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
};

function SettingsView({ status, backupDirectory, statusTone }: SettingsViewProps) {
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
      <Panel icon={<Info size={20} />} title="Application">
        <dl className="detail-list">
          <Detail label="Version" value="0.2.0" />
          <Detail label="Mode" value="Read-only journal" />
          <Detail label="Writes" value="Manual backup only" />
        </dl>
      </Panel>
    </section>
  );
}

function AboutView() {
  return (
    <section className="about-panel">
      <h3>Capsule Tauri</h3>
      <p>
        Phase 1 adds read-only browsing for the active Capsule database: dashboard counts,
        recent entries, pinned entries, random entry, filters, entry detail, tags, moods,
        locations, attachment counts, and thread metadata.
      </p>
      <p>Journal mutations remain unavailable in this phase. Manual backup creation is still available.</p>
    </section>
  );
}

type EntryDetailProps = {
  entry: Entry | null;
  loading: boolean;
};

function EntryDetail({ entry, loading }: EntryDetailProps) {
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
        <p>Select an entry to inspect its read-only details.</p>
      </aside>
    );
  }

  return (
    <aside className="detail-panel">
      <div className="entry-detail-heading">
        <p className="eyebrow">{formatDateTime(entry.createdAt)}</p>
        <h3>{entry.title || entry.textPlain.slice(0, 72) || "Untitled entry"}</h3>
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
          {(entry.location.weatherCondition || entry.location.weatherTempF !== null) && (
            <p>
              {entry.location.weatherCondition ?? "Weather"} /{" "}
              {entry.location.weatherTempF !== null
                ? `${entry.location.weatherTempF.toFixed(1)} F`
                : "No temperature"}
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
      {entry.thread && <span className="tag-chip">{entry.thread.entryCount} in thread</span>}
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

function splitFilter(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

export default App;
