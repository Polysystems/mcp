# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-02-22

### Added

#### Filesystem Module Enhancements (+4 tools)
- **fs_tree** - Visual directory tree with depth control, size display, hidden files, pattern filter
- **fs_grep** - Regex content search across files with context lines and file pattern filter
- **fs_tail** - Read last N lines of a file (default: 20)
- **fs_replace** - Bulk find/replace with regex support, file pattern filter, dry-run mode

#### Filesystem Improvements
- **fs_find** now supports glob pattern matching, max_depth, and max_results
- **fs_read** now returns `total_lines` in output

#### New Module: Clipboard (5 tools)
Session-based copy/paste with tagging — saves tokens by avoiding redundant file reads and content re-generation.

- **clip_copy_file** - Copy text from a file (with optional line ranges) and store with a tag
- **clip_copy** - Copy arbitrary text directly into session clipboard with a tag
- **clip_paste_file** - Paste tagged content into a file (overwrite/append/prepend/line-replace modes)
- **clip_paste** - Retrieve tagged content or list all stored entries
- **clip_clear** - Clear one or all clipboard entries

#### New Module: Transform (7 tools)
Stateless text and data processing utilities for agents.

- **transform_diff** - Compare two texts or files (unified/inline/stats output)
- **transform_encode** - Encode/decode base64, URL, hex, HTML entities
- **transform_hash** - Cryptographic hashing (SHA256, SHA512, MD5, BLAKE3) for text or files
- **transform_regex** - Regex operations: match, find_all, replace, split, extract capture groups
- **transform_json** - JSON manipulation: pretty-print, minify, validate, dot-notation query, merge, keys, flatten
- **transform_text** - Text transformations: case conversion (snake/camel/kebab/title), sort/reverse/unique/trim/number lines, wrap, truncate, stats
- **transform_archive** - Create, extract, and list zip and tar.gz archives

#### Time Module Enhancements (+4 tools)
- **time_timezone** - Convert timestamps between IANA timezones, list available timezones
- **time_stopwatch** - Named stopwatches with start/stop/lap/reset/status/list
- **time_timer** - Countdown timers with duration/unit, check remaining, cancel, list
- **time_alarm** - Set alarms by time or offset, check/cancel/list with lazy trigger detection

### Fixed
- Hardcoded version `"0.1.0"` in `get_server_info()` now uses `env!("CARGO_PKG_VERSION")`
- **ctx_compact**: gzip mode was using ZlibEncoder instead of GzEncoder — now produces valid gzip output
- **silent_script**: timeout parameter was accepted but never enforced — now uses `tokio::time::timeout`
- **silent_script**: predictable temp file names (PID-based) replaced with UUID to prevent race conditions
- **transform_encode**: hex decode no longer panics on odd-length input strings
- **ctx_estimate_cost**: updated pricing for Claude 4.x, GPT-4o, o1/o3-mini models

### Changed
- Total tools: 53 → 73 across 9 → 11 modules
- Updated banner and module listing to reflect new modules and tool counts

### Dependencies Added
- `chrono-tz` 0.10 (IANA timezone database)
- `similar` 2.0 (text diffing)
- `urlencoding` 2.1 (URL encode/decode)
- `sha2` 0.10 (SHA256/SHA512 hashing)
- `md-5` 0.10 (MD5 hashing)
- `blake3` 1.5 (BLAKE3 hashing)
- `regex` 1.10 (regular expressions)
- `tar` 0.4 (tar archive support)

## [0.1.1] - 2026-02-21

### Fixed
- Made urgency calls Linux-only for cross-platform compatibility
- Made gitent-core optional with feature flag for cross-compilation compatibility

## [0.1.0] - 2026-02-20

### Added
- Initial release with 9 modules and 53 tools
- Filesystem (13 tools), Diagnostics (1), Silent (2), Time (3), Network (6), Context (7), Git (8), Input (6), Gitent (7)
- Dual transport: stdio and HTTP server modes
- JSON-RPC 2.0 protocol support
