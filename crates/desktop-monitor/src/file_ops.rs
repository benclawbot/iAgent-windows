//! Safe file operations with path traversal protection and recoverable deletes.
//!
//! Inspired by MARK-XXXIX's file_controller with improvements:
//! - Resolves symlinks to prevent traversal attacks
//! - Enforces home directory boundary
//! - Supports XDG shortcuts on Linux
//! - Protected directories cannot be deleted
//! - Uses system trash instead of permanent deletion

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use anyhow::{Result, anyhow};
use serde::Serialize;

// ---------------------------------------------------------------------------
// XDG / Platform helpers
// ---------------------------------------------------------------------------

/// Get the user's home directory.
pub fn home_dir() -> PathBuf {
    dirs::home_dir()
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Resolve XDG directories for Linux, fall back to macOS/Windows conventions.
pub fn xdg_dir(_env_key: &str, fallback: &str) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var(_env_key)
            .ok()
            .map(PathBuf::from)
            .and_then(|p| if p.exists() { Some(p) } else { None })
    }
    #[cfg(not(target_os = "linux"))]
    {
        home_dir()
            .join(fallback)
            .exists()
            .then(|| home_dir().join(fallback))
    }
}

pub fn desktop_dir() -> PathBuf {
    xdg_dir("XDG_DESKTOP_DIR", "Desktop").unwrap_or_else(home_dir)
}

pub fn downloads_dir() -> PathBuf {
    xdg_dir("XDG_DOWNLOAD_DIR", "Downloads").unwrap_or_else(home_dir)
}

pub fn documents_dir() -> PathBuf {
    xdg_dir("XDG_DOCUMENTS_DIR", "Documents").unwrap_or_else(home_dir)
}

pub fn pictures_dir() -> PathBuf {
    xdg_dir("XDG_PICTURES_DIR", "Pictures").unwrap_or_else(home_dir)
}

pub fn music_dir() -> PathBuf {
    xdg_dir("XDG_MUSIC_DIR", "Music").unwrap_or_else(home_dir)
}

pub fn videos_dir() -> PathBuf {
    xdg_dir("XDG_VIDEOS_DIR", "Videos").unwrap_or_else(home_dir)
}

// ---------------------------------------------------------------------------
// Shortcut resolution
// ---------------------------------------------------------------------------

/// Resolve shortcut names to actual paths.
/// Shortcuts: desktop, downloads, documents, pictures, music, videos, home
pub fn resolve_shortcut(name: &str) -> Option<PathBuf> {
    match name.trim().to_lowercase().as_str() {
        "desktop" => Some(desktop_dir()),
        "downloads" => Some(downloads_dir()),
        "documents" | "docs" => Some(documents_dir()),
        "pictures" | "images" | "photos" => Some(pictures_dir()),
        "music" => Some(music_dir()),
        "videos" => Some(videos_dir()),
        "home" | "~" => Some(home_dir()),
        _ => None,
    }
}

/// Resolve a raw path string — handles shortcuts and ~ expansion.
pub fn resolve_path(raw: &str) -> PathBuf {
    if let Some(shortcut) = resolve_shortcut(raw) {
        return shortcut;
    }
    // Expand ~ to home directory
    let expanded = if raw.starts_with('~') {
        raw.replacen('~', home_dir().to_string_lossy().as_ref(), 1)
    } else {
        raw.to_string()
    };
    PathBuf::from(expanded)
}

// ---------------------------------------------------------------------------
// Safety checks
// ---------------------------------------------------------------------------

/// Directories that cannot be deleted or modified.
pub fn protected_directories() -> Vec<PathBuf> {
    vec![
        home_dir(),
        desktop_dir(),
        downloads_dir(),
        documents_dir(),
        pictures_dir(),
        music_dir(),
        videos_dir(),
    ]
}

/// Check if `target` is safely within allowed boundaries.
/// Resolves symlinks to prevent traversal attacks.
pub fn is_safe_path(target: &Path) -> bool {
    let resolved = match target.canonicalize() {
        Ok(p) => p,
        // Path doesn't exist yet — check parent exists and is safe
        Err(_) => {
            if let Some(parent) = target.parent() {
                match parent.canonicalize() {
                    Ok(p) => p.join(target.file_name().unwrap_or_default()),
                    Err(_) => return false,
                }
            } else {
                return false;
            }
        }
    };

    let home = home_dir();
    let home_resolved = match home.canonicalize() {
        Ok(p) => p,
        Err(_) => home,
    };

    // Must be within home directory
    resolved.starts_with(&home_resolved)
}

