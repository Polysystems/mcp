use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use gitent_core::{Storage, Session, Change, ChangeType, Commit};
use uuid::Uuid;

pub struct GitentModule {
    state: Arc<Mutex<Option<GitentState>>>,
}

struct GitentState {
    storage: Storage,
    session: Session,
    db_path: PathBuf,
}

impl GitentModule {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "gitent_init",
                "description": "Initialize or connect to a gitent session for tracking file changes",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to track (defaults to current directory)"
                        },
                        "db_path": {
                            "type": "string",
                            "description": "Database path (defaults to .gitent/gitent.db or GITENT_DB_PATH env var)"
                        },
                        "force_new": {
                            "type": "boolean",
                            "description": "Force create new session even if active one exists (default: false)"
                        }
                    }
                }
            }),
            json!({
                "name": "gitent_status",
                "description": "View current session status and uncommitted changes",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "verbose": {
                            "type": "boolean",
                            "description": "Show detailed information about each change (default: false)"
                        }
                    }
                }
            }),
            json!({
                "name": "gitent_track",
                "description": "Manually track a file change (create, modify, delete, rename)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path relative to session root"
                        },
                        "change_type": {
                            "type": "string",
                            "enum": ["create", "modify", "delete", "rename"],
                            "description": "Type of change being tracked"
                        },
                        "content": {
                            "type": "string",
                            "description": "File content (for create/modify operations)"
                        },
                        "old_path": {
                            "type": "string",
                            "description": "Previous path (required for rename operations)"
                        },
                        "agent_id": {
                            "type": "string",
                            "description": "Agent identifier (default: poly-mcp)"
                        }
                    },
                    "required": ["path", "change_type"]
                }
            }),
            json!({
                "name": "gitent_commit",
                "description": "Commit tracked changes with a message",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Commit message describing the changes"
                        },
                        "agent_id": {
                            "type": "string",
                            "description": "Agent identifier (default: poly-mcp)"
                        },
                        "change_ids": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Specific change IDs to commit (default: all uncommitted changes)"
                        }
                    },
                    "required": ["message"]
                }
            }),
            json!({
                "name": "gitent_log",
                "description": "View commit history for the current session",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of commits to show (default: 10)"
                        },
                        "verbose": {
                            "type": "boolean",
                            "description": "Show detailed file information for each commit (default: false)"
                        }
                    }
                }
            }),
            json!({
                "name": "gitent_diff",
                "description": "View differences for commits or uncommitted changes",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "commit_id": {
                            "type": "string",
                            "description": "Commit ID to show diff for (omit for uncommitted changes)"
                        },
                        "file": {
                            "type": "string",
                            "description": "Filter diff to specific file path"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["unified", "structured"],
                            "description": "Diff output format (default: unified)"
                        }
                    }
                }
            }),
            json!({
                "name": "gitent_rollback",
                "description": "Rollback to a previous commit state (preview mode by default)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "commit_id": {
                            "type": "string",
                            "description": "Commit ID to rollback to"
                        },
                        "execute": {
                            "type": "boolean",
                            "description": "Actually perform the rollback (default: false - preview only)"
                        }
                    },
                    "required": ["commit_id"]
                }
            }),
        ]
    }

    pub async fn init(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let db_path_arg = args["db_path"].as_str();
        let force_new = args["force_new"].as_bool().unwrap_or(false);

        let root_path = PathBuf::from(path);
        let db_path = Self::get_db_path(db_path_arg);

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        // Create or open storage
        let storage = Storage::new(&db_path)
            .context("Failed to open gitent database")?;

        // Try to get active session or create new one
        let session = if force_new {
            let new_session = Session::new(root_path.clone());
            storage.create_session(&new_session)
                .context("Failed to create new session")?;
            new_session
        } else {
            match storage.get_active_session() {
                Ok(session) => session,
                Err(_) => {
                    let new_session = Session::new(root_path.clone());
                    storage.create_session(&new_session)
                        .context("Failed to create new session")?;
                    new_session
                }
            }
        };

        // Update module state
        let mut state_guard = self.state.lock().unwrap();
        *state_guard = Some(GitentState {
            storage,
            session: session.clone(),
            db_path: db_path.clone(),
        });

        Ok(json!({
            "success": true,
            "session_id": session.id.to_string(),
            "root_path": session.root_path.to_string_lossy(),
            "started": session.started.to_rfc3339(),
            "db_path": db_path.to_string_lossy(),
            "active": session.active
        }))
    }

    pub async fn status(&self, args: Value) -> Result<Value> {
        let verbose = args["verbose"].as_bool().unwrap_or(false);

        let state_guard = self.state.lock().unwrap();
        let state = Self::ensure_session(&state_guard)?;

        let uncommitted = state.storage.get_uncommitted_changes(&state.session.id)?;

        let changes_info: Vec<Value> = uncommitted.iter().map(|change| {
            if verbose {
                json!({
                    "id": change.id.to_string(),
                    "type": change.change_type.as_str(),
                    "path": change.path.to_string_lossy(),
                    "timestamp": change.timestamp.to_rfc3339(),
                    "agent_id": change.agent_id,
                    "old_path": change.old_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    "has_content": change.content_after.is_some()
                })
            } else {
                json!({
                    "type": change.change_type.as_str(),
                    "path": change.path.to_string_lossy()
                })
            }
        }).collect();

        Ok(json!({
            "session_id": state.session.id.to_string(),
            "root_path": state.session.root_path.to_string_lossy(),
            "active": state.session.active,
            "uncommitted_count": uncommitted.len(),
            "uncommitted_changes": changes_info
        }))
    }

    pub async fn track(&self, args: Value) -> Result<Value> {
        let state_guard = self.state.lock().unwrap();
        let state = Self::ensure_session(&state_guard)?;

        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let change_type_str = args["change_type"].as_str().context("Missing 'change_type' parameter")?;
        let change_type = ChangeType::parse(change_type_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid change_type: {}", change_type_str))?;

        let mut change = Change::new(change_type, PathBuf::from(path), state.session.id);

        // Set agent_id
        if let Some(agent_id) = args["agent_id"].as_str() {
            change = change.with_agent_id(agent_id.to_string());
        } else {
            change = change.with_agent_id("poly-mcp".to_string());
        }

        // Handle content for create/modify
        if matches!(change_type, ChangeType::Create | ChangeType::Modify) {
            if let Some(content) = args["content"].as_str() {
                change = change.with_content_after(content.as_bytes().to_vec());
            }
        }

        // Handle rename
        if change_type == ChangeType::Rename {
            if let Some(old_path) = args["old_path"].as_str() {
                change = change.with_old_path(PathBuf::from(old_path));
            } else {
                return Err(anyhow::anyhow!("'old_path' is required for rename operations"));
            }
        }

        state.storage.create_change(&change)?;

        Ok(json!({
            "success": true,
            "change_id": change.id.to_string(),
            "change_type": change.change_type.as_str(),
            "path": change.path.to_string_lossy(),
            "timestamp": change.timestamp.to_rfc3339()
        }))
    }

    pub async fn commit(&self, args: Value) -> Result<Value> {
        let state_guard = self.state.lock().unwrap();
        let state = Self::ensure_session(&state_guard)?;

        let message = args["message"].as_str().context("Missing 'message' parameter")?;
        let agent_id = args["agent_id"].as_str().unwrap_or("poly-mcp");

        // Get changes to commit
        let change_ids: Vec<Uuid> = if let Some(ids_array) = args["change_ids"].as_array() {
            // Parse specific change IDs
            ids_array.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect()
        } else {
            // Get all uncommitted changes
            let uncommitted = state.storage.get_uncommitted_changes(&state.session.id)?;
            uncommitted.iter().map(|c| c.id).collect()
        };

        if change_ids.is_empty() {
            return Err(anyhow::anyhow!("No changes to commit"));
        }

        // Get latest commit to set parent
        let commits = state.storage.get_commits_for_session(&state.session.id)?;
        let parent = commits.first().map(|info| info.commit.id);

        // Create commit
        let mut commit = Commit::new(
            message.to_string(),
            agent_id.to_string(),
            change_ids.clone(),
            state.session.id
        );

        if let Some(parent_id) = parent {
            commit = commit.with_parent(parent_id);
        }

        state.storage.create_commit(&commit)?;

        Ok(json!({
            "success": true,
            "commit_id": commit.id.to_string(),
            "message": commit.message,
            "agent_id": commit.agent_id,
            "timestamp": commit.timestamp.to_rfc3339(),
            "change_count": change_ids.len(),
            "parent": commit.parent.map(|p| p.to_string())
        }))
    }

    pub async fn log(&self, args: Value) -> Result<Value> {
        let state_guard = self.state.lock().unwrap();
        let state = Self::ensure_session(&state_guard)?;

        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        let verbose = args["verbose"].as_bool().unwrap_or(false);

        let commits = state.storage.get_commits_for_session(&state.session.id)?;
        let commits_to_show = commits.iter().take(limit);

        let commits_info: Vec<Value> = commits_to_show.map(|info| {
            if verbose {
                json!({
                    "commit_id": info.commit.id.to_string(),
                    "message": info.commit.message,
                    "agent_id": info.commit.agent_id,
                    "timestamp": info.commit.timestamp.to_rfc3339(),
                    "parent": info.commit.parent.map(|p| p.to_string()),
                    "change_count": info.change_count,
                    "files": info.files_affected.iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<String>>()
                })
            } else {
                json!({
                    "commit_id": info.commit.id.to_string(),
                    "message": info.commit.message,
                    "timestamp": info.commit.timestamp.to_rfc3339(),
                    "change_count": info.change_count
                })
            }
        }).collect();

        Ok(json!({
            "session_id": state.session.id.to_string(),
            "total_commits": commits.len(),
            "showing": commits_info.len(),
            "commits": commits_info
        }))
    }

    pub async fn diff(&self, args: Value) -> Result<Value> {
        let state_guard = self.state.lock().unwrap();
        let state = Self::ensure_session(&state_guard)?;

        let format = args["format"].as_str().unwrap_or("unified");
        let file_filter = args["file"].as_str();

        let changes = if let Some(commit_id_str) = args["commit_id"].as_str() {
            // Get changes from specific commit
            let commit_id = Uuid::parse_str(commit_id_str)
                .context("Invalid commit_id")?;
            let commit = state.storage.get_commit(&commit_id)?;

            commit.changes.iter()
                .filter_map(|id| state.storage.get_change(id).ok())
                .collect()
        } else {
            // Get uncommitted changes
            state.storage.get_uncommitted_changes(&state.session.id)?
        };

        // Apply file filter if specified
        let filtered_changes: Vec<_> = if let Some(filter) = file_filter {
            changes.into_iter()
                .filter(|c| c.path.to_string_lossy().contains(filter))
                .collect()
        } else {
            changes
        };

        let diffs: Vec<Value> = filtered_changes.iter().map(|change| {
            let before_content = change.content_before.as_ref()
                .and_then(|c| String::from_utf8(c.clone()).ok())
                .unwrap_or_default();
            let after_content = change.content_after.as_ref()
                .and_then(|c| String::from_utf8(c.clone()).ok())
                .unwrap_or_default();

            if format == "structured" {
                json!({
                    "path": change.path.to_string_lossy(),
                    "type": change.change_type.as_str(),
                    "old_path": change.old_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    "content_before": before_content,
                    "content_after": after_content,
                    "hash_before": change.content_hash_before,
                    "hash_after": change.content_hash_after
                })
            } else {
                // Unified diff format
                let diff_text = Self::generate_unified_diff(
                    &before_content,
                    &after_content,
                    &change.path.to_string_lossy(),
                    change.change_type
                );
                json!({
                    "path": change.path.to_string_lossy(),
                    "type": change.change_type.as_str(),
                    "diff": diff_text
                })
            }
        }).collect();

        Ok(json!({
            "format": format,
            "change_count": diffs.len(),
            "diffs": diffs
        }))
    }

    pub async fn rollback(&self, args: Value) -> Result<Value> {
        let state_guard = self.state.lock().unwrap();
        let state = Self::ensure_session(&state_guard)?;

        let commit_id_str = args["commit_id"].as_str().context("Missing 'commit_id' parameter")?;
        let execute = args["execute"].as_bool().unwrap_or(false);

        let commit_id = Uuid::parse_str(commit_id_str)
            .context("Invalid commit_id")?;
        let commit = state.storage.get_commit(&commit_id)?;

        // Get all changes in this commit
        let changes: Vec<_> = commit.changes.iter()
            .filter_map(|id| state.storage.get_change(id).ok())
            .collect();

        if !execute {
            // Preview mode - show what would be restored
            let preview: Vec<Value> = changes.iter().map(|change| {
                json!({
                    "path": change.path.to_string_lossy(),
                    "type": change.change_type.as_str(),
                    "action": match change.change_type {
                        ChangeType::Create => "would restore file",
                        ChangeType::Modify => "would restore content",
                        ChangeType::Delete => "would restore deleted file",
                        ChangeType::Rename => "would restore original path"
                    },
                    "has_content": change.content_after.is_some()
                })
            }).collect();

            Ok(json!({
                "preview": true,
                "commit_id": commit_id.to_string(),
                "message": commit.message,
                "timestamp": commit.timestamp.to_rfc3339(),
                "change_count": changes.len(),
                "changes": preview,
                "warning": "Set execute: true to actually perform the rollback"
            }))
        } else {
            // Execute mode - actually restore files
            let mut restored = Vec::new();
            let mut errors = Vec::new();

            for change in changes {
                match Self::restore_change(&change, &state.session.root_path) {
                    Ok(msg) => restored.push(json!({
                        "path": change.path.to_string_lossy(),
                        "status": "restored",
                        "message": msg
                    })),
                    Err(e) => errors.push(json!({
                        "path": change.path.to_string_lossy(),
                        "status": "error",
                        "error": e.to_string()
                    }))
                }
            }

            Ok(json!({
                "executed": true,
                "commit_id": commit_id.to_string(),
                "restored_count": restored.len(),
                "error_count": errors.len(),
                "restored": restored,
                "errors": errors
            }))
        }
    }

    // Helper methods

    fn get_db_path(custom_path: Option<&str>) -> PathBuf {
        if let Some(path) = custom_path {
            PathBuf::from(path)
        } else if let Ok(env_path) = std::env::var("GITENT_DB_PATH") {
            PathBuf::from(env_path)
        } else {
            PathBuf::from(".gitent/gitent.db")
        }
    }

    fn ensure_session<'a>(state_guard: &'a std::sync::MutexGuard<'a, Option<GitentState>>) -> Result<&'a GitentState> {
        state_guard.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "No active gitent session. Call gitent_init first to start tracking."
            )
        })
    }

    fn generate_unified_diff(before: &str, after: &str, path: &str, change_type: ChangeType) -> String {
        match change_type {
            ChangeType::Create => {
                format!("--- /dev/null\n+++ {}\n@@ -0,0 +1,{} @@\n{}",
                    path,
                    after.lines().count(),
                    after.lines().map(|l| format!("+{}", l)).collect::<Vec<_>>().join("\n")
                )
            },
            ChangeType::Delete => {
                format!("--- {}\n+++ /dev/null\n@@ -1,{} +0,0 @@\n{}",
                    path,
                    before.lines().count(),
                    before.lines().map(|l| format!("-{}", l)).collect::<Vec<_>>().join("\n")
                )
            },
            ChangeType::Modify => {
                let before_lines: Vec<&str> = before.lines().collect();
                let after_lines: Vec<&str> = after.lines().collect();

                format!("--- {}\n+++ {}\n@@ -1,{} +1,{} @@\n{}\n{}",
                    path, path,
                    before_lines.len(), after_lines.len(),
                    before_lines.iter().map(|l| format!("-{}", l)).collect::<Vec<_>>().join("\n"),
                    after_lines.iter().map(|l| format!("+{}", l)).collect::<Vec<_>>().join("\n")
                )
            },
            ChangeType::Rename => {
                format!("rename from old_path\nrename to {}", path)
            }
        }
    }

    fn restore_change(change: &Change, root_path: &PathBuf) -> Result<String> {
        use std::fs;
        use std::io::Write;

        let full_path = root_path.join(&change.path);

        match change.change_type {
            ChangeType::Create | ChangeType::Modify => {
                if let Some(content) = &change.content_after {
                    // Create parent directories if needed
                    if let Some(parent) = full_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    let mut file = fs::File::create(&full_path)?;
                    file.write_all(content)?;

                    Ok(format!("Restored content to {:?}", full_path))
                } else {
                    Err(anyhow::anyhow!("No content available to restore"))
                }
            },
            ChangeType::Delete => {
                if let Some(content) = &change.content_before {
                    // Restore the deleted file
                    if let Some(parent) = full_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    let mut file = fs::File::create(&full_path)?;
                    file.write_all(content)?;

                    Ok(format!("Restored deleted file to {:?}", full_path))
                } else {
                    Err(anyhow::anyhow!("No content available to restore deleted file"))
                }
            },
            ChangeType::Rename => {
                if let Some(old_path) = &change.old_path {
                    let old_full_path = root_path.join(old_path);
                    fs::rename(&full_path, &old_full_path)?;
                    Ok(format!("Renamed {:?} back to {:?}", full_path, old_full_path))
                } else {
                    Err(anyhow::anyhow!("No old path available for rename operation"))
                }
            }
        }
    }
}
