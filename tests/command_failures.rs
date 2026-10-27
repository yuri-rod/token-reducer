use std::process::Command;

#[cfg(unix)]
#[test]
fn mcp_exec_reports_failed_command_as_tool_error() {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_token-reducer"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.take().unwrap(), "{}", serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "reduce_exec", "arguments": {"command": "printf 'build failed'; exit 17"}}
    })).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("build failed"));
}

#[cfg(unix)]
#[test]
fn run_preserves_failure_status_and_diagnostics() {
    let output = Command::new(env!("CARGO_BIN_EXE_token-reducer"))
        .args([
            "run",
            "--",
            "sh",
            "-c",
            "printf 'Error: build failed\\n' >&2; exit 17",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(17));
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("Error: build failed"));
}

#[cfg(unix)]
#[test]
fn run_preserves_success_status() {
    let output = Command::new(env!("CARGO_BIN_EXE_token-reducer"))
        .args(["run", "--", "sh", "-c", "printf 'build finished\\n'"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("build finished"));
}

#[test]
fn diff_preserves_git_error_status_and_stderr() {
    let workspace = tempfile::tempdir().unwrap();
    let expected = Command::new("git")
        .args(["diff", "--invalid-token-reducer-option"])
        .current_dir(workspace.path())
        .output()
        .unwrap();
    assert!(!expected.status.success());
    let output = Command::new(env!("CARGO_BIN_EXE_token-reducer"))
        .args(["diff", "--", "--invalid-token-reducer-option"])
        .current_dir(workspace.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), expected.status.code());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains(String::from_utf8(expected.stderr).unwrap().trim()));
}