/// Check if the target is a protected directory.
pub fn is_protected(target: &Path) -> bool {
    let resolved = match target.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    protected_directories()
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .any(|protected| resolved == protected)
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

/// Format byte size as human-readable string.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    }
}

/// Get file metadata safely.
pub fn get_file_info(path: &Path) -> Result<FileInfo> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let created = metadata.created().unwrap_or(modified);

    Ok(FileInfo {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        created: unix_timestamp(created),
        modified: unix_timestamp(modified),
        extension: path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub created: i64,
    pub modified: i64,
    pub extension: String,
}

/// Move file to system trash (recoverable delete).
#[cfg(target_os = "macos")]
pub fn move_to_trash(path: &Path) -> Result<()> {
    // Use AppleScript on macOS
    let path_str = path.to_string_lossy();
    let script = format!(
        r#"osascript -e 'tell app "Finder" to delete POSIX file "{}"'"#,
        path_str.replace('"', "\\\"")
    );
    std::process::Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn move_to_trash(path: &Path) -> Result<()> {
    // Use gio on Linux (GIO_TRASH)
    std::process::Command::new("gio")
        .args(["trash", &path.to_string_lossy()])
        .output()?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn move_to_trash(path: &Path) -> Result<()> {
    // Use PowerShell on Windows
    let path_str = path.to_string_lossy();
    let script = format!(
        r#"Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')"#,
        path_str.replace("'", "''")
    );
    std::process::Command::new("powershell")
        .args(["-Command", &script])
        .output()?;
    Ok(())
}

/// List directory contents.
pub fn list_dir(path: &Path, include_hidden: bool) -> Result<Vec<DirEntry>> {
    if !path.is_dir() {
        return Err(anyhow!("Not a directory: {}", path.display()));
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files if not requested
        if !include_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata()?;
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);

        entries.push(DirEntry {
            name,
            is_dir,
            size,
            modified: unix_timestamp(modified),
            extension: entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default(),
        });
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(entries)
}

#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
    pub extension: String,
}

/// Create a new file with content.
pub fn create_file(path: &Path, content: &str) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Create a new directory.
pub fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

/// Read file content (truncated to max_chars).
pub fn read_file(path: &Path, max_chars: usize) -> Result<String> {
    let content = fs::read_to_string(path)?;
    if content.len() > max_chars {
        Ok(format!(
            "{}\n\n[Truncated — {} total chars]",
            &content[..max_chars],
            content.len()
        ))
    } else {
        Ok(content)
    }
}

/// Write content to file (optionally append).
pub fn write_file(path: &Path, content: &str, append: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if append {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all(content.as_bytes())?;
    } else {
        fs::write(path, content)?;
    }
    Ok(())
}

/// Rename a file.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("No parent directory"))?;
    let new_path = parent.join(new_name);

    if new_path.exists() {
        return Err(anyhow!("Target already exists: {}", new_path.display()));
    }

    fs::rename(path, &new_path)?;
    Ok(new_path)
}

/// Move file to destination.
pub fn move_file(src: &Path, dst: &Path) -> Result<PathBuf> {
    let final_dst = if dst.is_dir() {
        dst.join(src.file_name().unwrap_or_default())
    } else {
        dst.to_path_buf()
    };

    fs::rename(src, &final_dst)?;
    Ok(final_dst)
}

/// Copy file to destination.
pub fn copy_file(src: &Path, dst: &Path) -> Result<PathBuf> {
    let final_dst = if dst.is_dir() {
        dst.join(src.file_name().unwrap_or_default())
    } else {
        dst.to_path_buf()
    };

    if src.is_dir() {
        fs::create_dir_all(&final_dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_entry = entry.path();
            let dst_entry = final_dst.join(entry.file_name().clone());
            copy_file(&src_entry, &dst_entry)?;
        }
    } else {
        fs::copy(src, &final_dst)?;
    }

    Ok(final_dst)
}

/// Delete file (moves to trash, doesn't permanently delete).
pub fn delete_file(path: &Path) -> Result<String> {
    if is_protected(path) {
        return Err(anyhow!(
            "Cannot delete protected directory: {}",
            path.display()
        ));
    }

    move_to_trash(path)?;
    Ok(format!(
        "Moved to trash: {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}

/// Search for files by name/extension with a max depth.
pub fn find_files(
    root: &Path,
    name_pattern: Option<&str>,
    extension: Option<&str>,
    max_results: usize,
    max_dirs: usize,
) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();
    let mut dir_count = 0;

    for entry in walkdir(root)? {
        if entry.is_dir {
            dir_count += 1;
            if dir_count > max_dirs {
                break;
            }
            continue;
        }

        // Filter by name pattern
        if let Some(pattern) = name_pattern {
            if !entry.name.to_lowercase().contains(&pattern.to_lowercase()) {
                continue;
            }
        }

        // Filter by extension
        if let Some(ext) = extension {
            let entry_ext = entry.extension.to_lowercase();
            let filter_ext = ext.trim_start_matches('.').to_lowercase();
            if entry_ext != filter_ext {
                continue;
            }
        }

        results.push(entry);

        if results.len() >= max_results {
            break;
        }
    }

    Ok(results)
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub extension: String,
    pub is_dir: bool,
}

fn walkdir(root: &Path) -> Result<Vec<SearchResult>> {
    let mut entries = Vec::new();

    fn walk(path: &Path, entries: &mut Vec<SearchResult>) -> std::io::Result<()> {
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                let name = entry.file_name().to_string_lossy().to_string();

                if file_type.is_dir() {
                    if !name.starts_with('.') {
                        walk(&entry.path(), entries)?;
                    }
                } else {
                    let metadata = entry.metadata()?;
                    let ext = Path::new(&name)
                        .extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_default();

                    entries.push(SearchResult {
                        name,
                        path: entry.path(),
                        size: metadata.len(),
                        extension: ext,
                        is_dir: false,
                    });
                }
            }
        }
        Ok(())
    }

    walk(root, &mut entries)?;
    Ok(entries)
}

/// Get disk usage for a path.
#[cfg(target_os = "linux")]
pub fn disk_usage(path: &Path) -> Result<DiskUsage> {
    use std::ffi::CString;

    // On Linux, use statvfs for accurate filesystem stats
    let path_str = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| anyhow!("path contains an interior NUL: {}", path.display()))?;
    let statvfs: libc::statvfs = unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(path_str.as_ptr(), &mut stat) != 0 {
            return Err(anyhow!(
                "Failed to get filesystem stats for {}",
                path.display()
            ));
        }
        stat
    };

    let total = statvfs.f_blocks.saturating_mul(statvfs.f_frsize as u64);
    let free = statvfs.f_bavail.saturating_mul(statvfs.f_frsize as u64);
    let used = total.saturating_sub(free);

    Ok(DiskUsage {
        total,
        used,
        free,
        percent_used: if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        },
    })
}

