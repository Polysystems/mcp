use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
struct ClipEntry {
    content: String,
    #[allow(dead_code)]
    tag: String,
    source: ClipSource,
    timestamp: DateTime<Utc>,
    line_count: usize,
    byte_size: usize,
}

#[derive(Clone, Debug)]
enum ClipSource {
    File {
        path: String,
        lines: Option<Vec<(usize, usize)>>,
    },
    Direct,
}

pub struct ClipboardModule {
    entries: Arc<Mutex<HashMap<String, ClipEntry>>>,
}

impl Default for ClipboardModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardModule {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "clip_copy_file",
                "description": "Copy text from a file into the session clipboard with a tag. Reads file content (optionally specific line ranges) and stores it for later paste operations. Saves tokens by avoiding redundant file reads.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to copy from"
                        },
                        "tag": {
                            "type": "string",
                            "description": "Tag name to store this content under (used to retrieve it later)"
                        },
                        "lines": {
                            "type": "array",
                            "description": "Optional line ranges to copy, e.g. [[1,10], [20,30]]. Copies entire file if omitted.",
                            "items": {
                                "type": "array",
                                "minItems": 2,
                                "maxItems": 2,
                                "items": { "type": "integer" }
                            }
                        }
                    },
                    "required": ["path", "tag"]
                }
            }),
            json!({
                "name": "clip_copy",
                "description": "Copy arbitrary text directly into the session clipboard with a tag. Use this to store computed/generated content or text from tool outputs for later reuse without re-generating.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Text content to store"
                        },
                        "tag": {
                            "type": "string",
                            "description": "Tag name to store this content under"
                        }
                    },
                    "required": ["content", "tag"]
                }
            }),
            json!({
                "name": "clip_paste_file",
                "description": "Paste tagged clipboard content into a file. Can overwrite the entire file, append, prepend, or replace specific line ranges.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tag": {
                            "type": "string",
                            "description": "Tag of the clipboard entry to paste"
                        },
                        "path": {
                            "type": "string",
                            "description": "Destination file path"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["overwrite", "append", "prepend", "lines"],
                            "description": "Paste mode (default: overwrite). 'lines' uses the lines parameter to replace specific ranges."
                        },
                        "lines": {
                            "type": "array",
                            "description": "Line ranges to replace in the destination file (for mode='lines'), e.g. [[5,10]].",
                            "items": {
                                "type": "array",
                                "minItems": 2,
                                "maxItems": 2,
                                "items": { "type": "integer" }
                            }
                        }
                    },
                    "required": ["tag", "path"]
                }
            }),
            json!({
                "name": "clip_paste",
                "description": "Retrieve tagged clipboard content. Returns the stored text so the agent can use it without re-reading the original source. Omit tag to list all entries.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tag": {
                            "type": "string",
                            "description": "Tag of the clipboard entry to retrieve. Omit to list all tags with metadata."
                        }
                    }
                }
            }),
            json!({
                "name": "clip_clear",
                "description": "Clear one or all clipboard entries from the session.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tag": {
                            "type": "string",
                            "description": "Tag to clear. Omit to clear all entries."
                        }
                    }
                }
            }),
        ]
    }

    pub async fn copy_file(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let tag = args["tag"].as_str().context("Missing 'tag' parameter")?;

        let full_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path))?;

        let mut line_ranges: Option<Vec<(usize, usize)>> = None;

        let content = if let Some(lines_array) = args.get("lines").and_then(|v| v.as_array()) {
            let all_lines: Vec<&str> = full_content.lines().collect();
            let mut selected_lines = Vec::new();
            let mut ranges = Vec::new();

            for range in lines_array {
                if let Some(range_arr) = range.as_array() {
                    if range_arr.len() == 2 {
                        let from = range_arr[0].as_u64().unwrap_or(1) as usize;
                        let to = range_arr[1].as_u64().unwrap_or(all_lines.len() as u64) as usize;
                        let from_idx = from.saturating_sub(1);
                        let to_idx = to.min(all_lines.len());

                        if from_idx < all_lines.len() && from_idx < to_idx {
                            selected_lines.extend_from_slice(&all_lines[from_idx..to_idx]);
                        }
                        ranges.push((from, to));
                    }
                }
            }

            line_ranges = Some(ranges);
            selected_lines.join("\n")
        } else {
            full_content
        };

        let line_count = content.lines().count();
        let byte_size = content.len();

        let entry = ClipEntry {
            content,
            tag: tag.to_string(),
            source: ClipSource::File {
                path: path.to_string(),
                lines: line_ranges.clone(),
            },
            timestamp: Utc::now(),
            line_count,
            byte_size,
        };

        let mut entries = self.entries.lock().unwrap();
        entries.insert(tag.to_string(), entry);

        Ok(json!({
            "success": true,
            "tag": tag,
            "source": path,
            "lines": line_ranges.map(|r| r.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>()),
            "line_count": line_count,
            "byte_size": byte_size,
            "total_entries": entries.len()
        }))
    }

    pub async fn copy(&self, args: Value) -> Result<Value> {
        let content = args["content"].as_str().context("Missing 'content' parameter")?;
        let tag = args["tag"].as_str().context("Missing 'tag' parameter")?;

        let line_count = content.lines().count();
        let byte_size = content.len();

        let entry = ClipEntry {
            content: content.to_string(),
            tag: tag.to_string(),
            source: ClipSource::Direct,
            timestamp: Utc::now(),
            line_count,
            byte_size,
        };

        let mut entries = self.entries.lock().unwrap();
        entries.insert(tag.to_string(), entry);

        Ok(json!({
            "success": true,
            "tag": tag,
            "source": "direct",
            "line_count": line_count,
            "byte_size": byte_size,
            "total_entries": entries.len()
        }))
    }

    pub async fn paste_file(&self, args: Value) -> Result<Value> {
        let tag = args["tag"].as_str().context("Missing 'tag' parameter")?;
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let mode = args["mode"].as_str().unwrap_or("overwrite");

        let entries = self.entries.lock().unwrap();
        let entry = entries.get(tag)
            .with_context(|| format!("No clipboard entry with tag '{}'", tag))?;
        let content = entry.content.clone();
        drop(entries);

        let final_content = match mode {
            "append" => {
                let existing = fs::read_to_string(path).unwrap_or_default();
                format!("{}{}", existing, content)
            }
            "prepend" => {
                let existing = fs::read_to_string(path).unwrap_or_default();
                format!("{}{}", content, existing)
            }
            "lines" => {
                let existing = fs::read_to_string(path).unwrap_or_default();
                let mut all_lines: Vec<String> = existing.lines().map(|s| s.to_string()).collect();
                let new_lines: Vec<&str> = content.lines().collect();
                let mut new_line_idx = 0;

                if let Some(lines_array) = args.get("lines").and_then(|v| v.as_array()) {
                    for range in lines_array {
                        if let Some(range_arr) = range.as_array() {
                            if range_arr.len() == 2 {
                                let from = range_arr[0].as_u64().unwrap_or(1) as usize;
                                let to = range_arr[1].as_u64().unwrap_or(all_lines.len() as u64) as usize;
                                let from_idx = from.saturating_sub(1);
                                let to_idx = to.min(all_lines.len());
                                let range_size = to_idx.saturating_sub(from_idx);

                                while all_lines.len() < to_idx {
                                    all_lines.push(String::new());
                                }

                                for i in 0..range_size {
                                    if new_line_idx < new_lines.len() {
                                        all_lines[from_idx + i] = new_lines[new_line_idx].to_string();
                                        new_line_idx += 1;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Mode 'lines' requires 'lines' parameter"));
                }

                all_lines.join("\n") + "\n"
            }
            _ => content.clone(), // overwrite
        };

        if let Some(parent) = std::path::Path::new(path).parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let bytes_written = final_content.len();
        fs::write(path, &final_content)
            .with_context(|| format!("Failed to write to file: {}", path))?;

        Ok(json!({
            "success": true,
            "tag": tag,
            "destination": path,
            "mode": mode,
            "bytes_written": bytes_written
        }))
    }

    pub async fn paste(&self, args: Value) -> Result<Value> {
        let entries = self.entries.lock().unwrap();

        if let Some(tag) = args["tag"].as_str() {
            if let Some(entry) = entries.get(tag) {
                let source = match &entry.source {
                    ClipSource::File { path, lines } => json!({
                        "type": "file",
                        "path": path,
                        "lines": lines.as_ref().map(|r| r.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>())
                    }),
                    ClipSource::Direct => json!({ "type": "direct" }),
                };

                Ok(json!({
                    "tag": tag,
                    "content": entry.content,
                    "source": source,
                    "line_count": entry.line_count,
                    "byte_size": entry.byte_size,
                    "timestamp": entry.timestamp.to_rfc3339(),
                    "found": true
                }))
            } else {
                Ok(json!({
                    "tag": tag,
                    "found": false,
                    "error": format!("No entry with tag '{}'", tag)
                }))
            }
        } else {
            let listing: Vec<Value> = entries.iter().map(|(tag, entry)| {
                let source_type = match &entry.source {
                    ClipSource::File { path, .. } => json!({"type": "file", "path": path}),
                    ClipSource::Direct => json!({"type": "direct"}),
                };
                json!({
                    "tag": tag,
                    "source": source_type,
                    "line_count": entry.line_count,
                    "byte_size": entry.byte_size,
                    "timestamp": entry.timestamp.to_rfc3339()
                })
            }).collect();

            Ok(json!({
                "entries": listing,
                "count": listing.len()
            }))
        }
    }

    pub async fn clear(&self, args: Value) -> Result<Value> {
        let mut entries = self.entries.lock().unwrap();

        if let Some(tag) = args["tag"].as_str() {
            let removed = entries.remove(tag).is_some();
            Ok(json!({
                "success": removed,
                "tag": tag,
                "removed": removed,
                "remaining": entries.len()
            }))
        } else {
            let count = entries.len();
            entries.clear();
            Ok(json!({
                "success": true,
                "cleared": count,
                "remaining": 0
            }))
        }
    }
}
