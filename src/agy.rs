use colored::*;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

use crate::compressors::{self, ContentType};
use crate::tokens::TokenStats;

pub fn install_agy_skill() -> io::Result<()> {
    let Some(home) = std::env::var("HOME").ok().map(PathBuf::from) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "HOME directory not found",
        ));
    };

    let skill_dir = home
        .join(".gemini")
        .join("config")
        .join("plugins")
        .join("token-reducer")
        .join("skills")
        .join("token-reducer");

    fs::create_dir_all(&skill_dir)?;

    let skill_md_content = r#"---
name: token-reducer
description: Summarizes test output, compiler errors, JSON, and diffs with tokenizer counts.
---

# token-reducer

Use this skill whenever running commands with large or verbose terminal outputs (such as `cargo test`, `npm test`, `pytest`, build logs, or git diffs).

## CLI Invocation

```bash
# Pipe any command output into token-reducer
$ npm test 2>&1 | token-reducer pipe

# Or wrap execution directly
$ token-reducer run -- cargo test
```
"#;

    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, skill_md_content)?;

    println!(
        "{} Successfully installed AGY skill to {}",
        "✓".green().bold(),
        skill_path.display()
    );

    Ok(())
}

pub fn analyze_agy_sessions() -> io::Result<()> {
    let Some(home) = std::env::var("HOME").ok().map(PathBuf::from) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "HOME directory not found",
        ));
    };

    let brain_dir = home.join(".gemini").join("antigravity-cli").join("brain");

    if !brain_dir.is_dir() {
        println!(
            "No Antigravity brain sessions found at {}",
            brain_dir.display()
        );
        return Ok(());
    }

    println!(
        "\n{}",
        "=== Antigravity (AGY) Token Reduction Analysis ==="
            .cyan()
            .bold()
    );

    let mut total_orig_tokens = 0usize;
    let mut total_reduced_tokens = 0usize;
    let mut analyzed_steps = 0usize;

    let mut transcripts = Vec::new();
    if let Ok(entries) = fs::read_dir(brain_dir) {
        for entry in entries.flatten() {
            let transcript = entry
                .path()
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            if transcript.is_file() {
                transcripts.push(transcript);
            }
        }
    }

    // Inspect at most five discovered transcripts.
    transcripts.truncate(5);

    for transcript in transcripts {
        if let Ok(file) = File::open(&transcript) {
            for line in BufReader::new(file).lines().flatten() {
                if let Ok(val) = serde_json::from_str::<Value>(&line) {
                    if let Some(content) = val.get("content").and_then(|s| s.as_str()) {
                        if content.len() > 100 && content.len() < 100_000 {
                            let reduced = compressors::compress(content, ContentType::Auto);
                            let stats = TokenStats::compute(content, &reduced);
                            total_orig_tokens += stats.original_tokens;
                            total_reduced_tokens += stats.reduced_tokens;
                            analyzed_steps += 1;
                        }
                    }
                }
            }
        }
    }

    if analyzed_steps == 0 {
        println!("No message content steps found to analyze.");
        return Ok(());
    }

    let saved = total_orig_tokens.saturating_sub(total_reduced_tokens);
    let pct = if total_orig_tokens > 0 {
        (saved as f64 / total_orig_tokens as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "Analyzed {} local AGY transcript steps",
        analyzed_steps.to_string().yellow().bold()
    );
    println!("Original Tokens:  {}", total_orig_tokens.to_string().red());
    println!(
        "Reduced Tokens:   {}",
        total_reduced_tokens.to_string().green()
    );
    println!(
        "Tokens Saved:     {} ({:.1}% reduction)",
        saved.to_string().cyan().bold(),
        pct
    );
    println!();

    Ok(())
}
