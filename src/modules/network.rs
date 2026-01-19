use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use reqwest;
use html2md;
use std::process::Command;
use std::time::Duration;

pub struct NetworkModule {
    client: reqwest::Client,
}

impl NetworkModule {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("poly-mcp/0.1.0")
            .build()
            .unwrap();

        Self { client }
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "net_fetch",
                "description": "Fetch content from URLs with automatic HTML to Markdown conversion",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to fetch"
                        },
                        "method": {
                            "type": "string",
                            "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"],
                            "description": "HTTP method (default: GET)"
                        },
                        "headers": {
                            "type": "object",
                            "description": "HTTP headers"
                        },
                        "body": {
                            "type": "string",
                            "description": "Request body (for POST/PUT/PATCH)"
                        },
                        "convert_to_markdown": {
                            "type": "boolean",
                            "description": "Convert HTML to Markdown (default: true)"
                        }
                    },
                    "required": ["url"]
                }
            }),
            json!({
                "name": "net_cargo",
                "description": "Query crates.io for Rust package information",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "crate_name": {
                            "type": "string",
                            "description": "Name of the crate"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["info", "search", "latest"],
                            "description": "Action to perform (default: info)"
                        }
                    },
                    "required": ["crate_name"]
                }
            }),
            json!({
                "name": "net_node",
                "description": "Query npm registry for Node.js package information",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "package_name": {
                            "type": "string",
                            "description": "Name of the package"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["info", "search", "latest"],
                            "description": "Action to perform (default: info)"
                        }
                    },
                    "required": ["package_name"]
                }
            }),
            json!({
                "name": "net_python",
                "description": "Query PyPI for Python package information",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "package_name": {
                            "type": "string",
                            "description": "Name of the package"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["info", "search", "latest"],
                            "description": "Action to perform (default: info)"
                        }
                    },
                    "required": ["package_name"]
                }
            }),
            json!({
                "name": "net_apt",
                "description": "Query APT package information",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "package_name": {
                            "type": "string",
                            "description": "Name of the package"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["info", "search", "show"],
                            "description": "Action to perform (default: info)"
                        }
                    },
                    "required": ["package_name"]
                }
            }),
            json!({
                "name": "net_ping",
                "description": "Check network connectivity to a host",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "host": {
                            "type": "string",
                            "description": "Host to ping"
                        },
                        "count": {
                            "type": "number",
                            "description": "Number of ping attempts (default: 4)"
                        },
                        "timeout": {
                            "type": "number",
                            "description": "Timeout in seconds (default: 5)"
                        }
                    },
                    "required": ["host"]
                }
            }),
        ]
    }

    pub async fn fetch(&self, args: Value) -> Result<Value> {
        let url = args["url"].as_str().context("Missing 'url' parameter")?;
        let method = args["method"].as_str().unwrap_or("GET");
        let convert_to_markdown = args["convert_to_markdown"].as_bool().unwrap_or(true);

        let mut request = match method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            "PATCH" => self.client.patch(url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
        };

        // Add headers
        if let Some(headers_obj) = args["headers"].as_object() {
            for (key, value) in headers_obj {
                if let Some(val_str) = value.as_str() {
                    request = request.header(key, val_str);
                }
            }
        }

        // Add body for POST/PUT/PATCH
        if let Some(body) = args["body"].as_str() {
            request = request.body(body.to_string());
        }

        let response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();

        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let body_text = response.text().await?;

        let processed_content = if convert_to_markdown && content_type.contains("text/html") {
            html2md::parse_html(&body_text)
        } else {
            body_text.clone()
        };

        let headers_map: serde_json::Map<String, Value> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v.to_str().unwrap_or(""))))
            .collect();

        Ok(json!({
            "url": url,
            "status": status.as_u16(),
            "status_text": status.canonical_reason().unwrap_or(""),
            "headers": headers_map,
            "content_type": content_type,
            "body": processed_content,
            "raw_body": body_text,
            "converted_to_markdown": convert_to_markdown && content_type.contains("text/html")
        }))
    }

    pub async fn cargo(&self, args: Value) -> Result<Value> {
        let crate_name = args["crate_name"].as_str().context("Missing 'crate_name' parameter")?;
        let action = args["action"].as_str().unwrap_or("info");

        match action {
            "latest" => {
                // Use cargo search to get latest version
                let output = Command::new("cargo")
                    .arg("search")
                    .arg(crate_name)
                    .arg("--limit")
                    .arg("1")
                    .output()
                    .context("Failed to run cargo search")?;

                let stdout = String::from_utf8_lossy(&output.stdout);

                if let Some(first_line) = stdout.lines().next() {
                    // Parse: "crate_name = \"version\"    # description"
                    if let Some(version_part) = first_line.split('=').nth(1) {
                        let version = version_part
                            .trim()
                            .trim_start_matches('"')
                            .split('"')
                            .next()
                            .unwrap_or("unknown");

                        return Ok(json!({
                            "crate": crate_name,
                            "latest_version": version,
                            "source": "cargo search"
                        }));
                    }
                }

                Err(anyhow::anyhow!("Crate not found: {}", crate_name))
            }
            "info" | "search" => {
                // Query crates.io API
                let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
                let response = self.client.get(&url).send().await?;

                if response.status().is_success() {
                    let data: Value = response.json().await?;

                    Ok(json!({
                        "crate": crate_name,
                        "info": data["crate"],
                        "versions": data["versions"],
                        "latest_version": data["crate"]["newest_version"],
                        "description": data["crate"]["description"],
                        "downloads": data["crate"]["downloads"],
                        "documentation": data["crate"]["documentation"],
                        "repository": data["crate"]["repository"],
                        "homepage": data["crate"]["homepage"]
                    }))
                } else {
                    Err(anyhow::anyhow!("Crate not found: {}", crate_name))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    pub async fn node(&self, args: Value) -> Result<Value> {
        let package_name = args["package_name"].as_str().context("Missing 'package_name' parameter")?;
        let action = args["action"].as_str().unwrap_or("info");

        match action {
            "latest" => {
                // Use npm view to get latest version
                let output = Command::new("npm")
                    .arg("view")
                    .arg(package_name)
                    .arg("version")
                    .output()
                    .context("Failed to run npm view")?;

                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

                if !version.is_empty() {
                    Ok(json!({
                        "package": package_name,
                        "latest_version": version,
                        "source": "npm view"
                    }))
                } else {
                    Err(anyhow::anyhow!("Package not found: {}", package_name))
                }
            }
            "info" | "search" => {
                // Query npm registry API
                let url = format!("https://registry.npmjs.org/{}", package_name);
                let response = self.client.get(&url).send().await?;

                if response.status().is_success() {
                    let data: Value = response.json().await?;

                    let latest_version = data["dist-tags"]["latest"].as_str().unwrap_or("unknown");

                    Ok(json!({
                        "package": package_name,
                        "latest_version": latest_version,
                        "description": data["description"],
                        "author": data["author"],
                        "license": data["license"],
                        "homepage": data["homepage"],
                        "repository": data["repository"],
                        "versions": data["versions"].as_object().map(|v| v.keys().collect::<Vec<_>>()),
                        "keywords": data["keywords"],
                        "dependencies": data["versions"][latest_version]["dependencies"]
                    }))
                } else {
                    Err(anyhow::anyhow!("Package not found: {}", package_name))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    pub async fn python(&self, args: Value) -> Result<Value> {
        let package_name = args["package_name"].as_str().context("Missing 'package_name' parameter")?;
        let action = args["action"].as_str().unwrap_or("info");

        match action {
            "latest" => {
                // Use pip index to get latest version
                let output = Command::new("pip3")
                    .arg("index")
                    .arg("versions")
                    .arg(package_name)
                    .output()
                    .context("Failed to run pip3 index")?;

                let stdout = String::from_utf8_lossy(&output.stdout);

                // Parse output for "LATEST: version"
                for line in stdout.lines() {
                    if line.contains("LATEST:") || line.contains("Available versions:") {
                        if let Some(version) = line.split_whitespace().nth(1) {
                            return Ok(json!({
                                "package": package_name,
                                "latest_version": version.trim_end_matches(','),
                                "source": "pip3 index"
                            }));
                        }
                    }
                }

                // Fallback: query PyPI API
                self.query_pypi_api(package_name, action).await
            }
            "info" | "search" => {
                self.query_pypi_api(package_name, action).await
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    async fn query_pypi_api(&self, package_name: &str, _action: &str) -> Result<Value> {
        let url = format!("https://pypi.org/pypi/{}/json", package_name);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let data: Value = response.json().await?;

            Ok(json!({
                "package": package_name,
                "latest_version": data["info"]["version"],
                "description": data["info"]["summary"],
                "author": data["info"]["author"],
                "license": data["info"]["license"],
                "homepage": data["info"]["home_page"],
                "project_urls": data["info"]["project_urls"],
                "requires_python": data["info"]["requires_python"],
                "classifiers": data["info"]["classifiers"]
            }))
        } else {
            Err(anyhow::anyhow!("Package not found: {}", package_name))
        }
    }

    pub async fn apt(&self, args: Value) -> Result<Value> {
        let package_name = args["package_name"].as_str().context("Missing 'package_name' parameter")?;
        let action = args["action"].as_str().unwrap_or("info");

        match action {
            "info" | "show" => {
                let output = Command::new("apt")
                    .arg("show")
                    .arg(package_name)
                    .output()
                    .context("Failed to run apt show")?;

                let stdout = String::from_utf8_lossy(&output.stdout);

                let mut info = serde_json::Map::new();

                for line in stdout.lines() {
                    if let Some((key, value)) = line.split_once(':') {
                        info.insert(
                            key.trim().to_lowercase().replace('-', "_"),
                            json!(value.trim())
                        );
                    }
                }

                Ok(json!({
                    "package": package_name,
                    "info": info
                }))
            }
            "search" => {
                let output = Command::new("apt")
                    .arg("search")
                    .arg(package_name)
                    .output()
                    .context("Failed to run apt search")?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut results = Vec::new();

                for line in stdout.lines() {
                    if line.contains('/') {
                        results.push(line.to_string());
                    }
                }

                Ok(json!({
                    "query": package_name,
                    "results": results,
                    "count": results.len()
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    pub async fn ping(&self, args: Value) -> Result<Value> {
        let host = args["host"].as_str().context("Missing 'host' parameter")?;
        let count = args["count"].as_u64().unwrap_or(4);
        let timeout = args["timeout"].as_u64().unwrap_or(5);

        let output = Command::new("ping")
            .arg("-c")
            .arg(count.to_string())
            .arg("-W")
            .arg(timeout.to_string())
            .arg(host)
            .output()
            .context("Failed to run ping")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let success = output.status.success();

        // Parse ping statistics
        let mut sent = 0;
        let mut received = 0;
        let mut min = 0.0;
        let mut avg = 0.0;
        let mut max = 0.0;

        for line in stdout.lines() {
            if line.contains("packets transmitted") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    sent = parts[0].parse().unwrap_or(0);
                    received = parts[3].parse().unwrap_or(0);
                }
            }
            if line.contains("min/avg/max") || line.contains("rtt") {
                if let Some(stats_part) = line.split('=').nth(1) {
                    let stats: Vec<&str> = stats_part.trim().split('/').collect();
                    if stats.len() >= 3 {
                        min = stats[0].parse().unwrap_or(0.0);
                        avg = stats[1].parse().unwrap_or(0.0);
                        max = stats[2].split_whitespace().next().unwrap_or("0").parse().unwrap_or(0.0);
                    }
                }
            }
        }

        let packet_loss = if sent > 0 {
            ((sent - received) as f64 / sent as f64) * 100.0
        } else {
            100.0
        };

        Ok(json!({
            "host": host,
            "reachable": success,
            "packets_sent": sent,
            "packets_received": received,
            "packet_loss_percent": packet_loss,
            "rtt_min_ms": min,
            "rtt_avg_ms": avg,
            "rtt_max_ms": max,
            "raw_output": stdout.to_string()
        }))
    }
}
