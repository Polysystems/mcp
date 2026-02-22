use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use anyhow::Result;
use clap::Parser;
use is_terminal::IsTerminal;
use tokio::sync::Mutex;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use tower_http::cors::CorsLayer;

mod modules;
use modules::{
    clipboard::ClipboardModule,
    filesystem::FilesystemModule,
    diagnostics::DiagnosticsModule,
    silent::SilentModule,
    time::TimeModule,
    network::NetworkModule,
    context::ContextModule,
    git::GitModule,
    input::InputModule,
    transform::TransformModule,
};
#[cfg(feature = "gitent")]
use modules::gitent::GitentModule;

/// Poly MCP - A comprehensive Model Context Protocol server
///
/// Provides 11 powerful modules for AI assistants:
/// • Filesystem - File operations, snapshots, permissions
/// • Diagnostics - Multi-language error detection
/// • Silent - Bash scripting & resource monitoring
/// • Time - Scheduling, timezones, stopwatch, timer, alarm
/// • Network - HTTP requests & package queries
/// • Context - Token counting & cost estimation
/// • Git - Complete git operations via libgit2
/// • Input - User interaction & notifications
/// • Gitent - Agent-centric version control tracking
/// • Clipboard - Session copy/paste with tags
/// • Transform - Diff, encode, hash, regex, JSON, text, archive
#[derive(Parser, Debug)]
#[command(name = "poly-mcp")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A comprehensive MCP server with 11 powerful modules", long_about = None)]
struct Cli {
    /// List all available modules and their tools
    #[arg(short, long)]
    list_modules: bool,

    /// Show verbose startup information
    #[arg(short, long)]
    verbose: bool,

    /// Run as HTTP server instead of stdio mode
    #[arg(short, long)]
    server: bool,

