use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use chrono::Local;
use notify::{Watcher, RecursiveMode};
use walkdir::WalkDir;
use std::sync::{Arc, Mutex};
use regex::Regex;

pub struct FilesystemModule {
    snapshots: Arc<Mutex<HashMap<String, Vec<SnapshotInfo>>>>,
}

#[derive(Clone)]
struct SnapshotInfo {
    #[allow(dead_code)]
    timestamp: String,
    path: PathBuf,
    compressed: bool,
}

impl FilesystemModule {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "fs_read",
                "description": "Read file contents, optionally reading specific line ranges",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        },
                        "lines": {
                            "type": "array",
                            "description": "Optional array of line ranges to read, e.g. [[1,10], [15,20]] reads lines 1-10 and 15-20",
                            "items": {
                                "type": "array",
                                "minItems": 2,
                                "maxItems": 2,
                                "items": {
                                    "type": "integer"
                                }
                            }
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_write",
                "description": "Write content to a file, optionally writing to specific line ranges",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        },
                        "lines": {
                            "type": "array",
                            "description": "Optional array of line ranges to replace, e.g. [[1,10], [15,20]]. Content will be split and replace specified ranges.",
                            "items": {
                                "type": "array",
                                "minItems": 2,
                                "maxItems": 2,
                                "items": {
                                    "type": "integer"
                                }
                            }
                        }
                    },
                    "required": ["path", "content"]
                }
            }),
            json!({
                "name": "fs_move",
                "description": "Move files or directories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source path"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination path"
                        }
                    },
                    "required": ["source", "destination"]
                }
            }),
            json!({
                "name": "fs_copy",
                "description": "Copy files or directories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source path"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination path"
                        }
                    },
                    "required": ["source", "destination"]
                }
            }),
            json!({
                "name": "fs_create",
                "description": "Create files or directories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to create"
                        },
                        "type": {
                            "type": "string",
                            "enum": ["file", "dir"],
                            "description": "Type to create (file or dir)"
                        }
                    },
                    "required": ["path", "type"]
                }
            }),
            json!({
                "name": "fs_delete",
                "description": "Delete files or directories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to delete"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_move_desktop",
                "description": "Move and organize items within the Desktop directory",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "item": {
                            "type": "string",
                            "description": "Item name or path within Desktop"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination subfolder within Desktop"
                        }
                    },
                    "required": ["item", "destination"]
                }
            }),
            json!({
                "name": "fs_find",
                "description": "Search for files and directories by name pattern",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Root path to search from"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (substring match, or glob with wildcards like *.rs)"
                        },
                        "type": {
                            "type": "string",
                            "enum": ["file", "dir", "all"],
                            "description": "Type to search for (default: all)"
                        },
                        "max_depth": {
                            "type": "number",
                            "description": "Maximum directory depth to search (default: unlimited)"
                        },
                        "max_results": {
                            "type": "number",
                            "description": "Maximum number of results to return (default: 1000)"
                        }
                    },
                    "required": ["path", "pattern"]
                }
            }),
            json!({
                "name": "fs_ld",
                "description": "List directory contents with details (like ls -la)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to list"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_stat",
                "description": "Get file/directory metadata and statistics",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to get stats for"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_permissions",
                "description": "Get or set file permissions",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to file/directory"
                        },
                        "mode": {
                            "type": "string",
                            "description": "Permission mode (e.g., '755', '644') - omit to get current permissions"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_watch",
                "description": "Watch a file or directory for changes",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to watch"
                        },
                        "duration": {
                            "type": "number",
                            "description": "Duration to watch in seconds (default: 60)"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_snapshot",
                "description": "Create lightweight timestamped backups with automatic management",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to snapshot"
                        },
                        "max_snapshots": {
                            "type": "number",
                            "description": "Maximum number of snapshots to keep (default: 10)"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_tree",
                "description": "Display a visual directory tree structure. Much faster than recursive fs_find + fs_ld for understanding project layout.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Root directory path"
                        },
                        "max_depth": {
                            "type": "number",
                            "description": "Maximum depth to display (default: 4)"
                        },
                        "show_hidden": {
                            "type": "boolean",
                            "description": "Show hidden files/directories (default: false)"
                        },
                        "show_size": {
                            "type": "boolean",
                            "description": "Show file sizes (default: false)"
                        },
                        "dirs_only": {
                            "type": "boolean",
                            "description": "Only show directories (default: false)"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Only show entries matching this pattern (substring match)"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_grep",
                "description": "Search file contents by pattern across a directory. Returns matching lines with file paths and line numbers.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File or directory to search in"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (regex supported)"
                        },
                        "case_insensitive": {
                            "type": "boolean",
                            "description": "Case-insensitive search (default: false)"
                        },
                        "max_results": {
                            "type": "number",
                            "description": "Maximum number of matches to return (default: 200)"
                        },
                        "context_lines": {
                            "type": "number",
                            "description": "Number of context lines before/after each match (default: 0)"
                        },
                        "file_pattern": {
                            "type": "string",
                            "description": "Only search files matching this pattern (e.g. '*.rs', '*.py')"
                        }
                    },
                    "required": ["path", "pattern"]
                }
            }),
            json!({
                "name": "fs_tail",
                "description": "Read the last N lines of a file. Essential for reading log files and build output.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "lines": {
                            "type": "number",
                            "description": "Number of lines from the end (default: 20)"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_replace",
                "description": "Find and replace text across one or multiple files. Supports regex patterns and directory-wide bulk replacement.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File or directory to perform replacements in"
                        },
                        "find": {
                            "type": "string",
                            "description": "Text or regex pattern to find"
                        },
                        "replace": {
                            "type": "string",
                            "description": "Replacement text (supports $1 capture groups with regex)"
                        },
                        "regex": {
                            "type": "boolean",
                            "description": "Treat find as regex pattern (default: false)"
                        },
                        "file_pattern": {
                            "type": "string",
                            "description": "Only process files matching this pattern (e.g. '*.rs') — required for directories"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "Preview changes without writing (default: false)"
                        }
                    },
                    "required": ["path", "find", "replace"]
                }
            }),
        ]
    }

    pub async fn read(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let full_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path))?;

        let total_lines = full_content.lines().count();

        // Check if lines parameter is provided
        let content = if let Some(lines_array) = args.get("lines").and_then(|v| v.as_array()) {
            let all_lines: Vec<&str> = full_content.lines().collect();
            let mut selected_lines = Vec::new();

            for range in lines_array {
                if let Some(range_arr) = range.as_array() {
                    if range_arr.len() == 2 {
                        let from = range_arr[0].as_i64().unwrap_or(1) as usize;
                        let to = range_arr[1].as_i64().unwrap_or(all_lines.len() as i64) as usize;

                        let from_idx = from.saturating_sub(1);
                        let to_idx = to.min(all_lines.len());

                        if from_idx < all_lines.len() && from_idx < to_idx {
                            selected_lines.extend_from_slice(&all_lines[from_idx..to_idx]);
                        }
                    }
                }
            }

            selected_lines.join("\n")
        } else {
            full_content
        };

        Ok(json!({
            "path": path,
            "content": content,
            "size": content.len(),
            "total_lines": total_lines
        }))
    }

    pub async fn write(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let content = args["content"].as_str().context("Missing 'content' parameter")?;

        // Check if lines parameter is provided
        let final_content = if let Some(lines_array) = args.get("lines").and_then(|v| v.as_array()) {
            // Read existing file or create empty if not exists
            let existing_content = fs::read_to_string(path).unwrap_or_default();
            let mut all_lines: Vec<String> = existing_content.lines().map(|s| s.to_string()).collect();
            let new_lines: Vec<&str> = content.lines().collect();
            let mut new_line_idx = 0;

            for range in lines_array {
                if let Some(range_arr) = range.as_array() {
                    if range_arr.len() == 2 {
                        let from = range_arr[0].as_i64().unwrap_or(1) as usize;
                        let to = range_arr[1].as_i64().unwrap_or(all_lines.len() as i64) as usize;

                        // Convert from 1-indexed to 0-indexed
                        let from_idx = from.saturating_sub(1);
                        let to_idx = to.min(all_lines.len());

                        // Calculate how many lines to replace
                        let range_size = to_idx.saturating_sub(from_idx);

                        // Extend file if needed
                        while all_lines.len() < to_idx {
                            all_lines.push(String::new());
                        }

                        // Replace lines in this range with corresponding new lines
                        for i in 0..range_size {
                            if new_line_idx < new_lines.len() {
                                all_lines[from_idx + i] = new_lines[new_line_idx].to_string();
                                new_line_idx += 1;
                            }
                        }
                    }
                }
            }

            all_lines.join("\n") + "\n"
        } else {
            content.to_string()
        };

        fs::write(path, &final_content)
            .with_context(|| format!("Failed to write file: {}", path))?;

        Ok(json!({
            "success": true,
            "path": path,
            "bytes_written": final_content.len()
        }))
    }

    pub async fn move_file(&self, args: Value) -> Result<Value> {
        let source = args["source"].as_str().context("Missing 'source' parameter")?;
        let destination = args["destination"].as_str().context("Missing 'destination' parameter")?;

        fs::rename(source, destination)
            .with_context(|| format!("Failed to move from {} to {}", source, destination))?;

        Ok(json!({
            "success": true,
            "source": source,
            "destination": destination
        }))
    }

    pub async fn copy(&self, args: Value) -> Result<Value> {
        let source = args["source"].as_str().context("Missing 'source' parameter")?;
        let destination = args["destination"].as_str().context("Missing 'destination' parameter")?;

        let source_path = Path::new(source);

        if source_path.is_file() {
            fs::copy(source, destination)
                .with_context(|| format!("Failed to copy file from {} to {}", source, destination))?;
        } else if source_path.is_dir() {
            copy_dir_all(source, destination)
                .with_context(|| format!("Failed to copy directory from {} to {}", source, destination))?;
        } else {
            anyhow::bail!("Source path does not exist: {}", source);
        }

        Ok(json!({
            "success": true,
            "source": source,
            "destination": destination
        }))
    }

    pub async fn create(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let type_str = args["type"].as_str().context("Missing 'type' parameter")?;

        match type_str {
            "file" => {
                fs::File::create(path)
                    .with_context(|| format!("Failed to create file: {}", path))?;
            }
            "dir" => {
                fs::create_dir_all(path)
                    .with_context(|| format!("Failed to create directory: {}", path))?;
            }
            _ => anyhow::bail!("Invalid type: {}. Must be 'file' or 'dir'", type_str),
        }

        Ok(json!({
            "success": true,
            "path": path,
            "type": type_str
        }))
    }

    pub async fn delete(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let path_obj = Path::new(path);

        if path_obj.is_file() {
            fs::remove_file(path)
                .with_context(|| format!("Failed to delete file: {}", path))?;
        } else if path_obj.is_dir() {
            fs::remove_dir_all(path)
                .with_context(|| format!("Failed to delete directory: {}", path))?;
        } else {
            anyhow::bail!("Path does not exist: {}", path);
        }

        Ok(json!({
            "success": true,
            "path": path
        }))
    }

    pub async fn move_desktop(&self, args: Value) -> Result<Value> {
        let item = args["item"].as_str().context("Missing 'item' parameter")?;
        let destination = args["destination"].as_str().context("Missing 'destination' parameter")?;

        // Get Desktop path
        let desktop = dirs::desktop_dir()
            .context("Could not find Desktop directory")?;

        let source_path = desktop.join(item);
        let dest_path = desktop.join(destination).join(item);

        // Create destination directory if it doesn't exist
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(&source_path, &dest_path)
            .with_context(|| format!("Failed to move {} to {}", source_path.display(), dest_path.display()))?;

        Ok(json!({
            "success": true,
            "item": item,
            "from": source_path,
            "to": dest_path
        }))
    }

    pub async fn find(&self, args: Value) -> Result<Value> {
        let root_path = args["path"].as_str().context("Missing 'path' parameter")?;
        let pattern = args["pattern"].as_str().context("Missing 'pattern' parameter")?;
        let search_type = args["type"].as_str().unwrap_or("all");
        let max_results = args["max_results"].as_u64().unwrap_or(1000) as usize;

        let mut walker = WalkDir::new(root_path);
        if let Some(depth) = args["max_depth"].as_u64() {
            walker = walker.max_depth(depth as usize);
        }

        // Determine if pattern is a glob (contains * or ?)
        let is_glob = pattern.contains('*') || pattern.contains('?');

        let mut results = Vec::new();

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy();

            let matches = if is_glob {
                glob_match(pattern, &file_name)
            } else {
                file_name.contains(pattern) || path.to_string_lossy().contains(pattern)
            };

            if !matches {
                continue;
            }

            // Type filtering
            match search_type {
                "file" if !path.is_file() => continue,
                "dir" if !path.is_dir() => continue,
                _ => {}
            }

            let size = path.metadata().map(|m| m.len()).unwrap_or(0);

            results.push(json!({
                "path": path.to_string_lossy(),
                "name": file_name,
                "type": if path.is_file() { "file" } else { "dir" },
                "size": size
            }));
        }

        let truncated = results.len() >= max_results;

        Ok(json!({
            "results": results,
            "count": results.len(),
            "truncated": truncated
        }))
    }

    pub async fn ld(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let path_obj = Path::new(path);

        if !path_obj.exists() {
            anyhow::bail!("Path does not exist: {}", path);
        }

        let mut entries = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_name = entry.file_name().to_string_lossy().to_string();

            #[cfg(unix)]
            use std::os::unix::fs::PermissionsExt;

            #[cfg(unix)]
            let permissions = format!("{:o}", metadata.permissions().mode() & 0o777);

            #[cfg(not(unix))]
            let permissions = if metadata.permissions().readonly() {
                "r--".to_string()
            } else {
                "rw-".to_string()
            };

            entries.push(json!({
                "name": file_name,
                "type": if metadata.is_file() { "file" } else if metadata.is_dir() { "dir" } else { "other" },
                "size": metadata.len(),
                "permissions": permissions,
                "modified": metadata.modified().ok().and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
                })
            }));
        }

        Ok(json!({
            "path": path,
            "entries": entries,
            "count": entries.len()
        }))
    }

    pub async fn stat(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path))?;

        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        #[cfg(unix)]
        let permissions = format!("{:o}", metadata.permissions().mode() & 0o777);

        #[cfg(not(unix))]
        let permissions = if metadata.permissions().readonly() {
            "readonly".to_string()
        } else {
            "read-write".to_string()
        };

        Ok(json!({
            "path": path,
            "type": if metadata.is_file() { "file" } else if metadata.is_dir() { "dir" } else { "other" },
            "size": metadata.len(),
            "permissions": permissions,
            "readonly": metadata.permissions().readonly(),
            "created": metadata.created().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
            }),
            "modified": metadata.modified().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
            }),
            "accessed": metadata.accessed().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
            })
        }))
    }

    pub async fn permissions(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;

        if let Some(mode_str) = args["mode"].as_str() {
            // Set permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = u32::from_str_radix(mode_str, 8)
                    .with_context(|| format!("Invalid permission mode: {}", mode_str))?;
                let permissions = std::fs::Permissions::from_mode(mode);
                fs::set_permissions(path, permissions)
                    .with_context(|| format!("Failed to set permissions for: {}", path))?;

                Ok(json!({
                    "success": true,
                    "path": path,
                    "mode": mode_str
                }))
            }

            #[cfg(not(unix))]
            {
                anyhow::bail!("Setting permissions is only supported on Unix systems");
            }
        } else {
            // Get permissions
            let metadata = fs::metadata(path)?;

            #[cfg(unix)]
            use std::os::unix::fs::PermissionsExt;

            #[cfg(unix)]
            let mode = format!("{:o}", metadata.permissions().mode() & 0o777);

            #[cfg(not(unix))]
            let mode = if metadata.permissions().readonly() {
                "readonly"
            } else {
                "read-write"
            };

            Ok(json!({
                "path": path,
                "mode": mode,
                "readonly": metadata.permissions().readonly()
            }))
        }
    }

    pub async fn watch(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let duration = args["duration"].as_u64().unwrap_or(60);

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::Recursive)?;

        let mut events = Vec::new();
        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < duration {
            if let Ok(Ok(event)) = rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(json!({
                    "kind": format!("{:?}", event.kind),
                    "paths": event.paths.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>()
                }));
            }
        }

        Ok(json!({
            "path": path,
            "duration": duration,
            "events": events,
            "event_count": events.len()
        }))
    }

    pub async fn snapshot(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let max_snapshots = args["max_snapshots"].as_u64().unwrap_or(10) as usize;

        let path_obj = Path::new(path);
        if !path_obj.exists() {
            anyhow::bail!("Path does not exist: {}", path);
        }

        // Create snapshot directory
        let snapshot_dir = path_obj.parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".snapshots")
            .join(path_obj.file_name().unwrap_or_else(|| path_obj.as_os_str()));

        fs::create_dir_all(&snapshot_dir)?;

        // Create timestamp
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let snapshot_name = format!("snapshot_{}", timestamp);
        let snapshot_path = snapshot_dir.join(&snapshot_name);

        // Copy the file/directory
        if path_obj.is_file() {
            fs::copy(path, &snapshot_path)?;
        } else {
            copy_dir_all(path, &snapshot_path)?;
        }

        // Store snapshot info
        let mut snapshots = self.snapshots.lock().unwrap();
        let key = path.to_string();
        let snapshot_list = snapshots.entry(key.clone()).or_insert_with(Vec::new);

        snapshot_list.push(SnapshotInfo {
            timestamp: timestamp.clone(),
            path: snapshot_path.clone(),
            compressed: false,
        });

        // Manage snapshots (compress old ones, delete oldest)
        if snapshot_list.len() > max_snapshots {
            // Compress older snapshots
            for snapshot in snapshot_list.iter_mut().rev().skip(3) {
                if !snapshot.compressed {
                    // TODO: Implement compression
                    snapshot.compressed = true;
                }
            }

            // Remove oldest snapshots
            while snapshot_list.len() > max_snapshots {
                if let Some(oldest) = snapshot_list.first() {
                    if oldest.path.exists() {
                        if oldest.path.is_file() {
                            fs::remove_file(&oldest.path)?;
                        } else {
                            fs::remove_dir_all(&oldest.path)?;
                        }
                    }
                }
                snapshot_list.remove(0);
            }
        }

        Ok(json!({
            "success": true,
            "path": path,
            "snapshot": snapshot_path,
            "timestamp": timestamp,
            "total_snapshots": snapshot_list.len(),
            "max_snapshots": max_snapshots
        }))
    }

    pub async fn tree(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let max_depth = args["max_depth"].as_u64().unwrap_or(4) as usize;
        let show_hidden = args["show_hidden"].as_bool().unwrap_or(false);
        let show_size = args["show_size"].as_bool().unwrap_or(false);
        let dirs_only = args["dirs_only"].as_bool().unwrap_or(false);
        let pattern = args["pattern"].as_str();

        let root = Path::new(path);
        if !root.exists() {
            anyhow::bail!("Path does not exist: {}", path);
        }

        let mut output = String::new();
        let mut file_count = 0usize;
        let mut dir_count = 0usize;

        output.push_str(&format!("{}\n", root.display()));
        build_tree(root, "", max_depth, 0, show_hidden, show_size, dirs_only, pattern, &mut output, &mut file_count, &mut dir_count)?;

        Ok(json!({
            "tree": output,
            "path": path,
            "directories": dir_count,
            "files": file_count,
            "max_depth": max_depth
        }))
    }

    pub async fn grep(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let pattern = args["pattern"].as_str().context("Missing 'pattern' parameter")?;
        let case_insensitive = args["case_insensitive"].as_bool().unwrap_or(false);
        let max_results = args["max_results"].as_u64().unwrap_or(200) as usize;
        let context_lines = args["context_lines"].as_u64().unwrap_or(0) as usize;
        let file_pattern = args["file_pattern"].as_str();

        let mut regex_pattern = String::new();
        if case_insensitive {
            regex_pattern.push_str("(?i)");
        }
        regex_pattern.push_str(pattern);

        let re = Regex::new(&regex_pattern)
            .with_context(|| format!("Invalid regex pattern: {}", pattern))?;

        let root = Path::new(path);
        let mut matches = Vec::new();

        let files: Vec<PathBuf> = if root.is_file() {
            vec![root.to_path_buf()]
        } else {
            WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .filter(|e| {
                    if let Some(fp) = file_pattern {
                        let name = e.file_name().to_string_lossy();
                        glob_match(fp, &name)
                    } else {
                        true
                    }
                })
                .map(|e| e.path().to_path_buf())
                .collect()
        };

        'outer: for file_path in &files {
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue, // skip binary/unreadable files
            };

            let lines: Vec<&str> = content.lines().collect();

            for (line_num, line) in lines.iter().enumerate() {
                if re.is_match(line) {
                    if matches.len() >= max_results {
                        break 'outer;
                    }

                    let mut context_before = Vec::new();
                    let mut context_after = Vec::new();

                    if context_lines > 0 {
                        let start = line_num.saturating_sub(context_lines);
                        for i in start..line_num {
                            context_before.push(lines[i]);
                        }
                        let end = (line_num + 1 + context_lines).min(lines.len());
                        for i in (line_num + 1)..end {
                            context_after.push(lines[i]);
                        }
                    }

                    let mut entry = json!({
                        "file": file_path.to_string_lossy(),
                        "line": line_num + 1,
                        "content": line
                    });

                    if context_lines > 0 {
                        entry["context_before"] = json!(context_before);
                        entry["context_after"] = json!(context_after);
                    }

                    matches.push(entry);
                }
            }
        }

        let truncated = matches.len() >= max_results;

        Ok(json!({
            "matches": matches,
            "count": matches.len(),
            "files_searched": files.len(),
            "pattern": pattern,
            "truncated": truncated
        }))
    }

    pub async fn tail(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let n = args["lines"].as_u64().unwrap_or(20) as usize;

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path))?;

        let all_lines: Vec<&str> = content.lines().collect();
        let total = all_lines.len();
        let start = total.saturating_sub(n);
        let tail_lines = &all_lines[start..];

        Ok(json!({
            "path": path,
            "content": tail_lines.join("\n"),
            "lines_returned": tail_lines.len(),
            "total_lines": total,
            "from_line": start + 1
        }))
    }

    pub async fn replace(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let find = args["find"].as_str().context("Missing 'find' parameter")?;
        let replace_with = args["replace"].as_str().context("Missing 'replace' parameter")?;
        let use_regex = args["regex"].as_bool().unwrap_or(false);
        let dry_run = args["dry_run"].as_bool().unwrap_or(false);
        let file_pattern = args["file_pattern"].as_str();

        let root = Path::new(path);

        let files: Vec<PathBuf> = if root.is_file() {
            vec![root.to_path_buf()]
        } else if root.is_dir() {
            let fp = file_pattern.context("'file_pattern' is required when path is a directory")?;
            WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .filter(|e| glob_match(fp, &e.file_name().to_string_lossy()))
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            anyhow::bail!("Path does not exist: {}", path);
        };

        let re = if use_regex {
            Some(Regex::new(find).with_context(|| format!("Invalid regex: {}", find))?)
        } else {
            None
        };

        let mut results = Vec::new();
        let mut total_replacements = 0usize;

        for file_path in &files {
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let (new_content, count) = if let Some(ref re) = re {
                let count = re.find_iter(&content).count();
                let replaced = re.replace_all(&content, replace_with).to_string();
                (replaced, count)
            } else {
                let count = content.matches(find).count();
                let replaced = content.replace(find, replace_with);
                (replaced, count)
            };

            if count > 0 {
                if !dry_run {
                    fs::write(file_path, &new_content)
                        .with_context(|| format!("Failed to write: {}", file_path.display()))?;
                }
                total_replacements += count;
                results.push(json!({
                    "file": file_path.to_string_lossy(),
                    "replacements": count
                }));
            }
        }

        Ok(json!({
            "success": true,
            "dry_run": dry_run,
            "files_modified": results.len(),
            "total_replacements": total_replacements,
            "files_searched": files.len(),
            "details": results
        }))
    }
}

