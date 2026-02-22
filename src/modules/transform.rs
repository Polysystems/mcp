use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::fs;
use std::io::Write as IoWrite;
use std::path::Path;

pub struct TransformModule;

impl TransformModule {
    pub fn new() -> Self {
        Self
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "transform_diff",
                "description": "Compare two texts or files and produce a diff. Useful for verifying changes or comparing versions without requiring a git repository.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "a": {
                            "type": "string",
                            "description": "First text content (or file path if from_file is true)"
                        },
                        "b": {
                            "type": "string",
                            "description": "Second text content (or file path if from_file is true)"
                        },
                        "from_file": {
                            "type": "boolean",
                            "description": "If true, treat a and b as file paths (default: false)"
                        },
                        "context_lines": {
                            "type": "number",
                            "description": "Number of context lines around changes (default: 3)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["unified", "inline", "stats"],
                            "description": "Output format (default: unified)"
                        }
                    },
                    "required": ["a", "b"]
                }
            }),
            json!({
                "name": "transform_encode",
                "description": "Encode or decode text using common encoding schemes (base64, URL, hex, HTML entities).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to encode or decode"
                        },
                        "encoding": {
                            "type": "string",
                            "enum": ["base64", "url", "hex", "html"],
                            "description": "Encoding scheme to use"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["encode", "decode"],
                            "description": "Whether to encode or decode (default: encode)"
                        }
                    },
                    "required": ["text", "encoding"]
                }
            }),
            json!({
                "name": "transform_hash",
                "description": "Generate cryptographic hashes of text or files (SHA256, SHA512, MD5, BLAKE3).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "input": {
                            "type": "string",
                            "description": "Text to hash, or file path if from_file is true"
                        },
                        "algorithm": {
                            "type": "string",
                            "enum": ["sha256", "sha512", "md5", "blake3"],
                            "description": "Hash algorithm (default: sha256)"
                        },
                        "from_file": {
                            "type": "boolean",
                            "description": "If true, hash the file at the given path (default: false)"
                        }
                    },
                    "required": ["input"]
                }
            }),
            json!({
                "name": "transform_regex",
                "description": "Perform regex operations on text: match, find all matches, replace, split, or extract capture groups.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to operate on"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Regular expression pattern"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["match", "find_all", "replace", "split", "extract"],
                            "description": "Operation to perform (default: find_all)"
                        },
                        "replacement": {
                            "type": "string",
                            "description": "Replacement string (for replace action, supports $1 capture groups)"
                        },
                        "flags": {
                            "type": "string",
                            "description": "Regex flags: 'i' case-insensitive, 'm' multiline, 's' dot-matches-newline"
                        }
                    },
                    "required": ["text", "pattern"]
                }
            }),
            json!({
                "name": "transform_json",
                "description": "Manipulate JSON: pretty-print, minify, validate, query with dot-notation, merge objects, list keys, or flatten nested structures.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "input": {
                            "type": "string",
                            "description": "JSON string to operate on"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["pretty", "minify", "validate", "query", "merge", "keys", "flatten"],
                            "description": "Operation to perform (default: pretty)"
                        },
                        "query": {
                            "type": "string",
                            "description": "Dot-notation path for query action (e.g. 'data.users[0].name')"
                        },
                        "merge_with": {
                            "type": "string",
                            "description": "Second JSON string to merge with (for merge action)"
                        }
                    },
                    "required": ["input"]
                }
            }),
            json!({
                "name": "transform_text",
                "description": "Perform common text transformations: case conversion, line sorting, deduplication, word/line counting, truncation, wrapping, and more.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to transform"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["uppercase", "lowercase", "title_case", "snake_case", "camel_case", "kebab_case", "sort_lines", "reverse_lines", "unique_lines", "trim_lines", "number_lines", "wrap", "truncate", "stats"],
                            "description": "Transformation to apply"
                        },
                        "width": {
                            "type": "number",
                            "description": "Line width for wrap, or max length for truncate (default: 80)"
                        }
                    },
                    "required": ["text", "action"]
                }
            }),
            json!({
                "name": "transform_archive",
                "description": "Create or extract archive files (zip, tar.gz). List archive contents without extracting.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["create", "extract", "list"],
                            "description": "Operation to perform"
                        },
                        "path": {
                            "type": "string",
                            "description": "Path to archive file (for extract/list) or output path (for create)"
                        },
                        "files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Files/directories to include (for create)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["zip", "tar_gz"],
                            "description": "Archive format (default: zip)"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Extraction destination directory (for extract, default: current directory)"
                        }
                    },
                    "required": ["action", "path"]
                }
            }),
        ]
    }

    // ── Diff ────────────────────────────────────────────────────────────

    pub async fn diff(&self, args: Value) -> Result<Value> {
        let a_raw = args["a"].as_str().context("Missing 'a' parameter")?;
        let b_raw = args["b"].as_str().context("Missing 'b' parameter")?;
        let from_file = args["from_file"].as_bool().unwrap_or(false);
        let format = args["format"].as_str().unwrap_or("unified");
        let context_lines = args["context_lines"].as_u64().unwrap_or(3) as usize;

        let (text_a, text_b, label_a, label_b) = if from_file {
            let a = fs::read_to_string(a_raw)
                .with_context(|| format!("Failed to read file: {}", a_raw))?;
            let b = fs::read_to_string(b_raw)
                .with_context(|| format!("Failed to read file: {}", b_raw))?;
            (a, b, a_raw.to_string(), b_raw.to_string())
        } else {
            (a_raw.to_string(), b_raw.to_string(), "a".to_string(), "b".to_string())
        };

        match format {
            "stats" => {
                let changeset = similar::TextDiff::from_lines(&text_a, &text_b);
                let mut additions = 0usize;
                let mut deletions = 0usize;
                let mut unchanged = 0usize;

                for change in changeset.iter_all_changes() {
                    match change.tag() {
                        similar::ChangeTag::Insert => additions += 1,
                        similar::ChangeTag::Delete => deletions += 1,
                        similar::ChangeTag::Equal => unchanged += 1,
                    }
                }

                Ok(json!({
                    "additions": additions,
                    "deletions": deletions,
                    "unchanged": unchanged,
                    "total_changes": additions + deletions,
                    "identical": additions == 0 && deletions == 0
                }))
            }
            "inline" => {
                let changeset = similar::TextDiff::from_lines(&text_a, &text_b);
                let mut lines = Vec::new();

                for change in changeset.iter_all_changes() {
                    let sign = match change.tag() {
                        similar::ChangeTag::Insert => "+",
                        similar::ChangeTag::Delete => "-",
                        similar::ChangeTag::Equal => " ",
                    };
                    lines.push(format!("{}{}", sign, change));
                }

                Ok(json!({
                    "diff": lines.join(""),
                    "format": "inline"
                }))
            }
            _ => {
                // unified
                let changeset = similar::TextDiff::from_lines(&text_a, &text_b);
                let unified = changeset
                    .unified_diff()
                    .context_radius(context_lines)
                    .header(&label_a, &label_b)
                    .to_string();

                Ok(json!({
                    "diff": unified,
                    "format": "unified",
                    "context_lines": context_lines
                }))
            }
        }
    }

    // ── Encode/Decode ───────────────────────────────────────────────────

    pub async fn encode(&self, args: Value) -> Result<Value> {
        let text = args["text"].as_str().context("Missing 'text' parameter")?;
        let encoding = args["encoding"].as_str().context("Missing 'encoding' parameter")?;
        let action = args["action"].as_str().unwrap_or("encode");

        let result = match (encoding, action) {
            ("base64", "encode") => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(text)
            }
            ("base64", "decode") => {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD.decode(text)
                    .context("Invalid base64 input")?;
                String::from_utf8(bytes).context("Decoded base64 is not valid UTF-8")?
            }
            ("url", "encode") => {
                urlencoding::encode(text).to_string()
            }
            ("url", "decode") => {
                urlencoding::decode(text)
                    .context("Invalid URL-encoded input")?
                    .to_string()
            }
            ("hex", "encode") => {
                text.as_bytes().iter().map(|b| format!("{:02x}", b)).collect()
            }
            ("hex", "decode") => {
                if text.len() % 2 != 0 {
                    return Err(anyhow::anyhow!("Hex string must have even length, got {}", text.len()));
                }
                let bytes: Result<Vec<u8>, _> = (0..text.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&text[i..i + 2], 16))
                    .collect();
                let bytes = bytes.context("Invalid hex input (non-hex characters found)")?;
                String::from_utf8(bytes).context("Decoded hex is not valid UTF-8")?
            }
            ("html", "encode") => {
                text.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\'', "&#39;")
            }
            ("html", "decode") => {
                text.replace("&amp;", "&")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&#39;", "'")
                    .replace("&#x27;", "'")
                    .replace("&apos;", "'")
            }
            _ => return Err(anyhow::anyhow!("Unsupported encoding/action: {}/{}", encoding, action)),
        };

        Ok(json!({
            "result": result,
            "encoding": encoding,
            "action": action,
            "input_length": text.len(),
            "output_length": result.len()
        }))
    }

    // ── Hash ────────────────────────────────────────────────────────────

    pub async fn hash(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing 'input' parameter")?;
        let algorithm = args["algorithm"].as_str().unwrap_or("sha256");
        let from_file = args["from_file"].as_bool().unwrap_or(false);

        let data = if from_file {
            fs::read(input)
                .with_context(|| format!("Failed to read file: {}", input))?
        } else {
            input.as_bytes().to_vec()
        };

        let hash = match algorithm {
            "sha256" => {
                use sha2::Digest;
                let result = sha2::Sha256::digest(&data);
                format!("{:x}", result)
            }
            "sha512" => {
                use sha2::Digest;
                let result = sha2::Sha512::digest(&data);
                format!("{:x}", result)
            }
            "md5" => {
                use md5::Digest;
                let result = md5::Md5::digest(&data);
                format!("{:x}", result)
            }
            "blake3" => {
                let result = blake3::hash(&data);
                result.to_hex().to_string()
            }
            _ => return Err(anyhow::anyhow!("Unsupported algorithm: {}", algorithm)),
        };

        Ok(json!({
            "hash": hash,
            "algorithm": algorithm,
            "input_size": data.len(),
            "from_file": from_file
        }))
    }

    // ── Regex ───────────────────────────────────────────────────────────

    pub async fn regex_op(&self, args: Value) -> Result<Value> {
        let text = args["text"].as_str().context("Missing 'text' parameter")?;
        let pattern_str = args["pattern"].as_str().context("Missing 'pattern' parameter")?;
        let action = args["action"].as_str().unwrap_or("find_all");
        let flags = args["flags"].as_str().unwrap_or("");

        let mut pattern = String::new();
        if !flags.is_empty() {
            pattern.push_str("(?");
            pattern.push_str(flags);
            pattern.push(')');
        }
        pattern.push_str(pattern_str);

        let re = regex::Regex::new(&pattern)
            .with_context(|| format!("Invalid regex pattern: {}", pattern_str))?;

        match action {
            "match" => {
                let is_match = re.is_match(text);
                let first = re.find(text).map(|m| json!({
                    "text": m.as_str(),
                    "start": m.start(),
                    "end": m.end()
                }));

                Ok(json!({
                    "matches": is_match,
                    "first_match": first,
                    "pattern": pattern_str
                }))
            }
            "find_all" => {
                let matches: Vec<Value> = re.find_iter(text).map(|m| {
                    json!({
                        "text": m.as_str(),
                        "start": m.start(),
                        "end": m.end()
                    })
                }).collect();

                Ok(json!({
                    "matches": matches,
                    "count": matches.len(),
                    "pattern": pattern_str
                }))
            }
            "replace" => {
                let replacement = args["replacement"].as_str().unwrap_or("");
                let result = re.replace_all(text, replacement).to_string();
                let count = re.find_iter(text).count();

                Ok(json!({
                    "result": result,
                    "replacements": count,
                    "pattern": pattern_str
                }))
            }
            "split" => {
                let parts: Vec<&str> = re.split(text).collect();

                Ok(json!({
                    "parts": parts,
                    "count": parts.len(),
                    "pattern": pattern_str
                }))
            }
            "extract" => {
                let mut groups = Vec::new();
                for caps in re.captures_iter(text) {
                    let mut group = Vec::new();
                    for i in 0..caps.len() {
                        group.push(json!({
                            "group": i,
                            "text": caps.get(i).map(|m| m.as_str())
                        }));
                    }
                    groups.push(json!(group));
                }

                Ok(json!({
                    "captures": groups,
                    "count": groups.len(),
                    "pattern": pattern_str
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown regex action: {}", action)),
        }
    }

    // ── JSON ────────────────────────────────────────────────────────────

    pub async fn json_op(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing 'input' parameter")?;
        let action = args["action"].as_str().unwrap_or("pretty");

        match action {
            "validate" => {
                match serde_json::from_str::<Value>(input) {
                    Ok(_) => Ok(json!({ "valid": true })),
                    Err(e) => Ok(json!({
                        "valid": false,
                        "error": e.to_string()
                    })),
                }
            }
            "pretty" => {
                let parsed: Value = serde_json::from_str(input)
                    .context("Invalid JSON input")?;
                let pretty = serde_json::to_string_pretty(&parsed)?;

                Ok(json!({
                    "result": pretty,
                    "action": "pretty"
                }))
            }
            "minify" => {
                let parsed: Value = serde_json::from_str(input)
                    .context("Invalid JSON input")?;
                let minified = serde_json::to_string(&parsed)?;

                Ok(json!({
                    "result": minified,
                    "action": "minify",
                    "original_length": input.len(),
                    "minified_length": minified.len()
                }))
            }
            "query" => {
                let query = args["query"].as_str().context("Missing 'query' parameter")?;
                let parsed: Value = serde_json::from_str(input)
                    .context("Invalid JSON input")?;
                let result = json_query(&parsed, query);

                Ok(json!({
                    "query": query,
                    "result": result,
                    "found": !result.is_null()
                }))
            }
            "merge" => {
                let merge_with = args["merge_with"].as_str()
                    .context("Missing 'merge_with' parameter")?;
                let mut base: Value = serde_json::from_str(input)
                    .context("Invalid JSON in 'input'")?;
                let overlay: Value = serde_json::from_str(merge_with)
                    .context("Invalid JSON in 'merge_with'")?;

                json_merge(&mut base, &overlay);
                let result = serde_json::to_string_pretty(&base)?;

                Ok(json!({
                    "result": result,
                    "action": "merge"
                }))
            }
            "keys" => {
                let parsed: Value = serde_json::from_str(input)
                    .context("Invalid JSON input")?;

                let keys = match &parsed {
                    Value::Object(map) => map.keys().cloned().collect::<Vec<_>>(),
                    _ => return Err(anyhow::anyhow!("Input is not a JSON object")),
                };

                Ok(json!({
                    "keys": keys,
                    "count": keys.len()
                }))
            }
            "flatten" => {
                let parsed: Value = serde_json::from_str(input)
                    .context("Invalid JSON input")?;
                let mut flat = serde_json::Map::new();
                json_flatten(&parsed, String::new(), &mut flat);

                Ok(json!({
                    "result": Value::Object(flat.clone()),
                    "count": flat.len()
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown JSON action: {}", action)),
        }
    }

    // ── Text ────────────────────────────────────────────────────────────

    pub async fn text(&self, args: Value) -> Result<Value> {
        let text = args["text"].as_str().context("Missing 'text' parameter")?;
        let action = args["action"].as_str().context("Missing 'action' parameter")?;
        let width = args["width"].as_u64().unwrap_or(80) as usize;

        match action {
            "uppercase" => Ok(json!({ "result": text.to_uppercase() })),
            "lowercase" => Ok(json!({ "result": text.to_lowercase() })),
            "title_case" => {
                let result: String = text.split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str().to_lowercase()),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                Ok(json!({ "result": result }))
            }
            "snake_case" => {
                let result = to_snake_case(text);
                Ok(json!({ "result": result }))
            }
            "camel_case" => {
                let result = to_camel_case(text);
                Ok(json!({ "result": result }))
            }
            "kebab_case" => {
                let result = to_kebab_case(text);
                Ok(json!({ "result": result }))
            }
            "sort_lines" => {
                let mut lines: Vec<&str> = text.lines().collect();
                lines.sort();
                Ok(json!({ "result": lines.join("\n") }))
            }
            "reverse_lines" => {
                let mut lines: Vec<&str> = text.lines().collect();
                lines.reverse();
                Ok(json!({ "result": lines.join("\n") }))
            }
            "unique_lines" => {
                let mut seen = std::collections::HashSet::new();
                let lines: Vec<&str> = text.lines()
                    .filter(|line| seen.insert(*line))
                    .collect();
                let removed = text.lines().count() - lines.len();
                Ok(json!({
                    "result": lines.join("\n"),
                    "duplicates_removed": removed
                }))
            }
            "trim_lines" => {
                let lines: Vec<&str> = text.lines()
                    .map(|l| l.trim())
                    .collect();
                Ok(json!({ "result": lines.join("\n") }))
            }
            "number_lines" => {
                let lines: Vec<String> = text.lines()
                    .enumerate()
                    .map(|(i, l)| format!("{:>4} | {}", i + 1, l))
                    .collect();
                Ok(json!({ "result": lines.join("\n") }))
            }
            "wrap" => {
                let lines: Vec<String> = text.lines()
                    .flat_map(|line| wrap_line(line, width))
                    .collect();
                Ok(json!({ "result": lines.join("\n"), "width": width }))
            }
            "truncate" => {
                let result = if text.len() > width {
                    format!("{}...", &text[..width.saturating_sub(3)])
                } else {
                    text.to_string()
                };
                Ok(json!({
                    "result": result,
                    "truncated": text.len() > width,
                    "original_length": text.len()
                }))
            }
            "stats" => {
                let lines: Vec<&str> = text.lines().collect();
                let words: Vec<&str> = text.split_whitespace().collect();
                let chars = text.chars().count();
                let bytes = text.len();
                let unique_words: std::collections::HashSet<_> = words.iter()
                    .map(|w| w.to_lowercase())
                    .collect();
                let avg_line_len = if lines.is_empty() { 0.0 } else {
                    chars as f64 / lines.len() as f64
                };

                Ok(json!({
                    "lines": lines.len(),
                    "words": words.len(),
                    "characters": chars,
                    "bytes": bytes,
                    "unique_words": unique_words.len(),
                    "average_line_length": format!("{:.1}", avg_line_len),
                    "longest_line": lines.iter().map(|l| l.len()).max().unwrap_or(0),
                    "shortest_line": lines.iter().map(|l| l.len()).min().unwrap_or(0)
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown text action: {}", action)),
        }
    }

    // ── Archive ─────────────────────────────────────────────────────────

    pub async fn archive(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().context("Missing 'action' parameter")?;
        let path = args["path"].as_str().context("Missing 'path' parameter")?;
        let format = args["format"].as_str().unwrap_or("zip");

        match action {
            "create" => self.archive_create(path, &args, format).await,
            "extract" => self.archive_extract(path, &args, format).await,
            "list" => self.archive_list(path, format).await,
            _ => Err(anyhow::anyhow!("Unknown archive action: {}", action)),
        }
    }

    async fn archive_create(&self, path: &str, args: &Value, format: &str) -> Result<Value> {
        let files = args["files"].as_array()
            .context("Missing 'files' parameter for create")?;

        let file_paths: Vec<&str> = files.iter()
            .filter_map(|v| v.as_str())
            .collect();

        match format {
            "zip" => {
                let file = fs::File::create(path)
                    .with_context(|| format!("Failed to create archive: {}", path))?;
                let mut archive = zip::ZipWriter::new(file);
                let options: zip::write::FileOptions = zip::write::FileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);

                for file_path in &file_paths {
                    let p = Path::new(file_path);
                    if p.is_file() {
                        let name = p.file_name().unwrap_or_default().to_string_lossy();
                        archive.start_file(name.to_string(), options)?;
                        let content = fs::read(file_path)?;
                        archive.write_all(&content)?;
                    } else if p.is_dir() {
                        add_dir_to_zip(&mut archive, p, p, options)?;
                    }
                }

                archive.finish()?;
            }
            "tar_gz" => {
                let file = fs::File::create(path)
                    .with_context(|| format!("Failed to create archive: {}", path))?;
                let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
                let mut archive = tar::Builder::new(enc);

                for file_path in &file_paths {
                    let p = Path::new(file_path);
                    if p.is_file() {
                        let name = p.file_name().unwrap_or_default().to_string_lossy();
                        archive.append_path_with_name(p, name.as_ref())?;
                    } else if p.is_dir() {
                        let dir_name = p.file_name().unwrap_or_default().to_string_lossy();
                        archive.append_dir_all(dir_name.as_ref(), p)?;
                    }
                }

                archive.finish()?;
            }
            _ => return Err(anyhow::anyhow!("Unsupported archive format: {}", format)),
        }

        let size = fs::metadata(path)?.len();

        Ok(json!({
            "success": true,
            "path": path,
            "format": format,
            "files_added": file_paths.len(),
            "archive_size": size
        }))
    }

    async fn archive_extract(&self, path: &str, args: &Value, format: &str) -> Result<Value> {
        let destination = args["destination"].as_str().unwrap_or(".");

        fs::create_dir_all(destination)
            .with_context(|| format!("Failed to create destination: {}", destination))?;

        let mut extracted = Vec::new();

        match format {
            "zip" => {
                let file = fs::File::open(path)
                    .with_context(|| format!("Failed to open archive: {}", path))?;
                let mut archive = zip::ZipArchive::new(file)?;

                for i in 0..archive.len() {
                    let mut entry = archive.by_index(i)?;
                    let name = entry.name().to_string();
                    let out_path = Path::new(destination).join(&name);

                    if entry.is_dir() {
                        fs::create_dir_all(&out_path)?;
                    } else {
                        if let Some(parent) = out_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut outfile = fs::File::create(&out_path)?;
                        std::io::copy(&mut entry, &mut outfile)?;
                    }
                    extracted.push(name);
                }
            }
            "tar_gz" => {
                let file = fs::File::open(path)
                    .with_context(|| format!("Failed to open archive: {}", path))?;
                let dec = flate2::read::GzDecoder::new(file);
                let mut archive = tar::Archive::new(dec);

                for entry in archive.entries()? {
                    let mut entry = entry?;
                    let entry_path = entry.path()?.to_path_buf();
                    let name = entry_path.to_string_lossy().to_string();
                    let out_path = Path::new(destination).join(&entry_path);

                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&out_path)?;
                    extracted.push(name);
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported archive format: {}", format)),
        }

        Ok(json!({
            "success": true,
            "path": path,
            "destination": destination,
            "format": format,
            "files_extracted": extracted.len(),
            "files": extracted
        }))
    }

    async fn archive_list(&self, path: &str, format: &str) -> Result<Value> {
        let mut entries = Vec::new();

        match format {
            "zip" => {
                let file = fs::File::open(path)
                    .with_context(|| format!("Failed to open archive: {}", path))?;
                let mut archive = zip::ZipArchive::new(file)?;

                for i in 0..archive.len() {
                    let entry = archive.by_index_raw(i)?;
                    entries.push(json!({
                        "name": entry.name(),
                        "size": entry.size(),
                        "compressed_size": entry.compressed_size(),
                        "is_dir": entry.is_dir()
                    }));
                }
            }
            "tar_gz" => {
                let file = fs::File::open(path)
                    .with_context(|| format!("Failed to open archive: {}", path))?;
                let dec = flate2::read::GzDecoder::new(file);
                let mut archive = tar::Archive::new(dec);

                for entry in archive.entries()? {
                    let entry = entry?;
                    let path = entry.path()?.to_string_lossy().to_string();
                    let size = entry.size();

                    entries.push(json!({
                        "name": path,
                        "size": size,
                        "is_dir": entry.header().entry_type().is_dir()
                    }));
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported archive format: {}", format)),
        }

        Ok(json!({
            "path": path,
            "format": format,
            "entries": entries,
            "count": entries.len()
        }))
    }
}

// ── Helper functions ────────────────────────────────────────────────────

fn json_query(value: &Value, query: &str) -> Value {
    let mut current = value.clone();

    for part in query.split('.') {
        // Handle array indexing like "users[0]"
        if let Some(bracket_pos) = part.find('[') {
            let key = &part[..bracket_pos];
            let idx_str = &part[bracket_pos + 1..part.len() - 1];

            if !key.is_empty() {
                current = current.get(key).cloned().unwrap_or(Value::Null);
            }

            if let Ok(idx) = idx_str.parse::<usize>() {
                current = current.get(idx).cloned().unwrap_or(Value::Null);
            } else {
                return Value::Null;
            }
        } else {
            current = current.get(part).cloned().unwrap_or(Value::Null);
        }

        if current.is_null() {
            return Value::Null;
        }
    }

    current
}

fn json_merge(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                let entry = base_map.entry(key.clone()).or_insert(Value::Null);
                json_merge(entry, value);
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

fn json_flatten(value: &Value, prefix: String, result: &mut serde_json::Map<String, Value>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let new_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                json_flatten(val, new_key, result);
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let new_key = format!("{}[{}]", prefix, i);
                json_flatten(val, new_key, result);
            }
        }
        _ => {
            result.insert(prefix, value.clone());
        }
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else if c == ' ' || c == '-' {
            result.push('_');
        } else {
            result.push(c);
        }
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split(|c: char| c == '_' || c == '-' || c == ' ')
        .filter(|p| !p.is_empty())
        .collect();
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            result.push_str(&part.to_lowercase());
        } else {
            let mut chars = part.chars();
            if let Some(c) = chars.next() {
                result.push(c.to_uppercase().next().unwrap());
                result.push_str(&chars.as_str().to_lowercase());
            }
        }
    }
    result
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else if c == '_' || c == ' ' {
            result.push('-');
        } else {
            result.push(c);
        }
    }
    result
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if line.len() <= width {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in line.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![line.to_string()]
    } else {
        lines
    }
}

fn add_dir_to_zip<W: IoWrite + std::io::Seek>(
    archive: &mut zip::ZipWriter<W>,
    dir: &Path,
    base: &Path,
    options: zip::write::FileOptions,
) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base).unwrap_or(path);

        if path.is_file() {
            archive.start_file(relative.to_string_lossy().to_string(), options)?;
            let content = fs::read(path)?;
            archive.write_all(&content)?;
        } else if path.is_dir() && path != base {
            archive.add_directory(relative.to_string_lossy().to_string(), options)?;
        }
    }
    Ok(())
}
