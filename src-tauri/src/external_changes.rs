//! Read-only external journal change detection for the desktop shell.
//!
//! The watcher intentionally owns one SQLite connection for its whole life.
//! SQLite's `data_version` is only useful when observations are compared on
//! that same connection, and a read-only connection keeps this boundary from
//! accidentally repairing or mutating the journal.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use rusqlite::Connection;
use serde::Serialize;

use crate::db;

const WAL_SUFFIX: &str = "-wal";

/// Result of one external-change probe.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalChangeStatus {
    /// Whether the frontend should refresh its read models.
    pub changed: bool,
    /// Whether the configured database could be opened read-only.
    pub available: bool,
    /// Whether the persistent connection was opened during this probe.
    pub reopened: bool,
    /// The path resolved for this probe (even when it is unavailable).
    pub database_path: String,
    /// A stable, frontend-safe reason for the result.
    pub reason: Option<String>,
}

struct ActiveConnection {
    database_identity: db::FileIdentity,
    wal_identity: db::FileIdentity,
    data_version: i64,
    connection: Connection,
}

/// Persistent, mutex-confined watcher state.  `rusqlite::Connection` is
/// deliberately never shared with the UI or another worker; each probe takes
/// the mutex and performs all connection work on that same state boundary.
#[derive(Default)]
pub struct ExternalChangeWatcher {
    active: Option<ActiveConnection>,
    last_path: Option<PathBuf>,
    last_database_identity: Option<db::FileIdentity>,
    last_wal_identity: Option<db::FileIdentity>,
    last_available: bool,
    observed: bool,
}

impl ExternalChangeWatcher {
    /// Probe an explicit path.  This seam keeps fixture tests independent of
    /// process environment and the user's live Capsule database.
    pub fn check_path(&mut self, path: impl Into<PathBuf>) -> ExternalChangeStatus {
        let path = path.into();
        let path_string = path.to_string_lossy().to_string();
        let was_available = self.active.is_some();
        let previous_path = self.last_path.replace(path.clone());
        let was_observed = self.observed;
        let path_changed = previous_path
            .as_ref()
            .map(|previous| {
                db::FileIdentity::for_path(previous).canonical_path
                    != db::FileIdentity::for_path(&path).canonical_path
            })
            .unwrap_or(false);
        self.observed = true;

        if !path.is_file() {
            let was_available = self.active.take().is_some() || self.last_available;
            self.last_available = false;
            return ExternalChangeStatus {
                changed: was_available || path_changed,
                available: false,
                reopened: false,
                database_path: path_string,
                reason: Some(if path_changed {
                    "database_path_changed".to_string()
                } else {
                    "database_unavailable".to_string()
                }),
            };
        }

        let database_identity = db::FileIdentity::for_path(&path);
        let wal_path = wal_path(&path);
        let wal_identity = db::FileIdentity::for_path(&wal_path);

        let mut reopen_reason: Option<&'static str> = None;
        if let Some(active) = self.active.as_ref() {
            let replacement_reason = if path_changed {
                Some("database_path_changed")
            } else if !active.database_identity.same_file(&database_identity) {
                Some("database_replaced")
            // SQLite commonly creates/removes the `-wal` sidecar as writers
            // come and go.  That lifecycle is not itself replacement: the
            // persistent connection's `data_version` remains the source of
            // truth for commits.  When both observations have a concrete OS
            // identity, however, a different identity means the sidecar was
            // swapped and the read connection must be reopened.
            } else if active.wal_identity.stable_id.is_some()
                && wal_identity.stable_id.is_some()
                && !active.wal_identity.same_file(&wal_identity)
            {
                Some("wal_replaced")
            } else {
                None
            };

            if replacement_reason.is_none() {
                let connection = &active.connection;
                match data_version(connection) {
                    Ok(next_version) if next_version != active.data_version => {
                        if let Some(active) = self.active.as_mut() {
                            active.data_version = next_version;
                            active.wal_identity = wal_identity;
                        }
                        return ExternalChangeStatus {
                            changed: true,
                            available: true,
                            reopened: false,
                            database_path: path_string,
                            reason: Some("data_version".to_string()),
                        };
                    }
                    Ok(_) => {
                        return ExternalChangeStatus {
                            changed: false,
                            available: true,
                            reopened: false,
                            database_path: path_string,
                            reason: None,
                        }
                    }
                    Err(error) => {
                        self.active = None;
                        self.last_available = false;
                        return ExternalChangeStatus {
                            changed: true,
                            available: false,
                            reopened: false,
                            database_path: path_string,
                            reason: Some(format!("database_unavailable: {error}")),
                        };
                    }
                }
            } else {
                reopen_reason = replacement_reason;
            }
        } else if was_observed {
            reopen_reason = if path_changed {
                Some("database_path_changed")
            } else if self
                .last_database_identity
                .as_ref()
                .map(|previous| !previous.same_file(&database_identity))
                .unwrap_or(false)
            {
                Some("database_replaced")
            } else if self
                .last_wal_identity
                .as_ref()
                .filter(|previous| previous.stable_id.is_some() && wal_identity.stable_id.is_some())
                .map(|previous| !previous.same_file(&wal_identity))
                .unwrap_or(false)
            {
                Some("wal_replaced")
            } else {
                None
            };
        }

        let had_connection = self.active.take().is_some();
        match open_active(&path, database_identity, wal_identity) {
            Ok(active) => {
                self.last_database_identity = Some(active.database_identity.clone());
                self.last_wal_identity = Some(active.wal_identity.clone());
                self.active = Some(active);
                self.last_available = true;
                ExternalChangeStatus {
                    changed: was_observed
                        && (!was_available || path_changed || reopen_reason.is_some()),
                    available: true,
                    reopened: was_observed
                        && (!was_available || path_changed || reopen_reason.is_some()),
                    database_path: path_string,
                    reason: if let Some(reason) = reopen_reason {
                        Some(reason.to_string())
                    } else if path_changed {
                        Some("database_path_changed".to_string())
                    } else if had_connection {
                        Some("database_replaced".to_string())
                    } else if was_observed && !was_available {
                        Some("database_recovered".to_string())
                    } else {
                        None
                    },
                }
            }
            Err(error) => {
                self.last_available = false;
                ExternalChangeStatus {
                    changed: had_connection || path_changed || reopen_reason.is_some(),
                    available: false,
                    reopened: false,
                    database_path: path_string,
                    reason: Some(if let Some(reason) = reopen_reason {
                        format!("{reason}: database_unavailable: {error}")
                    } else {
                        format!("database_unavailable: {error}")
                    }),
                }
            }
        }
    }

