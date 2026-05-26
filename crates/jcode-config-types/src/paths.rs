//! Canonical runtime path resolution for iAgent.
//!
//! All components must use these functions rather than hardcoding paths.

use std::path::PathBuf;
use anyhow::{Context, Result};

const APP_NAME: &str = "iAgent";
const LEGACY_APP_NAME: &str = "jcode";

/// Returns %LOCALAPPDATA%\iAgent on Windows, $HOME/.local/share/iAgent elsewhere.
pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .context("cannot determine local data directory")?;
    Ok(base.join(APP_NAME))
}

/// Returns %APPDATA%\iAgent on Windows, $HOME/.config/iAgent elsewhere.
pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .context("cannot determine config directory")?;
    Ok(base.join(APP_NAME))
}

/// Returns the log directory: <data_dir>/logs
pub fn log_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("logs"))
}

/// Returns the sessions directory: <data_dir>/sessions
pub fn sessions_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("sessions"))
}

/// On first run, migrate data from the legacy jcode paths if they exist.
///
/// Copies %LOCALAPPDATA%\jcode → %LOCALAPPDATA%\iAgent if the destination
/// does not yet exist. Leaves a `MIGRATED_TO_IAGENT` marker file in the
/// source directory so this runs only once.
pub fn migrate_legacy_paths() -> Result<()> {
    let base = dirs::data_local_dir()
        .context("cannot determine local data directory")?;
    let legacy = base.join(LEGACY_APP_NAME);
    let current = base.join(APP_NAME);
    let marker = legacy.join("MIGRATED_TO_IAGENT");

    if !legacy.exists() || marker.exists() || current.exists() {
        // Nothing to migrate or already done.
        return Ok(());
    }

    tracing::info!(
        "Migrating data from {} to {}",
        legacy.display(),
        current.display()
    );

    copy_dir_all(&legacy, &current)
        .with_context(|| format!(
            "migrate {} → {}", legacy.display(), current.display()
        ))?;

    // Write marker so we don't migrate again.
    std::fs::write(&marker, format!(
        "Migrated to {} on {}\n",
        current.display(),
        chrono::Utc::now().to_rfc3339()
    ))?;

    tracing::info!("Migration complete.");
    Ok(())
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

/// Legacy alias — prefer data_dir() directly.
#[deprecated(since = "0.12.2", note = "use data_dir() instead")]
pub fn jcode_dir() -> Result<PathBuf> {
    data_dir()
}