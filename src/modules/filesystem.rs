use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use chrono::Local;
use notify::{Watcher, RecursiveMode};
use walkdir::WalkDir;
use std::sync::{Arc, Mutex};

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
                "description": "Read file contents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "fs_write",
                "description": "Write content to a file",
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
                "description": "Search for files and directories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Root path to search from"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (glob or regex)"
                        },
                        "type": {
                            "type": "string",
                            "enum": ["file", "dir", "all"],
                            "description": "Type to search for"
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
        ]
    }

    pub async fn read(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path))?;

        Ok(json!({
            "path": path,
            "content": content,
            "size": content.len()
        }))
    }

    pub async fn write(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let content = args["content"].as_str().context("Missing 'content' parameter")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write file: {}", path))?;

        Ok(json!({
            "success": true,
            "path": path,
            "bytes_written": content.len()
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

        let mut results = Vec::new();

        for entry in WalkDir::new(root_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Simple pattern matching (contains)
            if !path_str.contains(pattern) {
                continue;
            }

            // Type filtering
            match search_type {
                "file" if !path.is_file() => continue,
                "dir" if !path.is_dir() => continue,
                _ => {}
            }

            results.push(json!({
                "path": path_str,
                "type": if path.is_file() { "file" } else { "dir" }
            }));
        }

        Ok(json!({
            "results": results,
            "count": results.len()
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