    /// Probe the path currently resolved by Capsule's path policy.
    pub fn check_resolved(&mut self) -> ExternalChangeStatus {
        self.check_path(db::resolve_database_path())
    }

    /// Drop the retained connection while keeping the last observed identity.
    /// The desktop normally relies on `Drop`; this explicit seam is useful for
    /// shutdown and for platform fixtures where an OS does not allow an open
    /// SQLite handle to be atomically replaced.
    pub fn close(&mut self) {
        self.active = None;
    }
}

fn open_active(
    path: &Path,
    database_identity: db::FileIdentity,
    wal_identity: db::FileIdentity,
) -> anyhow::Result<ActiveConnection> {
    let connection = db::open_read_only_connection_with_timeout(path, Duration::from_millis(100))?;
    let data_version = data_version(&connection)?;
    Ok(ActiveConnection {
        database_identity,
        wal_identity,
        data_version,
        connection,
    })
}

fn data_version(connection: &Connection) -> anyhow::Result<i64> {
    Ok(connection.query_row("PRAGMA data_version", [], |row| row.get(0))?)
}

fn wal_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(WAL_SUFFIX);
    PathBuf::from(value)
}

/// Tauri-managed state.  Keeping the `Arc<Mutex<...>>` behind this small type
/// makes the command boundary Send/Sync-safe without exposing a connection to
/// the webview or requiring a daemon process.
#[derive(Clone, Default)]
pub struct ExternalChangeState(Arc<Mutex<ExternalChangeWatcher>>);

impl ExternalChangeState {
    pub fn check_resolved(&self) -> ExternalChangeStatus {
        let mut watcher = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        watcher.check_resolved()
    }
}

impl Drop for ExternalChangeWatcher {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use super::*;

