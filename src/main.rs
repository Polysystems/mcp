use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use anyhow::Result;

mod modules;
use modules::{
    filesystem::FilesystemModule,
    diagnostics::DiagnosticsModule,
    silent::SilentModule,
    time::TimeModule,
    network::NetworkModule,
    context::ContextModule,
    git::GitModule,
    input::InputModule,
};

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

struct PolyMcp {
    filesystem: FilesystemModule,
    diagnostics: DiagnosticsModule,
    silent: SilentModule,
    time: TimeModule,
    network: NetworkModule,
    context: ContextModule,
    git: GitModule,
    input: InputModule,
}

impl PolyMcp {
    fn new() -> Self {
        Self {
            filesystem: FilesystemModule::new(),
            diagnostics: DiagnosticsModule::new(),
            silent: SilentModule::new(),
            time: TimeModule::new(),
            network: NetworkModule::new(),
            context: ContextModule::new(),
            git: GitModule::new(),
            input: InputModule::new(),
        }
    }

    fn get_server_info(&self) -> Value {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "poly-mcp",
                "version": "0.1.0"
            }
        })
    }

    fn list_tools(&self) -> Value {
        let mut tools = Vec::new();

        // Filesystem tools
        tools.extend(self.filesystem.get_tools());

        // Diagnostics tools
        tools.extend(self.diagnostics.get_tools());

        // Silent tools
        tools.extend(self.silent.get_tools());

        // Time tools
        tools.extend(self.time.get_tools());

        // Network tools
        tools.extend(self.network.get_tools());

        // Context tools
        tools.extend(self.context.get_tools());

        // Git tools
        tools.extend(self.git.get_tools());

        // Input tools
        tools.extend(self.input.get_tools());

        json!({ "tools": tools })
    }

    async fn call_tool(&mut self, name: &str, arguments: Option<Value>) -> Result<Value> {
        let args = arguments.unwrap_or(json!({}));

        // Route to appropriate module
        match name {
            // Filesystem
            "fs_read" => self.filesystem.read(args).await,
            "fs_write" => self.filesystem.write(args).await,
            "fs_move" => self.filesystem.move_file(args).await,
            "fs_copy" => self.filesystem.copy(args).await,
            "fs_create" => self.filesystem.create(args).await,
            "fs_delete" => self.filesystem.delete(args).await,
            "fs_move_desktop" => self.filesystem.move_desktop(args).await,
            "fs_find" => self.filesystem.find(args).await,
            "fs_ld" => self.filesystem.ld(args).await,
            "fs_stat" => self.filesystem.stat(args).await,
            "fs_permissions" => self.filesystem.permissions(args).await,
            "fs_watch" => self.filesystem.watch(args).await,
            "fs_snapshot" => self.filesystem.snapshot(args).await,

            // Diagnostics
            "diagnostics_get" => self.diagnostics.get(args).await,

            // Silent
            "silent_script" => self.silent.script(args).await,
            "silent_resources" => self.silent.resources(args).await,

            // Time
            "time_now" => self.time.now(args).await,
            "time_sleep" => self.time.sleep(args).await,
            "time_schedule" => self.time.schedule(args).await,

            // Network
            "net_fetch" => self.network.fetch(args).await,
            "net_cargo" => self.network.cargo(args).await,
            "net_node" => self.network.node(args).await,
            "net_python" => self.network.python(args).await,
            "net_apt" => self.network.apt(args).await,
            "net_ping" => self.network.ping(args).await,

            // Context
            "ctx_context" => self.context.context(args).await,
            "ctx_compact" => self.context.compact_context(args).await,
            "ctx_remove" => self.context.remove_context(args).await,
            "ctx_token_count" => self.context.token_count(args).await,
            "ctx_memory_store" => self.context.memory_store(args).await,
            "ctx_memory_recall" => self.context.memory_recall(args).await,
            "ctx_estimate_cost" => self.context.estimate_cost(args).await,

            // Git
            "git_status" => self.git.status(args).await,
            "git_diff" => self.git.diff(args).await,
            "git_commit" => self.git.commit(args).await,
            "git_branch" => self.git.branch(args).await,
            "git_checkout" => self.git.checkout(args).await,
            "git_blame" => self.git.blame(args).await,
            "git_log" => self.git.log(args).await,
            "git_tag" => self.git.tag(args).await,

            // Input
            "input_notify" => self.input.notify(args).await,
            "input_prompt" => self.input.prompt_user(args).await,
            "input_select" => self.input.select(args).await,
            "input_progress" => self.input.progress(args).await,
            "input_clipboard_read" => self.input.clipboard_read(args).await,
            "input_clipboard_write" => self.input.clipboard_write(args).await,

            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(self.get_server_info()),
                error: None,
            },
            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(self.list_tools()),
                error: None,
            },
            "tools/call" => {
                let params = request.params.unwrap_or(json!({}));
                let name = params["name"].as_str().unwrap_or("");
                let arguments = params.get("arguments").cloned();

                match self.call_tool(name, arguments).await {
                    Ok(result) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": result.to_string()
                                }
                            ]
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: e.to_string(),
                            data: None,
                        }),
                    },
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut server = PolyMcp::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => {
                let response = server.handle_request(request).await;
                let response_json = serde_json::to_string(&response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
            }
            Err(e) => {
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                let response_json = serde_json::to_string(&error_response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}
