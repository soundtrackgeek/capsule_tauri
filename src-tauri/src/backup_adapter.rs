use std::{fs, process::Command};

use anyhow::{Context, Result};

use crate::db;

/// Desktop-only shell integration.  The shared core intentionally has no
/// dependency on Explorer, a windowing toolkit, or process lifecycle APIs.
pub fn open_backup_folder() -> Result<()> {
    let backup_directory = db::backup_directory_for_database(&db::resolve_database_path());
    fs::create_dir_all(&backup_directory)
        .with_context(|| format!("failed to create {}", backup_directory.display()))?;
    Command::new("explorer.exe")
        .arg(&backup_directory)
        .spawn()
        .with_context(|| format!("failed to open {}", backup_directory.display()))?;
    Ok(())
}