    /// Port to bind HTTP server to (default: 3000)
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Host to bind HTTP server to (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

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
    #[cfg(feature = "gitent")]
    gitent: GitentModule,
    clipboard: ClipboardModule,
    transform: TransformModule,
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
            #[cfg(feature = "gitent")]
            gitent: GitentModule::new(),
            clipboard: ClipboardModule::new(),
            transform: TransformModule::new(),
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
                "version": env!("CARGO_PKG_VERSION")
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

        // Gitent tools
        #[cfg(feature = "gitent")]
        tools.extend(self.gitent.get_tools());

        // Clipboard tools
        tools.extend(self.clipboard.get_tools());

        // Transform tools
        tools.extend(self.transform.get_tools());

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
            "fs_tree" => self.filesystem.tree(args).await,
            "fs_grep" => self.filesystem.grep(args).await,
            "fs_tail" => self.filesystem.tail(args).await,
            "fs_replace" => self.filesystem.replace(args).await,

            // Diagnostics
            "diagnostics_get" => self.diagnostics.get(args).await,

            // Silent
            "silent_script" => self.silent.script(args).await,
            "silent_resources" => self.silent.resources(args).await,

            // Time
            "time_now" => self.time.now(args).await,
            "time_sleep" => self.time.sleep(args).await,
            "time_schedule" => self.time.schedule(args).await,
            "time_timezone" => self.time.timezone(args).await,
            "time_stopwatch" => self.time.stopwatch(args).await,
            "time_timer" => self.time.timer(args).await,
            "time_alarm" => self.time.alarm(args).await,

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

            // Gitent
            #[cfg(feature = "gitent")]
            "gitent_init" => self.gitent.init(args).await,
            #[cfg(feature = "gitent")]
            "gitent_status" => self.gitent.status(args).await,
            #[cfg(feature = "gitent")]
            "gitent_track" => self.gitent.track(args).await,
            #[cfg(feature = "gitent")]
            "gitent_commit" => self.gitent.commit(args).await,
            #[cfg(feature = "gitent")]
            "gitent_log" => self.gitent.log(args).await,
            #[cfg(feature = "gitent")]
            "gitent_diff" => self.gitent.diff(args).await,
            #[cfg(feature = "gitent")]
            "gitent_rollback" => self.gitent.rollback(args).await,

            // Clipboard
            "clip_copy_file" => self.clipboard.copy_file(args).await,
            "clip_copy" => self.clipboard.copy(args).await,
            "clip_paste_file" => self.clipboard.paste_file(args).await,
            "clip_paste" => self.clipboard.paste(args).await,
            "clip_clear" => self.clipboard.clear(args).await,

            // Transform
            "transform_diff" => self.transform.diff(args).await,
            "transform_encode" => self.transform.encode(args).await,
            "transform_hash" => self.transform.hash(args).await,
            "transform_regex" => self.transform.regex_op(args).await,
            "transform_json" => self.transform.json_op(args).await,
            "transform_text" => self.transform.text(args).await,
            "transform_archive" => self.transform.archive(args).await,

            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    fn print_banner(&self, verbose: bool) {
        eprintln!("\n╭────────────────────────────────────────────────────╮");
        eprintln!("│         🔧 Poly MCP Server v{}              │", env!("CARGO_PKG_VERSION"));
        eprintln!("╰────────────────────────────────────────────────────╯\n");

        eprintln!("📡 Protocol: Model Context Protocol (MCP)");
        eprintln!("🔗 Transport: stdio (stdin/stdout) - no network port");
        eprintln!("📋 Format: JSON-RPC 2.0");
        eprintln!("📦 Modules: 11 active modules loaded\n");

        if verbose {
            eprintln!("Available Modules:");
            eprintln!("  • Filesystem    - 17 tools for file operations");
            eprintln!("  • Diagnostics   - 1 tool for error detection");
            eprintln!("  • Silent        - 2 tools for scripting & monitoring");
            eprintln!("  • Time          - 7 tools for scheduling & timekeeping");
            eprintln!("  • Network       - 6 tools for HTTP & packages");
            eprintln!("  • Context       - 7 tools for token management");
            eprintln!("  • Git           - 8 tools for version control");
            eprintln!("  • Input         - 6 tools for user interaction");
            eprintln!("  • Gitent        - 7 tools for agent tracking");
            eprintln!("  • Clipboard     - 5 tools for session copy/paste");
            eprintln!("  • Transform     - 7 tools for text/data processing\n");
        }

        eprintln!("✓ Server ready and listening for JSON-RPC requests...");
        eprintln!("ℹ Use --help for more information\n");
    }

    fn list_all_modules(&self) {
        println!("\n╭────────────────────────────────────────────────────╮");
        println!("│         🔧 Poly MCP - Available Modules           │");
        println!("╰────────────────────────────────────────────────────╯\n");

        let modules = vec![
            ("Filesystem", "File and directory operations", vec![
                "fs_read", "fs_write", "fs_move", "fs_copy", "fs_create", "fs_delete",
                "fs_move_desktop", "fs_find", "fs_ld", "fs_stat", "fs_permissions",
                "fs_watch", "fs_snapshot", "fs_tree", "fs_grep", "fs_tail", "fs_replace"
            ]),
            ("Diagnostics", "Language-agnostic error detection", vec![
                "diagnostics_get"
            ]),
            ("Silent", "Bash scripting and resource monitoring", vec![
                "silent_script", "silent_resources"
            ]),
            ("Time", "Time management, scheduling & timekeeping", vec![
                "time_now", "time_sleep", "time_schedule",
                "time_timezone", "time_stopwatch", "time_timer", "time_alarm"
            ]),
            ("Network", "HTTP requests and package queries", vec![
                "net_fetch", "net_cargo", "net_node", "net_python", "net_apt", "net_ping"
            ]),
            ("Context", "Token counting and cost estimation", vec![
                "ctx_context", "ctx_compact", "ctx_remove", "ctx_token_count",
                "ctx_memory_store", "ctx_memory_recall", "ctx_estimate_cost"
            ]),
            ("Git", "Complete git operations", vec![
                "git_status", "git_diff", "git_commit", "git_branch",
                "git_checkout", "git_blame", "git_log", "git_tag"
            ]),
            ("Input", "User interaction and notifications", vec![
                "input_notify", "input_prompt", "input_select", "input_progress",
                "input_clipboard_read", "input_clipboard_write"
            ]),
            ("Gitent", "Agent-centric version control tracking", vec![
                "gitent_init", "gitent_status", "gitent_track", "gitent_commit",
                "gitent_log", "gitent_diff", "gitent_rollback"
            ]),
            ("Clipboard", "Session copy/paste with tags", vec![
                "clip_copy_file", "clip_copy", "clip_paste_file", "clip_paste", "clip_clear"
            ]),
            ("Transform", "Text & data processing", vec![
                "transform_diff", "transform_encode", "transform_hash", "transform_regex",
                "transform_json", "transform_text", "transform_archive"
            ]),
        ];

        for (name, description, tools) in modules {
            println!("📦 {} - {}", name, description);
            println!("   {} tools: {}", tools.len(), tools.join(", "));
            println!();
        }

        println!("Total: 73 tools across 11 modules\n");
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

// Shared state type for HTTP server
type SharedState = Arc<Mutex<PolyMcp>>;

// HTTP handler for JSON-RPC requests
async fn handle_jsonrpc(
    State(state): State<SharedState>,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    let mut server = state.lock().await;
    let response = server.handle_request(request).await;
    Json(response).into_response()
}

// HTTP handler for health check
async fn health_check() -> Response {
    Json(json!({
        "status": "healthy",
        "service": "poly-mcp",
        "version": env!("CARGO_PKG_VERSION")
    }))
    .into_response()
}

// Run server in stdio mode (original behavior)
async fn run_stdio_mode(cli: &Cli) -> Result<()> {
    let mut server = PolyMcp::new();

    // Only print startup banner if stdin is a terminal (interactive mode)
    if io::stdin().is_terminal() {
        server.print_banner(cli.verbose);
    }

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

// Run server in HTTP mode
async fn run_http_mode(cli: &Cli) -> Result<()> {
    let server = PolyMcp::new();
    let state = Arc::new(Mutex::new(server));

    // Build HTTP router
    let app = Router::new()
        .route("/", post(handle_jsonrpc))
        .route("/jsonrpc", post(handle_jsonrpc))
        .route("/health", axum::routing::get(health_check))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    eprintln!("\n╭────────────────────────────────────────────────────╮");
    eprintln!("│         🔧 Poly MCP Server v{}              │", env!("CARGO_PKG_VERSION"));
    eprintln!("╰────────────────────────────────────────────────────╯\n");
    eprintln!("📡 Protocol: Model Context Protocol (MCP)");
    eprintln!("🔗 Transport: HTTP (JSON-RPC 2.0)");
    eprintln!("🌐 Address: http://{}", addr);
    eprintln!("📦 Modules: 11 active modules loaded");
    eprintln!("💚 Health: http://{}/health\n", addr);

    if cli.verbose {
        eprintln!("Available Modules:");
        eprintln!("  • Filesystem    - 17 tools for file operations");
        eprintln!("  • Diagnostics   - 1 tool for error detection");
        eprintln!("  • Silent        - 2 tools for scripting & monitoring");
        eprintln!("  • Time          - 7 tools for scheduling & timekeeping");
        eprintln!("  • Network       - 6 tools for HTTP & packages");
        eprintln!("  • Context       - 7 tools for token management");
        eprintln!("  • Git           - 8 tools for version control");
        eprintln!("  • Input         - 6 tools for user interaction");
        eprintln!("  • Gitent        - 7 tools for agent tracking");
        eprintln!("  • Clipboard     - 5 tools for session copy/paste");
        eprintln!("  • Transform     - 7 tools for text/data processing\n");
    }

    eprintln!("✓ Server ready and listening for HTTP requests...");
    eprintln!("ℹ Press Ctrl+C to stop\n");

    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Handle --list-modules flag
    if cli.list_modules {
        let server = PolyMcp::new();
        server.list_all_modules();
        return Ok(());
    }

    // Choose mode based on CLI flags
    if cli.server {
        run_http_mode(&cli).await
    } else {
        run_stdio_mode(&cli).await
    }
}
