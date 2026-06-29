import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Archive,
  CheckCircle2,
  Database,
  FileArchive,
  HardDrive,
  Plus,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import { createBackup, getDatabaseStatus, listBackups } from "./backend";
import { StatusPill } from "./components/StatusPill";
import { formatBytes, formatDateTime } from "./lib/format";
import type { BackupInfo, DatabaseStatus } from "./types";
import "./styles.css";

type ActiveView = "dashboard" | "backups" | "about";

const navItems: Array<{ id: ActiveView; label: string }> = [
  { id: "dashboard", label: "Dashboard" },
  { id: "backups", label: "Backups" },
  { id: "about", label: "About" },
];

function App() {
  const [activeView, setActiveView] = useState<ActiveView>("dashboard");
  const [status, setStatus] = useState<DatabaseStatus | null>(null);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupDirectory, setBackupDirectory] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [creatingBackup, setCreatingBackup] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [nextStatus, nextBackups] = await Promise.all([
        getDatabaseStatus(),
        listBackups(),
      ]);
      setStatus(nextStatus);
      setBackups(nextBackups.backups);
      setBackupDirectory(nextBackups.backupDirectory);
    } catch (refreshError) {
      setError(refreshError instanceof Error ? refreshError.message : "Unable to refresh");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

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

  const statusTone = useMemo(() => {
    if (!status || !status.dbExists || !status.readable) {
      return "warn";
    }

    return status.warnings.length > 0 ? "neutral" : "good";
  }, [status]);

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
              {item.label}
            </button>
          ))}
        </nav>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Phase 0</p>
            <h2>
              {activeView === "dashboard" && "Safety Baseline"}
              {activeView === "backups" && "Backups"}
              {activeView === "about" && "About"}
            </h2>
          </div>

          <div className="topbar-actions">
            {status && (
              <StatusPill tone={statusTone}>
                {status.readable ? "Database readable" : "Database needs attention"}
              </StatusPill>
            )}
            <button
              aria-label="Refresh database status"
              className="icon-button"
              disabled={loading}
              onClick={refresh}
              title="Refresh database status"
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
              <Plus size={18} />
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
          <section className="content-grid" aria-label="Database dashboard">
            <Panel
              icon={<Database size={20} />}
              title="Database"
              action={<StatusPill tone={statusTone}>{status?.security.mode ?? "unknown"}</StatusPill>}
            >
              <dl className="detail-list">
                <Detail label="Path" value={status?.dbPath ?? "Loading"} />
                <Detail label="Exists" value={status?.dbExists ? "Yes" : "No"} />
                <Detail label="Readable" value={status?.readable ? "Yes" : "No"} />
                <Detail label="Size" value={formatBytes(status?.dbSizeBytes)} />
                <Detail label="Modified" value={formatDateTime(status?.dbModifiedAt)} />
              </dl>
            </Panel>

            <Panel icon={<HardDrive size={20} />} title="Schema">
              <dl className="metric-list">
                <Metric label="Entries" value={status?.entryCount ?? "Unknown"} />
                <Metric label="Tags" value={status?.tagCount ?? "Unknown"} />
                <Metric label="Tables" value={status?.schemaSummary.tableCount ?? 0} />
              </dl>
              {status?.schemaSummary.missingCoreTables.length ? (
                <p className="muted">
                  Missing: {status.schemaSummary.missingCoreTables.join(", ")}
                </p>
              ) : (
                <p className="muted">Core tables detected.</p>
              )}
            </Panel>

            <Panel icon={<Archive size={20} />} title="Backup Safety">
              <dl className="detail-list">
                <Detail label="Directory" value={backupDirectory || "Not available"} />
                <Detail label="Backups" value={status?.backupCount ?? backups.length} />
                <Detail label="Last backup" value={status?.lastBackupPath ?? "No backups found"} />
              </dl>
            </Panel>

            <Panel icon={<ShieldCheck size={20} />} title="Warnings">
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
          </section>
        )}

        {activeView === "backups" && (
          <section className="backup-view" aria-label="Backup list">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Backup directory</p>
                <h3>{backupDirectory || "Not available"}</h3>
              </div>
              <button
                className="secondary-button"
                disabled={creatingBackup || !status?.dbExists}
                onClick={handleCreateBackup}
                type="button"
              >
                <FileArchive size={18} />
                Create backup
              </button>
            </div>

            <div className="backup-list">
              {backups.length === 0 && (
                <div className="empty-state">No Capsule backups found yet.</div>
              )}
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
        )}

        {activeView === "about" && (
          <section className="about-panel">
            <h3>Capsule Tauri</h3>
            <p>
              This Phase 0 build proves the desktop shell, mockable frontend backend facade,
              read-only database status, backup listing, and manual SQLite backup creation.
            </p>
            <p>
              Journal browsing and writes are intentionally absent until the read model and backup
              guard are implemented in later phases.
            </p>
          </section>
        )}
      </main>
    </div>
  );
}

type PanelProps = {
  icon: React.ReactNode;
  title: string;
  action?: React.ReactNode;
  children: React.ReactNode;
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
  value: React.ReactNode;
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

export default App;
