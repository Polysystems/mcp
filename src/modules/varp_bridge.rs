use serde_json::{json, Value};
use anyhow::{Result, Context};
use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// Wraps the `varp-bridge` binary as a poly-mcp compatible module.
/// Communicates via stdin/stdout JSON-RPC — zero VARP source code needed.
pub struct VarpModule {
    child: Mutex<Child>,
    tools_cache: Vec<Value>,
}

impl VarpModule {
    /// Spawn the varp-bridge binary and fetch tool definitions.
    /// Looks for `varp-bridge` in PATH or VARP_BRIDGE_PATH env.
    pub fn new() -> Result<Option<Self>> {
        // Check for license key first — no key means skip entirely
        if std::env::var("VARP_LICENSE_KEY").unwrap_or_default().is_empty() {
            return Ok(None);
        }

        let bin = std::env::var("VARP_BRIDGE_PATH")
            .unwrap_or_else(|_| "varp-bridge".to_string());

        let mut child = Command::new(&bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn '{}' — is varp-bridge installed?", bin))?;

        // Fetch tool definitions
        let tools = Self::rpc_sync(&mut child, json!({
            "method": "tools/list",
            "id": 1
        }))?;

        let tools_cache = tools["result"]["tools"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        if tools_cache.is_empty() {
            // Bridge started but returned no tools — license might be invalid
            let _ = child.kill();
            return Err(anyhow::anyhow!("varp-bridge returned no tools (check license key)"));
        }

        Ok(Some(Self {
            child: Mutex::new(child),
            tools_cache,
        }))
    }

    pub fn get_tools(&self) -> Vec<Value> {
        self.tools_cache.clone()
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        let request = json!({
            "method": "tools/call",
            "id": 1,
            "params": {
                "name": name,
                "arguments": args
            }
        });

        let mut child = self.child.lock()
            .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;

        let resp = Self::rpc_sync(&mut child, request)?;

        if let Some(err) = resp.get("error") {
            Err(anyhow::anyhow!("{}", err["message"].as_str().unwrap_or("unknown error")))
        } else {
            Ok(resp.get("result").cloned().unwrap_or(json!(null)))
        }
    }

    /// Send a JSON-RPC request and read the response (synchronous, for subprocess IO).
    fn rpc_sync(child: &mut Child, request: Value) -> Result<Value> {
        let stdin = child.stdin.as_mut()
            .context("varp-bridge stdin not available")?;
        let stdout = child.stdout.as_mut()
            .context("varp-bridge stdout not available")?;

        let mut line = serde_json::to_string(&request)?;
        line.push('\n');
        stdin.write_all(line.as_bytes())?;
        stdin.flush()?;

        let mut reader = std::io::BufReader::new(stdout);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;

        serde_json::from_str(&response_line)
            .context("failed to parse varp-bridge response")
    }
}

impl Drop for VarpModule {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
