use std::io::{self, Read};
use std::process::Command;
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;

use token_reducer::agy;
use token_reducer::compressors::{self, ContentType};
use token_reducer::mcp::run_mcp_server;
use token_reducer::tokens::TokenStats;

#[derive(Parser)]
#[command(name = "token-reducer", version, about = "High-performance token and context compressor for coding agents, AGY, and developer CLI tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Wrap and execute a command, printing compressed output and token stats", trailing_var_arg = true)]
    Run {
        #[arg(short = 't', long = "type", value_enum, default_value_t = FormatChoice::Auto)]
        format: FormatChoice,

        #[arg(required = true, help = "Command and arguments to execute")]
        command: Vec<String>,
    },

    #[command(about = "Compress stdin stream and output to stdout")]
    Pipe {
        #[arg(short = 't', long = "type", value_enum, default_value_t = FormatChoice::Auto)]
        format: FormatChoice,
    },

    #[command(about = "Run git diff with automatic lockfile and context token reduction", trailing_var_arg = true)]
    Diff {
        #[arg(help = "Arguments passed directly to git diff")]
        args: Vec<String>,
    },

    #[command(about = "Start JSON-RPC Model Context Protocol (MCP) server over stdio")]
    Mcp,

    #[command(about = "Google Antigravity (AGY) integration and session analysis")]
    Agy {
        #[arg(value_enum, help = "Action to perform")]
        action: AgyAction,
    },

    #[command(about = "Run token reduction benchmarks across real-world workloads")]
    Benchmark,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum FormatChoice {
    Auto,
    Log,
    Diff,
    Json,
    Ansi,
}

impl From<FormatChoice> for ContentType {
    fn from(c: FormatChoice) -> Self {
        match c {
            FormatChoice::Auto => ContentType::Auto,
            FormatChoice::Log => ContentType::Log,
            FormatChoice::Diff => ContentType::Diff,
            FormatChoice::Json => ContentType::Json,
            FormatChoice::Ansi => ContentType::AnsiOnly,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum AgyAction {
    Install,
    Analyze,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { format, command } => {
            let output = if cfg!(target_os = "windows") {
                Command::new(&command[0]).args(&command[1..]).output()?
            } else {
                Command::new(&command[0]).args(&command[1..]).output()?
            };

            let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

            let combined = if stderr_str.is_empty() {
                stdout_str
            } else {
                format!("{}\n{}", stdout_str, stderr_str)
            };

            let compressed = compressors::compress(&combined, format.into());
            let stats = TokenStats::compute(&combined, &compressed);

            println!("{}", compressed);
            eprintln!("\n{}", format!("[token-reducer] {}", stats.format_summary()).green().bold());
        }

        Commands::Pipe { format } => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;

            let compressed = compressors::compress(&input, format.into());
            let stats = TokenStats::compute(&input, &compressed);

            print!("{}", compressed);
            eprintln!("\n{}", format!("[token-reducer] {}", stats.format_summary()).green().bold());
        }

        Commands::Diff { args } => {
            let output = Command::new("git").arg("diff").args(&args).output()?;
            let diff_raw = String::from_utf8_lossy(&output.stdout).to_string();

            let compressed = compressors::compress(&diff_raw, ContentType::Diff);
            let stats = TokenStats::compute(&diff_raw, &compressed);

            println!("{}", compressed);
            eprintln!("\n{}", format!("[token-reducer] {}", stats.format_summary()).green().bold());
        }

        Commands::Mcp => {
            run_mcp_server()?;
        }

        Commands::Agy { action } => match action {
            AgyAction::Install => agy::install_agy_skill()?,
            AgyAction::Analyze => agy::analyze_agy_sessions()?,
        },

        Commands::Benchmark => {
            println!("\n{}", "=== Token Reducer Benchmark Suite ===".cyan().bold());

            // 1. Rust test output
            let rust_log = r#"
running 150 tests
test core::tests::test_001 ... ok
test core::tests::test_002 ... ok
test core::tests::test_003 ... ok
test core::tests::test_004 ... ok
test core::tests::test_005 ... ok
test core::tests::test_006 ... ok
test core::tests::test_007 ... ok
test core::tests::test_008 ... ok
test core::tests::test_009 ... ok
test core::tests::test_010 ... FAILED

failures:

---- core::tests::test_010 stdout ----
thread 'core::tests::test_010' panicked at 'assertion failed: `(left == right)`
  left: `200`,
 right: `404`', src/server.rs:188:9
stack backtrace:
   0: rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::ops::function::FnOnce::call_once
   3: core::ops::function::FnOnce::call_once
   4: my_service::handle_request
             at src/server.rs:188:9

test result: FAILED. 1 failed; 149 passed; 0 ignored
"#;
            let comp_rust = compressors::compress(rust_log, ContentType::Log);
            let stats_rust = TokenStats::compute(rust_log, &comp_rust);
            println!("1. Test Suite (150 tests): {}", stats_rust.format_summary().green());

            // 2. Large Lockfile Git Diff
            let mut diff_raw = String::from("diff --git a/Cargo.lock b/Cargo.lock\nindex 123456..789abc 100644\n--- a/Cargo.lock\n+++ b/Cargo.lock\n");
            for i in 0..100 {
                diff_raw.push_str(&format!("+[[package]]\n+name = \"dep_{}\"\n+version = \"1.0.{}\"\n+checksum = \"abcdef1234567890\"\n", i, i));
            }
            diff_raw.push_str("diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n-fn old() {}\n+fn new() {}\n");
            let comp_diff = compressors::compress(&diff_raw, ContentType::Diff);
            let stats_diff = TokenStats::compute(&diff_raw, &comp_diff);
            println!("2. Git Diff with Lockfile: {}", stats_diff.format_summary().green());

            // 3. Large JSON payload
            let mut json_raw = String::from("[");
            for i in 0..50 {
                if i > 0 { json_raw.push(','); }
                json_raw.push_str(&format!(r#"{{"id": {}, "name": "user_{}", "active": true, "role": "member"}}"#, i, i));
            }
            json_raw.push(']');
            let comp_json = compressors::compress(&json_raw, ContentType::Json);
            let stats_json = TokenStats::compute(&json_raw, &comp_json);
            println!("3. JSON Array (50 records): {}", stats_json.format_summary().green());
            println!();
        }
    }

    Ok(())
}