    fn create_database(path: &Path, seed: &str) {
        let connection = Connection::open(path).expect("open fixture database");
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE IF NOT EXISTS entries (id INTEGER PRIMARY KEY, body TEXT NOT NULL);
                 DELETE FROM entries;
                ",
            )
            .expect("create fixture schema");
        connection
            .execute("INSERT INTO entries (body) VALUES (?1)", [seed])
            .expect("seed fixture");
    }

    fn external_write(path: &Path, body: &str) {
        let connection = Connection::open(path).expect("open external writer");
        connection
            .execute("INSERT INTO entries (body) VALUES (?1)", [body])
            .expect("write externally");
    }

    #[test]
    fn detects_wal_commit_without_main_database_mtime_change() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("journal.db");
        create_database(&path, "first");
        let before = fs::metadata(&path)
            .expect("database metadata")
            .modified()
            .expect("database mtime");

        let mut watcher = ExternalChangeWatcher::default();
        assert!(!watcher.check_path(&path).changed);
        external_write(&path, "from another connection");
        let after = fs::metadata(&path)
            .expect("database metadata")
            .modified()
            .expect("database mtime");

        assert_eq!(before, after, "WAL-only commit changed the main mtime");
        let status = watcher.check_path(&path);
        assert_eq!(status.reason.as_deref(), Some("data_version"));
        assert!(status.changed);
        assert!(status.available);
        assert!(!watcher.check_path(&path).changed);
    }

    #[test]
    fn coalesces_rapid_external_writes_into_a_new_version() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("journal.db");
        create_database(&path, "first");
        let mut watcher = ExternalChangeWatcher::default();
        watcher.check_path(&path);

        for index in 0..12 {
            external_write(&path, &format!("rapid-{index}"));
        }

        let status = watcher.check_path(&path);
        assert!(status.changed);
        assert_eq!(status.reason.as_deref(), Some("data_version"));
        assert!(!watcher.check_path(&path).changed);
    }

    #[test]
    fn reports_no_change_for_repeated_probes() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("journal.db");
        create_database(&path, "first");
        let mut watcher = ExternalChangeWatcher::default();
        let first = watcher.check_path(&path);
        let second = watcher.check_path(&path);

        assert!(first.available);
        assert!(!first.changed);
        assert!(!second.changed);
        assert!(second.reason.is_none());
    }

    #[test]
    fn reopens_after_same_path_database_replacement() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("journal.db");
        let replacement = directory.path().join("replacement.db");
        create_database(&path, "first");
        create_database(&replacement, "replacement");
        let mut watcher = ExternalChangeWatcher::default();
        watcher.check_path(&path);

        watcher.close();
        fs::remove_file(&path).expect("remove old database");
        fs::rename(&replacement, &path).expect("replace database");
        let status = watcher.check_path(&path);

        assert!(status.available);
        assert!(status.changed);
        assert!(status.reopened);
        assert_eq!(status.reason.as_deref(), Some("database_replaced"));
        assert!(!watcher.check_path(&path).changed);
    }

    #[test]
    fn reopens_when_wal_sidecar_is_replaced() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("journal.db");
        let wal = wal_path(&path);
        let replacement = directory.path().join("replacement-wal");
        create_database(&path, "first");
        let writer = Connection::open(&path).expect("open writer");
        writer
            .execute("INSERT INTO entries (body) VALUES ('wal')", [])
            .expect("write WAL");
        assert!(wal.exists(), "fixture should have a WAL sidecar");
        fs::write(&replacement, fs::read(&wal).expect("read WAL")).expect("copy WAL");
        let mut watcher = ExternalChangeWatcher::default();
        watcher.check_path(&path);

        // Keep the replacement test independent of SQLite's checkpoint timing:
        // close the retained connection before the filesystem swap, while the
        // identity snapshot remains bound to the prior sidecar.  Windows may
        // checkpoint and remove the sidecar when the writer closes, so copy a
        // valid prior WAL back into place to exercise identity replacement.
        watcher.close();
        drop(writer);
        if wal.exists() {
            fs::remove_file(&wal).expect("remove old WAL");
        }
        fs::rename(&replacement, &wal).expect("replace WAL");
        let status = watcher.check_path(&path);

        assert!(status.changed);
        assert!(status
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("wal_replaced")));
    }

    #[test]
    fn reports_unavailable_path_and_recovers_without_creating_a_database() {
        let directory = tempfile::tempdir().expect("tempdir");
        let first = directory.path().join("first.db");
        let recovered = directory.path().join("recovered.db");
        create_database(&first, "first");
        let mut watcher = ExternalChangeWatcher::default();
        assert!(watcher.check_path(&first).available);

        let missing = directory.path().join("missing.db");
        let unavailable = watcher.check_path(&missing);
        assert!(!unavailable.available);
        assert!(unavailable.changed);
        assert!(
            !missing.exists(),
            "watcher must not create a missing database"
        );
        assert!(!watcher.check_path(&missing).changed);

        create_database(&recovered, "recovered");
        let recovered_status = watcher.check_path(&recovered);
        assert!(recovered_status.available);
        assert!(recovered_status.changed);
        assert!(recovered_status.reopened);
    }

    #[test]
    fn recovers_after_same_path_database_disappears_and_returns() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("journal.db");
        let replacement = directory.path().join("replacement.db");
        create_database(&path, "first");
        create_database(&replacement, "replacement");
        let mut watcher = ExternalChangeWatcher::default();
        assert!(watcher.check_path(&path).available);

        watcher.close();
        fs::remove_file(&path).expect("remove database");
        let unavailable = watcher.check_path(&path);
        assert!(!unavailable.available);
        assert!(unavailable.changed);

        fs::rename(&replacement, &path).expect("restore database at same path");
        let recovered = watcher.check_path(&path);
        assert!(recovered.available);
        assert!(recovered.changed);
        assert!(recovered.reopened);
    }

    #[test]
    fn independent_connections_are_used_for_external_writes() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("journal.db");
        create_database(&path, "first");
        let mut watcher = ExternalChangeWatcher::default();
        watcher.check_path(&path);
        let before = SystemTime::now();
        external_write(&path, "independent");
        let status = watcher.check_path(&path);

        assert!(SystemTime::now() >= before);
        assert!(status.changed);
        assert_eq!(status.reason.as_deref(), Some("data_version"));
    }
}
