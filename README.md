# token-reducer

Summarize command output, logs, JSON arrays, and Git diffs before adding them to a coding-agent prompt.

Use it when a build produces hundreds of passing test lines but you need the failure, or when a lockfile diff obscures the source changes. Compression is deliberately lossy. Keep the original output when you need a complete diagnostic record or an applicable patch.

## Install

With [Rust stable](https://www.rust-lang.org/tools/install) installed:

```sh
cargo install --locked --git https://github.com/yuri-rod/token-reducer.git
```

To develop locally:

```sh
git clone https://github.com/yuri-rod/token-reducer.git
cd token-reducer
cargo build --locked
```

The development binary is `target/debug/token-reducer`.

## Try it

This synthetic example preserves a TypeScript failure while collapsing a passing test:

```sh
printf 'PASS src/ok.ts\nFAIL src/broken.ts\nError: missing config.ts\n' | token-reducer pipe --type log
```

Compressed text goes to stdout:

```text
[token-reducer] Collapsed 1 passing test lines
FAIL src/broken.ts
Error: missing config.ts
```

Token and byte counts go to stderr. For a command that edits or reads your project:

```sh
token-reducer run -- cargo test
git diff | token-reducer pipe --type diff
token-reducer diff -- --stat
```

`run` waits for the child to finish, combines stdout followed by stderr, and then compresses it. It preserves the child's exit code; termination without an exit code returns 1. `diff` preserves Git's status and error output, including `git diff --exit-code` returning 1 when changes exist. These wrappers do not stream output or preserve ordering between stdout and stderr.

## Reduction modes

| Mode | Behavior |
| --- | --- |
| `log` | Strip ANSI escapes; collapse recognized passing-test lines, repeated empty lines, some internal stack frames, and warning headers after the first two |
| `diff` | Summarize recognized lockfile and minified-file sections with added/deleted line counts; retain other diff text |
| `json` | For arrays longer than five items, retain the first two and last item and insert an omission string; recurse into objects and retained arrays |
| `ansi` | Remove ANSI escape sequences and carriage-return characters without applying the other reducers |
| `auto` | Choose a reducer from the beginning and ending of the input; use an explicit mode when its guess is unsuitable |

Diff handling is text-based. JSON output is a summary, not a replacement API response: omission strings can change an array's element types, and omitted entries need not be similar. Small JSON documents can become larger after pretty-printing. Invalid JSON is returned unchanged.

[Token statistics](src/tokens.rs) use `cl100k_base`, with a bytes-divided-by-four fallback if loading that tokenizer fails. These counts are not a billing estimate for every model. Savings depend on the input; the tool's summary reports zero savings rather than a negative percentage when output grows.

## MCP over stdio

Configure a local MCP client to launch the installed executable:

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

The server provides `reduce_text` and `reduce_exec`. `reduce_exec` runs a shell command with the server process's permissions and reports command failure with `isError: true`. Enable it only for clients allowed to execute local commands. There is no sandbox, timeout, or output-size limit. The server accepts newline-delimited JSON-RPC over stdin/stdout and advertises protocol version `2024-11-05`; compatibility with a particular client must be checked there.

The optional `agy install` command writes or replaces its skill file under the user's Gemini plugin directory. `agy analyze` reads up to five discovered local transcript files. Neither command is required for the CLI or synthetic examples above.

## Implementation and tests

- [Reducers](src/compressors) contain the log, diff, JSON, and ANSI transformations and their unit tests.
- [CLI execution](src/main.rs) captures command output and preserves process outcomes.
- [Command regressions](tests/command_failures.rs) check successful and failed child commands, Git errors, and MCP execution failures using synthetic inputs.
- [MCP transport](src/mcp.rs) exposes the same reducers over stdio.

```sh
cargo check --locked
cargo test --locked
cargo run --locked --release -- benchmark
```

`benchmark` runs three built-in synthetic samples and prints their tokenizer counts. It does not measure throughput or represent production workloads. CI runs the checks on Linux and macOS. Contributions should include a failing example for a reduction bug and preserve diagnostics users need to understand failures.

The manifest declares the MIT license. This checkout currently has no standalone license file.