#[cfg(not(target_os = "linux"))]
pub fn disk_usage(path: &Path) -> Result<DiskUsage> {
    let used = if path.is_file() {
        fs::metadata(path)?.len()
    } else {
        let mut total = 0u64;
        for entry in walkdir(path)? {
            if !entry.is_dir {
                total = total.saturating_add(entry.size);
            }
        }
        total
    };

    Ok(DiskUsage {
        total: used,
        used,
        free: 0,
        percent_used: if used > 0 { 100.0 } else { 0.0 },
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskUsage {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub percent_used: f64,
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn unix_timestamp(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_resolve_shortcut() {
        assert!(resolve_shortcut("desktop").is_some());
        assert!(resolve_shortcut("downloads").is_some());
        assert!(resolve_shortcut("DOCUMENTS").is_some());
        assert!(resolve_shortcut("unknown").is_none());
    }

    #[test]
    fn test_resolve_path() {
        let path = resolve_path("desktop");
        assert!(path.exists() || path.to_string_lossy().contains("Desktop"));

        let with_tilde = resolve_path("~/Documents");
        assert!(with_tilde.to_string_lossy().contains("Documents"));
    }

    #[test]
    fn test_is_safe_path() {
        // Paths within home should be safe
        let home = home_dir();
        assert!(is_safe_path(&home.join("some_file.txt")));

        // Path traversal should be blocked
        let dangerous = home.join("..").join("..").join("etc").join("passwd");
        assert!(!is_safe_path(&dangerous));
    }
}
