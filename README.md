# token-reducer

[![CI](https://github.com/yuri-rod/token-reducer/actions/workflows/ci.yml/badge.svg)](https://github.com/yuri-rod/token-reducer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/yuri-rod/token-reducer?color=blue)](https://github.com/yuri-rod/token-reducer/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

**High-performance token and context compressor for coding agents, AGY, and developer CLI tools.**

`token-reducer` shrinks compiler logs, test outputs, AST git diffs, and large JSON payloads by **60% to 98%**, cutting LLM token bills and preventing context dilution for tools like Codex, Claude Code, Google Antigravity (AGY), Aider, and Cursor.

```bash
# Wrap command execution with real-time compression & token stats
$ token-reducer run -- cargo test
$ token-reducer run -- npm test
$ token-reducer run -- pytest

# Pipe raw noisy terminal outputs
$ git diff | token-reducer pipe --type diff
$ cat big_response.json | token-reducer pipe --type json

# Start Model Context Protocol (MCP) server for AGY, Claude, and Codex
$ token-reducer mcp

# Install as native Google Antigravity (AGY) skill
$ token-reducer agy install
```

---

## Benchmark Results

| Workload | Raw Tokens | Reduced Tokens | Reduction | Description |
| :--- | :---: | :---: | :---: | :--- |
| **Lockfile Git Diff** (`Cargo.lock` / `package-lock.json`) | **3,269** | **63** | **98.1%** | Collapses dependency lockfile hash spam into compact summary line. |
| **JSON Array Payload** (50 records) | **1,203** | **111** | **90.8%** | Preserves schema and boundary items, collapses repetitive middle. |
| **Test Suite Log** (150 passing tests + 1 failure) | **266** | **157** | **41.0%** | Omits passing test noise and internal runtime stack frames. |

---

## Features

1. **Test Suite & Compiler Squeezer**:
   - Strips ANSI color sequences and carriage return noise.
   - Collapses passing test lines while highlighting root failures and file:line locations.
   - Truncates repetitive runtime stack trace frames (`node:internal/`, `core::ops::function`).
2. **AST & Lockfile Diff Pruner**:
   - Compresses large dependency lockfile diffs into a 1-line summary.
   - Trims bloated unchanged context lines down to essential 2-line anchors.
3. **Structured JSON Reducer**:
   - Preserves typed object shapes and boundary examples while omitting array bloat.
4. **First-Class AGY & MCP Support**:
   - JSON-RPC 2.0 stdio MCP server for Claude Desktop, Codex, and Cursor.
   - Automatic skill installer for Google Antigravity (AGY).

---

## Installation

### Via Cargo

```bash
cargo install --git https://github.com/yuri-rod/token-reducer.git
```

### From Source

```bash
git clone https://github.com/yuri-rod/token-reducer.git
cd token-reducer
cargo build --release
cp target/release/token-reducer /usr/local/bin/
```

---

## Model Context Protocol (MCP) Configuration

Add `token-reducer` to your `claude_desktop_config.json` or MCP settings:

```json
{
  "mcpServers": {
    "token-reducer": {
      "command": "token-reducer",
      "args": ["mcp"]
    }
  }
}
```

---

## License

MIT License.
