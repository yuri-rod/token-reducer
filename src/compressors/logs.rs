use std::borrow::Cow;

use super::ansi::strip_ansi;

pub fn compress_logs(input: &str) -> String {
    let clean = strip_ansi(input);

    let mut output_lines: Vec<Cow<'_, str>> = Vec::new();
    let mut omitted_passing_tests = 0;
    let mut omitted_internal_frames = 0;
    let mut warning_count = 0;

    let mut in_failure_block = false;

    for line in clean.lines() {
        let trimmed = line.trim();

        // 1. Filter passing test noise
        if (trimmed.starts_with("test ") && trimmed.ends_with("... ok"))
            || (trimmed.starts_with("PASS ")
                && (trimmed.ends_with(".js") || trimmed.ends_with(".ts")))
            || (trimmed.starts_with("✓ ") || trimmed.starts_with("✔ "))
            || (trimmed.starts_with("=== RUN ") || (trimmed.starts_with("--- PASS:")))
        {
            omitted_passing_tests += 1;
            continue;
        }

        // 2. Detect failure sections
        if trimmed.starts_with("FAIL ")
            || trimmed.starts_with("FAILED ")
            || trimmed.starts_with("error[")
            || trimmed.starts_with("Error:")
            || trimmed.starts_with("AssertionError:")
            || trimmed.starts_with("failures:")
            || trimmed.starts_with("Traceback (most recent call last):")
        {
            in_failure_block = true;
        }

        // 3. Compress compiler warnings
        if trimmed.starts_with("warning:") || trimmed.starts_with("warning[") {
            warning_count += 1;
            if warning_count <= 2 {
                output_lines.push(line.into());
            }
            continue;
        }

        // 4. Collapse deep internal runtime stack frames
        let is_internal_frame = trimmed.contains("node:internal/")
            || trimmed.contains("core::ops::function::FnOnce")
            || trimmed.contains("std::panicking::")
            || trimmed.contains("site-packages/")
            || trimmed.contains("node_modules/jest-")
            || trimmed.contains("node_modules/@jest");

        if is_internal_frame && in_failure_block {
            omitted_internal_frames += 1;
            continue;
        } else if omitted_internal_frames > 0 {
            output_lines.push(
                format!(
                    "    [... {} internal runtime stack frames omitted ...]",
                    omitted_internal_frames
                )
                .into(),
            );
            omitted_internal_frames = 0;
        }

        // Skip excessive empty lines
        if trimmed.is_empty() && output_lines.last().is_some_and(|l| l.trim().is_empty()) {
            continue;
        }

        output_lines.push(line.into());
    }

    if omitted_internal_frames > 0 {
        output_lines.push(
            format!(
                "    [... {} internal runtime stack frames omitted ...]",
                omitted_internal_frames
            )
            .into(),
        );
    }

    let mut result = String::new();
    if omitted_passing_tests > 0 {
        result.push_str(&format!(
            "[token-reducer] Collapsed {} passing test lines\n",
            omitted_passing_tests
        ));
    }
    if warning_count > 2 {
        result.push_str(&format!(
            "[token-reducer] + {} additional compiler warnings omitted\n",
            warning_count - 2
        ));
    }

    result.push_str(&output_lines.join("\n"));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_typescript_failures_and_source_paths() {
        let raw = "PASS src/ok.ts\nFAIL src/broken.ts\nError: could not read src/config.ts\n  at src/worker.ts\n";
        let compressed = compress_logs(raw);
        assert!(!compressed.contains("PASS src/ok.ts"));
        assert!(compressed.contains("FAIL src/broken.ts"));
        assert!(compressed.contains("Error: could not read src/config.ts"));
        assert!(compressed.contains("at src/worker.ts"));
        assert!(compressed.contains("Collapsed 1 passing test lines"));
    }

    #[test]
    fn test_compress_logs() {
        let raw = r#"
running 10 tests
test core::tests::test_a ... ok
test core::tests::test_b ... ok
test core::tests::test_c ... ok
test core::tests::test_d ... FAILED
test core::tests::test_e ... ok

failures:

---- core::tests::test_d stdout ----
thread 'core::tests::test_d' panicked at 'assertion failed: `(left == right)`
  left: `1`,
 right: `2`', src/core.rs:42:9
stack backtrace:
   0: rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::ops::function::FnOnce::call_once
   3: core::ops::function::FnOnce::call_once
   4: std::sys::backtrace::__rust_begin_short_backtrace
   5: my_app::run_logic
             at src/core.rs:42:9

test result: FAILED. 1 failed; 9 passed; 0 ignored
"#;
        let compressed = compress_logs(raw);
        assert!(!compressed.contains("test core::tests::test_a ... ok"));
        assert!(compressed.contains("test core::tests::test_d ... FAILED"));
        assert!(compressed.contains("assertion failed"));
        assert!(compressed.contains("my_app::run_logic"));
        assert!(compressed.contains("Collapsed 4 passing test lines"));
    }
}
