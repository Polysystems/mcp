use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::process::Command;
use std::path::Path;

pub struct DiagnosticsModule;

impl DiagnosticsModule {
    pub fn new() -> Self {
        Self
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "diagnostics_get",
                "description": "Get errors and warnings for a specific file or entire project (language-agnostic)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to file or directory to check (default: current directory)"
                        },
                        "tool": {
                            "type": "string",
                            "description": "Specific diagnostic tool to use (auto-detected if not specified)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["json", "text"],
                            "description": "Output format (default: json)"
                        }
                    }
                }
            }),
        ]
    }

    pub async fn get(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let tool = args["tool"].as_str();
        let format = args["format"].as_str().unwrap_or("json");

        let path_obj = Path::new(path);

        // Auto-detect diagnostic tool if not specified
        let detected_tool = if let Some(t) = tool {
            t.to_string()
        } else {
            self.detect_tool(path_obj)?
        };

        let diagnostics = match detected_tool.as_str() {
            "cargo" => self.run_cargo_diagnostics(path)?,
            "rustc" => self.run_rustc_diagnostics(path)?,
            "tsc" => self.run_tsc_diagnostics(path)?,
            "eslint" => self.run_eslint_diagnostics(path)?,
            "pylint" => self.run_pylint_diagnostics(path)?,
            "mypy" => self.run_mypy_diagnostics(path)?,
            "ruff" => self.run_ruff_diagnostics(path)?,
            "gcc" | "g++" => self.run_gcc_diagnostics(path)?,
            "clang" => self.run_clang_diagnostics(path)?,
            _ => anyhow::bail!("Unsupported diagnostic tool: {}", detected_tool),
        };

        Ok(json!({
            "path": path,
            "tool": detected_tool,
            "diagnostics": diagnostics,
            "format": format
        }))
    }

    fn detect_tool(&self, path: &Path) -> Result<String> {
        // Check for Rust
        if path.join("Cargo.toml").exists() || path.extension().map_or(false, |e| e == "rs") {
            return Ok("cargo".to_string());
        }

        // Check for TypeScript/JavaScript
        if path.join("tsconfig.json").exists() || path.extension().map_or(false, |e| e == "ts" || e == "tsx") {
            return Ok("tsc".to_string());
        }

        if path.join("package.json").exists() || path.extension().map_or(false, |e| e == "js" || e == "jsx") {
            return Ok("eslint".to_string());
        }

        // Check for Python
        if path.extension().map_or(false, |e| e == "py") {
            // Prefer ruff if available, fallback to pylint
            if Command::new("ruff").arg("--version").output().is_ok() {
                return Ok("ruff".to_string());
            }
            return Ok("pylint".to_string());
        }

        // Check for C/C++
        if path.extension().map_or(false, |e| e == "c" || e == "cpp" || e == "cc" || e == "cxx") {
            if Command::new("clang").arg("--version").output().is_ok() {
                return Ok("clang".to_string());
            }
            return Ok("gcc".to_string());
        }

        anyhow::bail!("Could not detect appropriate diagnostic tool for: {}", path.display())
    }

    fn run_cargo_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=json")
            .current_dir(path)
            .output()
            .context("Failed to run cargo check")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();

        for line in stdout.lines() {
            if let Ok(msg) = serde_json::from_str::<Value>(line) {
                if msg["reason"] == "compiler-message" {
                    if let Some(message) = msg.get("message") {
                        diagnostics.push(json!({
                            "level": message["level"],
                            "message": message["message"],
                            "file": message["spans"][0]["file_name"],
                            "line": message["spans"][0]["line_start"],
                            "column": message["spans"][0]["column_start"],
                            "code": message.get("code").and_then(|c| c.get("code"))
                        }));
                    }
                }
            }
        }

        Ok(diagnostics)
    }

    fn run_rustc_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("rustc")
            .arg("--error-format=json")
            .arg(path)
            .output()
            .context("Failed to run rustc")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut diagnostics = Vec::new();

        for line in stdout.lines().chain(stderr.lines()) {
            if let Ok(msg) = serde_json::from_str::<Value>(line) {
                if msg["$message_type"] == "diagnostic" {
                    diagnostics.push(json!({
                        "level": msg["level"],
                        "message": msg["message"],
                        "file": msg["spans"][0]["file_name"],
                        "line": msg["spans"][0]["line_start"],
                        "column": msg["spans"][0]["column_start"]
                    }));
                }
            }
        }

        Ok(diagnostics)
    }

    fn run_tsc_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("tsc")
            .arg("--noEmit")
            .arg("--pretty")
            .arg("false")
            .current_dir(path)
            .output()
            .context("Failed to run tsc")?;

        self.parse_generic_output(&output.stdout, &output.stderr)
    }

    fn run_eslint_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("eslint")
            .arg("--format=json")
            .arg(path)
            .output()
            .context("Failed to run eslint")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(results) = serde_json::from_str::<Value>(&stdout) {
            let mut diagnostics = Vec::new();

            if let Some(files) = results.as_array() {
                for file in files {
                    if let Some(messages) = file["messages"].as_array() {
                        for msg in messages {
                            diagnostics.push(json!({
                                "level": if msg["severity"] == 2 { "error" } else { "warning" },
                                "message": msg["message"],
                                "file": file["filePath"],
                                "line": msg["line"],
                                "column": msg["column"],
                                "code": msg["ruleId"]
                            }));
                        }
                    }
                }
            }

            Ok(diagnostics)
        } else {
            self.parse_generic_output(&output.stdout, &output.stderr)
        }
    }

    fn run_pylint_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("pylint")
            .arg("--output-format=json")
            .arg(path)
            .output()
            .context("Failed to run pylint")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(results) = serde_json::from_str::<Value>(&stdout) {
            let mut diagnostics = Vec::new();

            if let Some(messages) = results.as_array() {
                for msg in messages {
                    diagnostics.push(json!({
                        "level": msg["type"],
                        "message": msg["message"],
                        "file": msg["path"],
                        "line": msg["line"],
                        "column": msg["column"],
                        "code": msg["message-id"]
                    }));
                }
            }

            Ok(diagnostics)
        } else {
            self.parse_generic_output(&output.stdout, &output.stderr)
        }
    }

    fn run_mypy_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("mypy")
            .arg(path)
            .output()
            .context("Failed to run mypy")?;

        self.parse_generic_output(&output.stdout, &output.stderr)
    }

    fn run_ruff_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("ruff")
            .arg("check")
            .arg("--output-format=json")
            .arg(path)
            .output()
            .context("Failed to run ruff")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(results) = serde_json::from_str::<Value>(&stdout) {
            let mut diagnostics = Vec::new();

            if let Some(messages) = results.as_array() {
                for msg in messages {
                    diagnostics.push(json!({
                        "level": msg["type"],
                        "message": msg["message"],
                        "file": msg["filename"],
                        "line": msg["location"]["row"],
                        "column": msg["location"]["column"],
                        "code": msg["code"]
                    }));
                }
            }

            Ok(diagnostics)
        } else {
            self.parse_generic_output(&output.stdout, &output.stderr)
        }
    }

    fn run_gcc_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("gcc")
            .arg("-fsyntax-only")
            .arg("-fdiagnostics-format=json")
            .arg(path)
            .output()
            .context("Failed to run gcc")?;

        self.parse_generic_output(&output.stdout, &output.stderr)
    }

    fn run_clang_diagnostics(&self, path: &str) -> Result<Vec<Value>> {
        let output = Command::new("clang")
            .arg("-fsyntax-only")
            .arg("-fdiagnostics-format=json")
            .arg(path)
            .output()
            .context("Failed to run clang")?;

        self.parse_generic_output(&output.stdout, &output.stderr)
    }

    fn parse_generic_output(&self, stdout: &[u8], stderr: &[u8]) -> Result<Vec<Value>> {
        let output = String::from_utf8_lossy(stdout);
        let error_output = String::from_utf8_lossy(stderr);

        let combined = format!("{}\n{}", output, error_output);
        let mut diagnostics = Vec::new();

        // Try to parse common diagnostic patterns
        for line in combined.lines() {
            // Pattern: file:line:column: level: message
            if let Some(caps) = self.parse_diagnostic_line(line) {
                diagnostics.push(caps);
            }
        }

        if diagnostics.is_empty() {
            // If no structured diagnostics found, return raw output
            diagnostics.push(json!({
                "level": "info",
                "message": combined.trim(),
                "raw": true
            }));
        }

        Ok(diagnostics)
    }

    fn parse_diagnostic_line(&self, line: &str) -> Option<Value> {
        // Common pattern: file:line:column: level: message
        let parts: Vec<&str> = line.splitn(5, ':').collect();

        if parts.len() >= 4 {
            let file = parts[0].trim();
            let line_num = parts[1].trim().parse::<u64>().ok()?;
            let column = parts[2].trim().parse::<u64>().ok();
            let level = parts[3].trim();
            let message = parts.get(4).map(|s| s.trim()).unwrap_or("");

            return Some(json!({
                "file": file,
                "line": line_num,
                "column": column,
                "level": level,
                "message": message
            }));
        }

        None
    }
}
