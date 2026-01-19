# Poly MCP

A comprehensive MCP (Model Context Protocol) server with extensive tooling for filesystem operations, diagnostics, scripting, time management, network utilities, context handling, git operations, and user input.

## Features

### 1. Filesystem Module

Advanced file and directory operations with snapshot management:

- **fs_read** - Read file contents
- **fs_write** - Write content to files
- **fs_move** - Move files or directories
- **fs_copy** - Copy files or directories recursively
- **fs_create** - Create files or directories
- **fs_delete** - Delete files or directories
- **fs_move_desktop** - Organize items within Desktop directory
- **fs_find** - Search for files with pattern matching
- **fs_ld** - Detailed directory listing (like ls -la)
- **fs_stat** - Get file/directory metadata
- **fs_permissions** - Get or set Unix file permissions
- **fs_watch** - Monitor file/directory changes
- **fs_snapshot** - Create timestamped backups with auto-management

### 2. Diagnostics Module

Language-agnostic error and warning detection:

- **diagnostics_get** - Get errors/warnings for files or projects
- Auto-detects appropriate diagnostic tool (cargo, tsc, eslint, pylint, etc.)
- Supports Rust, TypeScript/JavaScript, Python, C/C++
- Parses compiler/linter output into structured JSON

### 3. Silent Module

Bash scripting and system resource monitoring:

- **silent_script** - Execute bash scripts with arguments, env vars, and timeout
- **silent_resources** - Monitor GPU/RAM/CPU usage with detailed process info
- Supports nvidia-smi for GPU monitoring
- Process filtering and sorting by resource usage

### 4. Time Module

Time management and task scheduling:

- **time_now** - Get current timestamp in multiple formats (Unix, ISO8601, RFC3339, custom)
- **time_sleep** - Delay execution with configurable duration
- **time_schedule** - In-memory task scheduler with create/cancel/list/status operations

### 5. Network Module

HTTP requests and package registry queries:

- **net_fetch** - Fetch URLs with automatic HTML to Markdown conversion
- **net_cargo** - Query crates.io for Rust package info
- **net_node** - Query npm registry for Node.js packages
- **net_python** - Query PyPI for Python packages
- **net_apt** - Query APT package information
- **net_ping** - Check network connectivity with statistics

### 6. Context Module

Token counting and context management for LLMs:

- **ctx_context** - Track token usage (total/used/left)
- **ctx_compact** - Compress text using zlib/gzip algorithms
- **ctx_remove** - Clear context and reset usage
- **ctx_token_count** - Count tokens for various LLM providers (GPT-4, Claude, etc.)
- **ctx_memory_store** - Store data in-memory (process lifetime)
- **ctx_memory_recall** - Retrieve stored data
- **ctx_estimate_cost** - Estimate API costs for Anthropic, OpenAI, Ollama, GLM

### 7. Git Module

Complete git operations via libgit2:

- **git_status** - Repository status with staged/unstaged/untracked files
- **git_diff** - View changes with patch format
- **git_commit** - Create commits
- **git_branch** - List, create, or delete branches
- **git_checkout** - Switch branches or commits
- **git_blame** - Show line-by-line authorship
- **git_log** - View commit history
- **git_tag** - Manage tags (lightweight and annotated)

### 8. Input Module

User interaction and notifications:

- **input_notify** - Send terminal and desktop notifications
- **input_prompt** - Interactive text prompts (terminal or MCP)
- **input_select** - Selection menus (terminal or MCP)
- **input_progress** - Display progress bars
- **input_clipboard_read** - Read from system clipboard
- **input_clipboard_write** - Write to system clipboard

## Installation

```bash
cargo add poly-mcp
```

## Usage

Run the MCP server:

```bash
poly-mcp
```

The server communicates via JSON-RPC 2.0 over stdin/stdout following the MCP protocol.

### MCP Protocol Messages

**Initialize:**
```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}
```

**List Tools:**
```json
{"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}
```

**Call Tool:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "fs_read",
    "arguments": {
      "path": "/path/to/file.txt"
    }
  }
}
```

## Example Tool Calls

### Read a File
```json
{
  "name": "fs_read",
  "arguments": {"path": "/etc/hosts"}
}
```

### Create a Snapshot
```json
{
  "name": "fs_snapshot",
  "arguments": {
    "path": "/important/project",
    "max_snapshots": 5
  }
}
```

### Run Diagnostics
```json
{
  "name": "diagnostics_get",
  "arguments": {"path": "./src"}
}
```

### Execute Bash Script
```json
{
  "name": "silent_script",
  "arguments": {
    "script": "#!/bin/bash\necho 'Hello World'\nls -la",
    "timeout": 30
  }
}
```

### Monitor Resources
```json
{
  "name": "silent_resources",
  "arguments": {
    "detailed": true,
    "process_filter": "rust"
  }
}
```

### Fetch URL as Markdown
```json
{
  "name": "net_fetch",
  "arguments": {
    "url": "https://example.com",
    "convert_to_markdown": true
  }
}
```

### Get Latest Package Version
```json
{
  "name": "net_cargo",
  "arguments": {
    "crate_name": "tokio",
    "action": "latest"
  }
}
```

### Count Tokens
```json
{
  "name": "ctx_token_count",
  "arguments": {
    "text": "Your text here",
    "model": "gpt-4"
  }
}
```

### Estimate API Cost
```json
{
  "name": "ctx_estimate_cost",
  "arguments": {
    "provider": "anthropic",
    "model": "claude-3-opus",
    "input_tokens": 1000,
    "output_tokens": 500
  }
}
```

### Git Operations
```json
{
  "name": "git_status",
  "arguments": {"path": "."}
}
```

```json
{
  "name": "git_commit",
  "arguments": {
    "message": "feat: add new feature",
    "author_name": "Developer",
    "author_email": "dev@example.com"
  }
}
```

### Send Notification
```json
{
  "name": "input_notify",
  "arguments": {
    "title": "Build Complete",
    "message": "Your project has been built successfully!",
    "type": "both",
    "urgency": "normal"
  }
}
```

### Clipboard Operations
```json
{
  "name": "input_clipboard_write",
  "arguments": {"content": "Hello from Poly MCP!"}
}
```

## License

Licensed under the MIT License.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.