fn build_tree(
    dir: &Path,
    prefix: &str,
    max_depth: usize,
    current_depth: usize,
    show_hidden: bool,
    show_size: bool,
    dirs_only: bool,
    pattern: Option<&str>,
    output: &mut String,
    file_count: &mut usize,
    dir_count: &mut usize,
) -> Result<()> {
    if current_depth >= max_depth {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();

    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    // Filter hidden files
    if !show_hidden {
        entries.retain(|e| {
            !e.file_name().to_string_lossy().starts_with('.')
        });
    }

    // Filter by pattern
    if let Some(pat) = pattern {
        entries.retain(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.path().is_dir();
            is_dir || name.contains(pat) || glob_match(pat, &name)
        });
    }

    // Filter dirs_only
    if dirs_only {
        entries.retain(|e| e.path().is_dir());
    }

    let count = entries.len();

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let is_dir = path.is_dir();

        let size_str = if show_size && !is_dir {
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            format!(" ({})", format_size(size))
        } else {
            String::new()
        };

        let dir_marker = if is_dir { "/" } else { "" };

        output.push_str(&format!("{}{}{}{}{}\n", prefix, connector, name, dir_marker, size_str));

        if is_dir {
            *dir_count += 1;
            let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            build_tree(&path, &new_prefix, max_depth, current_depth + 1, show_hidden, show_size, dirs_only, pattern, output, file_count, dir_count)?;
        } else {
            *file_count += 1;
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1}G", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1}M", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

/// Simple glob matching: supports * (any chars) and ? (single char)
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_recursive(&pattern.chars().collect::<Vec<_>>(), &text.chars().collect::<Vec<_>>(), 0, 0)
}

fn glob_match_recursive(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
    if pi == pattern.len() && ti == text.len() {
        return true;
    }
    if pi == pattern.len() {
        return false;
    }

    match pattern[pi] {
        '*' => {
            // * matches zero or more characters
            for i in ti..=text.len() {
                if glob_match_recursive(pattern, text, pi + 1, i) {
                    return true;
                }
            }
            false
        }
        '?' => {
            if ti < text.len() {
                glob_match_recursive(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
        c => {
            if ti < text.len() && (c == text[ti] || c.to_lowercase().next() == text[ti].to_lowercase().next()) {
                glob_match_recursive(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
    }
}

// Helper function to copy directories recursively
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}
