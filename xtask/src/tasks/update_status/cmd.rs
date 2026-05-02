//! Command helpers for status regeneration.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;

/// Run a command with a timeout, returning combined stdout+stderr or empty string on failure.
fn stream_reader<R: Read>(reader: R, log_prefix: &'static str) -> String {
    let mut captured = String::new();
    let mut buf = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        let Ok(bytes) = buf.read_line(&mut line) else {
            break;
        };
        if bytes == 0 {
            break;
        }
        eprint!("[{log_prefix}] {line}");
        captured.push_str(&line);
    }
    captured
}

pub(super) fn run_cmd(root: &Path, args: &[&str], timeout: Duration) -> String {
    let Some((&program, rest)) = args.split_first() else {
        return String::new();
    };

    eprintln!("[update-status] running: {}", args.join(" "));
    let result = Command::new(program)
        .args(rest)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match result {
        Ok(c) => c,
        Err(err) => {
            eprintln!("[update-status] failed to start `{}`: {err}", args.join(" "));
            return String::new();
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let out_handle = stdout.map(|out| std::thread::spawn(move || stream_reader(out, "stdout")));
    let err_handle = stderr.map(|err| std::thread::spawn(move || stream_reader(err, "stderr")));

    let started_at = Instant::now();
    let mut last_heartbeat = started_at;
    let command_name = args.join(" ");

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if started_at.elapsed() >= timeout {
                    eprintln!(
                        "[update-status] command timed out after {timeout:?}: {command_name}"
                    );
                    let _ = child.kill();
                    break child.wait().ok();
                }
                if last_heartbeat.elapsed() >= Duration::from_secs(30) {
                    eprintln!("[update-status] still running (heartbeat): {command_name}");
                    last_heartbeat = Instant::now();
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                eprintln!("[update-status] failed to poll `{command_name}`: {err}");
                let _ = child.kill();
                break child.wait().ok();
            }
        }
    };

    let mut combined = String::new();
    if let Some(handle) = out_handle {
        combined.push_str(&handle.join().unwrap_or_default());
    }
    if let Some(handle) = err_handle {
        combined.push_str(&handle.join().unwrap_or_default());
    }
    if let Some(status) = status
        && !status.success()
    {
        eprintln!("[update-status] command exited with {status}: {}", args.join(" "));
    }
    combined
}

/// Like `run_cmd` but merges stderr into stdout via shell `2>&1`.
///
/// Essential for `cargo test -- --list`: cargo writes crate headers to stderr and test
/// names to stdout, so without `2>&1` the parser sees all names before all headers and
/// can never associate a name with its crate.  Single-quote-escapes each argument to
/// avoid shell injection while preserving flags like `--`.
pub(super) fn run_cmd_merged(root: &Path, args: &[&str], timeout: Duration) -> String {
    let _ = timeout;
    if args.is_empty() {
        return String::new();
    }
    #[cfg(unix)]
    let shell_cmd = {
        let shell_args: Vec<String> =
            args.iter().map(|&a| format!("'{}'", a.replace('\'', "'\\''"))).collect();
        format!("{} 2>&1", shell_args.join(" "))
    };
    #[cfg(not(unix))]
    let shell_cmd = {
        let shell_args: Vec<String> = args.iter().map(|&a| a.to_owned()).collect();
        format!("{} 2>&1", shell_args.join(" "))
    };
    #[cfg(unix)]
    let merged = ["sh", "-c", &shell_cmd];
    #[cfg(not(unix))]
    let merged = ["cmd", "/C", &shell_cmd];
    run_cmd(root, &merged, timeout)
}

pub(super) fn run_subsystem<T>(
    name: &str,
    repro: &str,
    action: impl FnOnce() -> Result<T>,
) -> Result<T> {
    eprintln!("[update-status] starting subsystem: {name}");
    let result = action();
    match result {
        Ok(value) => {
            eprintln!("[update-status] completed subsystem: {name}");
            Ok(value)
        }
        Err(err) => {
            eprintln!("[update-status] subsystem failed: {name}");
            eprintln!("[update-status] repro: {repro}");
            Err(err)
        }
    }
}
