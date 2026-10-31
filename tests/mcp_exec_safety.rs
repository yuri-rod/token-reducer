use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use token_reducer::executor::{execute_bounded, ExecConfig};

#[test]
fn normal_command_completion() {
    let config = ExecConfig {
        timeout: Duration::from_secs(5),
        max_output_bytes: 1024 * 1024,
    };
    let result = execute_bounded("echo 'hello safety test'", &config);
    assert!(!result.failed);
    assert!(!result.timed_out);
    assert!(!result.overflowed);
    assert_eq!(result.exit_code, Some(0));
    assert!(result.combined.contains("hello safety test"));
}

#[test]
fn failing_command_preserves_exit_code() {
    let config = ExecConfig {
        timeout: Duration::from_secs(5),
        max_output_bytes: 1024 * 1024,
    };
    let result = execute_bounded("echo 'failing step' && exit 42", &config);
    assert!(result.failed);
    assert_eq!(result.exit_code, Some(42));
    assert!(result.combined.contains("failing step"));
}

#[cfg(unix)]
#[test]
fn command_timeout_terminates_process_group() {
    let config = ExecConfig {
        timeout: Duration::from_millis(300),
        max_output_bytes: 1024 * 1024,
    };
    let start = Instant::now();
    let result = execute_bounded("sleep 10", &config);
    let elapsed = start.elapsed();

    assert!(result.timed_out);
    assert!(result.failed);
    assert!(elapsed < Duration::from_secs(2));
    assert!(result
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("timed out"));
}

#[cfg(unix)]
#[test]
fn oversized_output_terminates_and_truncates() {
    let config = ExecConfig {
        timeout: Duration::from_secs(5),
        max_output_bytes: 20 * 1024, // 20 KiB limit
    };
    // Generate ~100 KiB of output
    let result = execute_bounded(
        "head -c 100000 /dev/zero | tr '\\0' 'A'",
        &config,
    );

    assert!(result.overflowed);
    assert!(result.failed);
    assert!(result.combined.len() <= 30 * 1024);
    assert!(result
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("exceeded limit"));
}

#[cfg(unix)]
#[test]
fn child_process_tree_is_terminated_on_timeout() {
    let config = ExecConfig {
        timeout: Duration::from_millis(300),
        max_output_bytes: 1024 * 1024,
    };
    // Spawn child background process that would otherwise stay alive
    let result = execute_bounded("sh -c 'sleep 15 & sleep 15'", &config);
    assert!(result.timed_out);
    assert!(result.failed);
}

#[cfg(unix)]
#[test]
fn mcp_reduce_exec_respects_custom_timeout_and_limit() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_token-reducer"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 101,
        "method": "tools/call",
        "params": {
            "name": "reduce_exec",
            "arguments": {
                "command": "sleep 10",
                "timeout_seconds": 1
            }
        }
    });

    writeln!(child.stdin.take().unwrap(), "{}", request).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("timed out"));
}
