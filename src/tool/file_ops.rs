//! `file` tool — Safe file operations with path traversal protection and trash-based deletion.
//!
//! Wraps desktop_monitor::file_ops functions with XDG shortcut support.
//! All operations enforce home-directory boundaries and protected directory rules.

use crate::tool::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use desktop_monitor::file_ops;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct FileOpsTool;

impl FileOpsTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum FileOpsInput {
    /// List directory contents.
    list {
        path: String,
        include_hidden: Option<bool>,
    },

    /// Read file content (truncated to max_chars).
    read {
        path: String,
        max_chars: Option<usize>,
    },

    /// Write content to file.
    write {
        path: String,
        content: String,
        append: Option<bool>,
    },

    /// Delete file (moves to system trash).
    delete { path: String },

    /// Move file to destination.
    move_file { src: String, dst: String },

    /// Copy file to destination.
    copy { src: String, dst: String },

    /// Search for files by name/extension.
    find {
        path: String,
        name_pattern: Option<String>,
        extension: Option<String>,
        max_results: Option<usize>,
        max_dirs: Option<usize>,
    },

    /// Get file metadata.
    info { path: String },

    /// Get disk usage for a path.
    disk_usage { path: String },

    /// Resolve a path (handles shortcuts like desktop, downloads, ~).
    resolve_path { raw: String },
}

#[async_trait]
impl Tool for FileOpsTool {
    fn name(&self) -> &str {
        "file"
    }

    fn description(&self) -> &str {
        "Safe file operations with path traversal protection, trash-based deletion, XDG shortcuts. \
         Actions: list, read, write, delete, move, copy, find, info, disk_usage, resolve_path"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "description": "The file operation to perform: list, read, write, delete, move, copy, find, info, disk_usage, resolve_path"
                },
                "path": { "type": "string", "description": "File or directory path" },
                "src": { "type": "string", "description": "Source path for move/copy" },
                "dst": { "type": "string", "description": "Destination path for move/copy" },
                "raw": { "type": "string", "description": "Raw path string for resolve_path (supports shortcuts: desktop, downloads, ~)" },
                "content": { "type": "string", "description": "File content for write" },
                "include_hidden": { "type": "boolean", "description": "Include hidden files in list (default: false)" },
                "max_chars": { "type": "integer", "description": "Maximum characters to read (default: 100000)" },
                "append": { "type": "boolean", "description": "Append to file instead of overwriting (for write)" },
                "name_pattern": { "type": "string", "description": "Filter files by name pattern (case-insensitive)" },
                "extension": { "type": "string", "description": "Filter files by extension (without dot)" },
                "max_results": { "type": "integer", "description": "Maximum search results (default: 100)" },
                "max_dirs": { "type": "integer", "description": "Maximum directories to scan (default: 100)" }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: FileOpsInput = serde_json::from_value(input)?;

        match input {
            FileOpsInput::list {
                path,
                include_hidden,
            } => {
                let resolved = file_ops::resolve_path(&path);
                if !file_ops::is_safe_path(&resolved) {
                    return Ok(ToolOutput::new(format!("Path not safe: {}", path)));
                }
                let entries = file_ops::list_dir(&resolved, include_hidden.unwrap_or(false))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&entries)?))
            }

            FileOpsInput::read { path, max_chars } => {
                let resolved = file_ops::resolve_path(&path);
                if !file_ops::is_safe_path(&resolved) {
                    return Ok(ToolOutput::new(format!("Path not safe: {}", path)));
                }
                let content = file_ops::read_file(&resolved, max_chars.unwrap_or(100000))?;
                Ok(ToolOutput::new(content))
            }

            FileOpsInput::write {
                path,
                content,
                append,
            } => {
                let resolved = file_ops::resolve_path(&path);
                if !file_ops::is_safe_path(&resolved) {
                    return Ok(ToolOutput::new(format!("Path not safe: {}", path)));
                }
                file_ops::write_file(&resolved, &content, append.unwrap_or(false))?;
                Ok(ToolOutput::new(format!("Written to {}", path)))
            }

            FileOpsInput::delete { path } => {
                let resolved = file_ops::resolve_path(&path);
                if !file_ops::is_safe_path(&resolved) {
                    return Ok(ToolOutput::new(format!("Path not safe: {}", path)));
                }
                let result = file_ops::delete_file(&resolved)?;
                Ok(ToolOutput::new(result))
            }

            FileOpsInput::move_file { src, dst } => {
                let src_resolved = file_ops::resolve_path(&src);
                let dst_resolved = file_ops::resolve_path(&dst);
                if !file_ops::is_safe_path(&src_resolved) {
                    return Ok(ToolOutput::new(format!("Source path not safe: {}", src)));
                }
                if !file_ops::is_safe_path(&dst_resolved) {
                    return Ok(ToolOutput::new(format!(
                        "Destination path not safe: {}",
                        dst
                    )));
                }
                let result = file_ops::move_file(&src_resolved, &dst_resolved)?;
                Ok(ToolOutput::new(format!("Moved to {}", result.display())))
            }

            FileOpsInput::copy { src, dst } => {
                let src_resolved = file_ops::resolve_path(&src);
                let dst_resolved = file_ops::resolve_path(&dst);
                if !file_ops::is_safe_path(&src_resolved) {
                    return Ok(ToolOutput::new(format!("Source path not safe: {}", src)));
                }
                if !file_ops::is_safe_path(&dst_resolved) {
                    return Ok(ToolOutput::new(format!(
                        "Destination path not safe: {}",
                        dst
                    )));
                }
                let result = file_ops::copy_file(&src_resolved, &dst_resolved)?;
                Ok(ToolOutput::new(format!("Copied to {}", result.display())))
            }

            FileOpsInput::find {
                path,
                name_pattern,
                extension,
                max_results,
                max_dirs,
            } => {
                let resolved = file_ops::resolve_path(&path);
                if !file_ops::is_safe_path(&resolved) {
                    return Ok(ToolOutput::new(format!("Path not safe: {}", path)));
                }
                let results = file_ops::find_files(
                    &resolved,
                    name_pattern.as_deref(),
                    extension.as_deref(),
                    max_results.unwrap_or(100),
                    max_dirs.unwrap_or(100),
                )?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&results)?))
            }

            FileOpsInput::info { path } => {
                let resolved = file_ops::resolve_path(&path);
                if !file_ops::is_safe_path(&resolved) {
                    return Ok(ToolOutput::new(format!("Path not safe: {}", path)));
                }
                let info = file_ops::get_file_info(&resolved)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&info)?))
            }

            FileOpsInput::disk_usage { path } => {
                let resolved = file_ops::resolve_path(&path);
                if !file_ops::is_safe_path(&resolved) {
                    return Ok(ToolOutput::new(format!("Path not safe: {}", path)));
                }
                let usage = file_ops::disk_usage(&resolved)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&usage)?))
            }

            FileOpsInput::resolve_path { raw } => {
                let resolved = file_ops::resolve_path(&raw);
                Ok(ToolOutput::new(resolved.to_string_lossy().to_string()))
            }
        }
    }
}
