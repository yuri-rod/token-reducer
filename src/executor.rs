use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 5 * 1024 * 1024; // 5 MiB

#[derive(Debug, Clone)]
pub struct ExecConfig {
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

#[derive(Debug)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub combined: String,
    pub exit_code: Option<i32>,
    pub failed: bool,
    pub timed_out: bool,
    pub overflowed: bool,
    pub error_message: Option<String>,
}

#[cfg(unix)]
fn prepare_command(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(not(unix))]
fn prepare_command(_cmd: &mut Command) {}

#[cfg(unix)]
fn kill_process_tree(pid: u32) {
    let pgid = pid as i32;
    unsafe {
        // Send SIGTERM to the process group first
        libc::kill(-pgid, libc::SIGTERM);
    }
    thread::sleep(Duration::from_millis(150));
    unsafe {
        // Force kill with SIGKILL to process group
        libc::kill(-pgid, libc::SIGKILL);
    }
}

#[cfg(windows)]
fn kill_process_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output();
}

#[cfg(not(any(unix, windows)))]
fn kill_process_tree(_pid: u32) {}

/// Execute a shell command safely with process group isolation,
/// a strict execution timeout, and a bounded output capture buffer.
pub fn execute_bounded(command_str: &str, config: &ExecConfig) -> ExecResult {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", command_str]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command_str]);
        c
    };

    prepare_command(&mut cmd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let err = format!("Failed to spawn process: {}", e);
            return ExecResult {
                stdout: String::new(),
                stderr: err.clone(),
                combined: err.clone(),
                exit_code: None,
                failed: true,
                timed_out: false,
                overflowed: false,
                error_message: Some(err),
            };
        }
    };

    let pid = child.id();
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let max_bytes = config.max_output_bytes;
    let total_bytes = Arc::new(AtomicUsize::new(0));
    let overflowed = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Spawn reader thread for stdout
    let total_stdout = Arc::clone(&total_bytes);
    let overflow_stdout = Arc::clone(&overflowed);
    let stop_stdout = Arc::clone(&stop_flag);
    let stdout_handle = thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut pipe) = stdout_pipe {
            let mut chunk = [0u8; 8192];
            while !stop_stdout.load(Ordering::Relaxed) {
                match pipe.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        let prev = total_stdout.fetch_add(n, Ordering::SeqCst);
                        if prev + n > max_bytes {
                            overflow_stdout.store(true, Ordering::SeqCst);
                            let allowed = max_bytes.saturating_sub(prev);
                            if allowed > 0 && buffer.len() < max_bytes {
                                buffer.extend_from_slice(&chunk[..allowed]);
                            }
                            break;
                        } else {
                            buffer.extend_from_slice(&chunk[..n]);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        }
        buffer
    });

    // Spawn reader thread for stderr
    let total_stderr = Arc::clone(&total_bytes);
    let overflow_stderr = Arc::clone(&overflowed);
    let stop_stderr = Arc::clone(&stop_flag);
    let stderr_handle = thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut pipe) = stderr_pipe {
            let mut chunk = [0u8; 8192];
            while !stop_stderr.load(Ordering::Relaxed) {
                match pipe.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        let prev = total_stderr.fetch_add(n, Ordering::SeqCst);
                        if prev + n > max_bytes {
                            overflow_stderr.store(true, Ordering::SeqCst);
                            let allowed = max_bytes.saturating_sub(prev);
                            if allowed > 0 && buffer.len() < max_bytes {
                                buffer.extend_from_slice(&chunk[..allowed]);
                            }
                            break;
                        } else {
                            buffer.extend_from_slice(&chunk[..n]);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        }
        buffer
    });

    let start_time = Instant::now();
    let mut exit_code = None;
    let mut timed_out = false;

    loop {
        if overflowed.load(Ordering::SeqCst) {
            break;
        }

        if let Ok(Some(status)) = child.try_wait() {
            exit_code = status.code();
            break;
        }

        if start_time.elapsed() >= config.timeout {
            timed_out = true;
            break;
        }

        thread::sleep(Duration::from_millis(20));
    }

    if timed_out || overflowed.load(Ordering::SeqCst) {
        stop_flag.store(true, Ordering::SeqCst);
        kill_process_tree(pid);
        let _ = child.kill();
        let _ = child.wait();
    }

    let stdout_bytes = stdout_handle.join().unwrap_or_default();
    let stderr_bytes = stderr_handle.join().unwrap_or_default();
    let is_overflow = overflowed.load(Ordering::SeqCst);

    let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr_str = String::from_utf8_lossy(&stderr_bytes).to_string();

    let mut error_message = None;
    let failed = if timed_out {
        error_message = Some(format!(
            "Execution timed out after {}s; process group terminated.",
            config.timeout.as_secs()
        ));
        true
    } else if is_overflow {
        error_message = Some(format!(
            "Execution output exceeded limit of {} bytes; process terminated.",
            config.max_output_bytes
        ));
        true
    } else {
        exit_code.map_or(true, |code| code != 0)
    };

    let mut combined = if stderr_str.is_empty() {
        stdout_str.clone()
    } else if stdout_str.is_empty() {
        stderr_str.clone()
    } else {
        format!("{}\n{}", stdout_str, stderr_str)
    };

    if let Some(ref err) = error_message {
        if combined.is_empty() {
            combined = err.clone();
        } else {
            combined = format!("{}\n\n[Warning: {}]", combined, err);
        }
    }

    ExecResult {
        stdout: stdout_str,
        stderr: stderr_str,
        combined,
        exit_code,
        failed,
        timed_out,
        overflowed: is_overflow,
        error_message,
    }
}
