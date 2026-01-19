use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use git2::{Repository, StatusOptions, DiffOptions, BranchType, ObjectType};
use std::path::Path;

pub struct GitModule;

impl GitModule {
    pub fn new() -> Self {
        Self
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "git_status",
                "description": "Get git repository status",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        }
                    }
                }
            }),
            json!({
                "name": "git_diff",
                "description": "Get diff of changes in repository",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        },
                        "staged": {
                            "type": "boolean",
                            "description": "Show staged changes only (default: false)"
                        },
                        "file": {
                            "type": "string",
                            "description": "Specific file to diff"
                        }
                    }
                }
            }),
            json!({
                "name": "git_commit",
                "description": "Create a git commit",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        },
                        "message": {
                            "type": "string",
                            "description": "Commit message"
                        },
                        "author_name": {
                            "type": "string",
                            "description": "Author name"
                        },
                        "author_email": {
                            "type": "string",
                            "description": "Author email"
                        }
                    },
                    "required": ["message"]
                }
            }),
            json!({
                "name": "git_branch",
                "description": "List, create, or delete branches",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["list", "create", "delete"],
                            "description": "Action to perform (default: list)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Branch name (for create/delete)"
                        }
                    }
                }
            }),
            json!({
                "name": "git_checkout",
                "description": "Checkout a branch or commit",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        },
                        "target": {
                            "type": "string",
                            "description": "Branch name or commit hash to checkout"
                        },
                        "create": {
                            "type": "boolean",
                            "description": "Create branch if it doesn't exist (default: false)"
                        }
                    },
                    "required": ["target"]
                }
            }),
            json!({
                "name": "git_blame",
                "description": "Show what revision and author last modified each line of a file",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        },
                        "file": {
                            "type": "string",
                            "description": "File to blame"
                        }
                    },
                    "required": ["file"]
                }
            }),
            json!({
                "name": "git_log",
                "description": "Show commit logs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Number of commits to show (default: 10)"
                        },
                        "file": {
                            "type": "string",
                            "description": "Show commits for specific file"
                        }
                    }
                }
            }),
            json!({
                "name": "git_tag",
                "description": "List, create, or delete tags",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (default: current directory)"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["list", "create", "delete"],
                            "description": "Action to perform (default: list)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Tag name (for create/delete)"
                        },
                        "message": {
                            "type": "string",
                            "description": "Tag message (for annotated tags)"
                        }
                    }
                }
            }),
        ]
    }

    pub async fn status(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let repo = Repository::open(path)?;

        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        let statuses = repo.statuses(Some(&mut opts))?;

        let mut result = json!({
            "staged": [],
            "unstaged": [],
            "untracked": []
        });

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("");
            let status = entry.status();

            if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
                result["staged"].as_array_mut().unwrap().push(json!({
                    "path": path,
                    "status": format!("{:?}", status)
                }));
            }

            if status.is_wt_modified() || status.is_wt_deleted() {
                result["unstaged"].as_array_mut().unwrap().push(json!({
                    "path": path,
                    "status": format!("{:?}", status)
                }));
            }

            if status.is_wt_new() {
                result["untracked"].as_array_mut().unwrap().push(json!({
                    "path": path
                }));
            }
        }

        // Get current branch
        let head = repo.head()?;
        let branch_name = head.shorthand().unwrap_or("HEAD");

        result["branch"] = json!(branch_name);
        result["is_detached"] = json!(!head.is_branch());

        Ok(result)
    }

    pub async fn diff(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let staged = args["staged"].as_bool().unwrap_or(false);
        let file_filter = args["file"].as_str();

        let repo = Repository::open(path)?;

        let mut diff_opts = DiffOptions::new();
        if let Some(file) = file_filter {
            diff_opts.pathspec(file);
        }

        let diff = if staged {
            // Diff between HEAD and index (staged changes)
            let head = repo.head()?.peel_to_tree()?;
            let _index = repo.index()?;
            let index_tree = repo.find_tree(repo.index()?.write_tree()?)?;
            repo.diff_tree_to_tree(Some(&head), Some(&index_tree), Some(&mut diff_opts))?
        } else {
            // Diff between index and working directory (unstaged changes)
            repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
        };

        let mut patches = Vec::new();

        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            patches.push(json!({
                "origin": format!("{}", line.origin()),
                "content": String::from_utf8_lossy(line.content())
            }));
            true
        })?;

        let stats = diff.stats()?;

        Ok(json!({
            "staged": staged,
            "files_changed": stats.files_changed(),
            "insertions": stats.insertions(),
            "deletions": stats.deletions(),
            "patches": patches
        }))
    }

    pub async fn commit(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let message = args["message"].as_str().context("Missing 'message' parameter")?;

        let repo = Repository::open(path)?;

        // Get signature
        let signature = if let (Some(name), Some(email)) = (args["author_name"].as_str(), args["author_email"].as_str()) {
            git2::Signature::now(name, email)?
        } else {
            repo.signature()?
        };

        // Get current index
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get parent commit
        let parent_commit = repo.head()?.peel_to_commit()?;

        // Create commit
        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(json!({
            "success": true,
            "commit_id": commit_id.to_string(),
            "message": message,
            "author": signature.name().unwrap_or(""),
            "email": signature.email().unwrap_or("")
        }))
    }

    pub async fn branch(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let action = args["action"].as_str().unwrap_or("list");

        let repo = Repository::open(path)?;

        match action {
            "list" => {
                let mut branches = Vec::new();

                for branch in repo.branches(None)? {
                    let (branch, _) = branch?;
                    let name = branch.name()?.unwrap_or("");
                    let is_head = branch.is_head();

                    branches.push(json!({
                        "name": name,
                        "is_current": is_head
                    }));
                }

                Ok(json!({
                    "branches": branches,
                    "count": branches.len()
                }))
            }
            "create" => {
                let name = args["name"].as_str().context("Missing 'name' parameter")?;
                let head = repo.head()?;
                let commit = head.peel_to_commit()?;

                repo.branch(name, &commit, false)?;

                Ok(json!({
                    "success": true,
                    "branch": name,
                    "action": "created"
                }))
            }
            "delete" => {
                let name = args["name"].as_str().context("Missing 'name' parameter")?;
                let mut branch = repo.find_branch(name, BranchType::Local)?;

                branch.delete()?;

                Ok(json!({
                    "success": true,
                    "branch": name,
                    "action": "deleted"
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    pub async fn checkout(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let target = args["target"].as_str().context("Missing 'target' parameter")?;
        let create = args["create"].as_bool().unwrap_or(false);

        let repo = Repository::open(path)?;

        // Try to find existing branch
        let branch_exists = repo.find_branch(target, BranchType::Local).is_ok();

        if !branch_exists && create {
            // Create new branch
            let head = repo.head()?;
            let commit = head.peel_to_commit()?;
            repo.branch(target, &commit, false)?;
        }

        // Checkout the branch
        let obj = repo.revparse_single(target)?;

        repo.checkout_tree(&obj, None)?;
        repo.set_head(&format!("refs/heads/{}", target))?;

        Ok(json!({
            "success": true,
            "target": target,
            "created": !branch_exists && create
        }))
    }

    pub async fn blame(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let file = args["file"].as_str().context("Missing 'file' parameter")?;

        let repo = Repository::open(path)?;
        let blame = repo.blame_file(Path::new(file), None)?;

        let mut lines = Vec::new();

        for hunk in blame.iter() {
            let commit = repo.find_commit(hunk.final_commit_id())?;

            lines.push(json!({
                "line_start": hunk.final_start_line(),
                "line_count": hunk.lines_in_hunk(),
                "commit": hunk.final_commit_id().to_string(),
                "author": commit.author().name().unwrap_or(""),
                "email": commit.author().email().unwrap_or(""),
                "timestamp": commit.time().seconds(),
                "message": commit.summary().unwrap_or("")
            }));
        }

        Ok(json!({
            "file": file,
            "lines": lines,
            "total_hunks": lines.len()
        }))
    }

    pub async fn log(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        let file_filter = args["file"].as_str();

        let repo = Repository::open(path)?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();

        for (idx, oid) in revwalk.enumerate() {
            if idx >= limit {
                break;
            }

            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            // If file filter is specified, check if commit affects the file
            if let Some(file) = file_filter {
                let tree = commit.tree()?;
                if tree.get_path(Path::new(file)).is_err() {
                    continue;
                }
            }

            commits.push(json!({
                "id": oid.to_string(),
                "short_id": format!("{:.7}", oid),
                "author": commit.author().name().unwrap_or(""),
                "email": commit.author().email().unwrap_or(""),
                "timestamp": commit.time().seconds(),
                "message": commit.message().unwrap_or(""),
                "summary": commit.summary().unwrap_or("")
            }));
        }

        Ok(json!({
            "commits": commits,
            "count": commits.len(),
            "limit": limit
        }))
    }

    pub async fn tag(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let action = args["action"].as_str().unwrap_or("list");

        let repo = Repository::open(path)?;

        match action {
            "list" => {
                let mut tags = Vec::new();

                for name in repo.tag_names(None)?.iter() {
                    if let Some(tag_name) = name {
                        tags.push(json!({
                            "name": tag_name
                        }));
                    }
                }

                Ok(json!({
                    "tags": tags,
                    "count": tags.len()
                }))
            }
            "create" => {
                let name = args["name"].as_str().context("Missing 'name' parameter")?;
                let message = args["message"].as_str();

                let head = repo.head()?;
                let target = head.peel(ObjectType::Commit)?;

                if let Some(msg) = message {
                    // Create annotated tag
                    let sig = repo.signature()?;
                    repo.tag(name, &target, &sig, msg, false)?;
                } else {
                    // Create lightweight tag
                    repo.tag_lightweight(name, &target, false)?;
                }

                Ok(json!({
                    "success": true,
                    "tag": name,
                    "action": "created",
                    "annotated": message.is_some()
                }))
            }
            "delete" => {
                let name = args["name"].as_str().context("Missing 'name' parameter")?;
                repo.tag_delete(name)?;

                Ok(json!({
                    "success": true,
                    "tag": name,
                    "action": "deleted"
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }
}
