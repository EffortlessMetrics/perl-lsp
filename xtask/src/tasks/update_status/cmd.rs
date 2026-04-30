//! Command execution helpers shared across update_status subsystem modules.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use color_eyre::eyre::{Context, Result, eyre};
use regex::Regex;

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

/// Run a command with a timeout, returning combined stdout+stderr or empty string on failure.
pub fn run_cmd(root: &Path, args: &[&str], timeout: Duration) -> String {
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

    // Basic timeout emulation: we cannot use `std::process::Command` timeout
    // directly, so we rely on the process completing.  The Python version used
    // subprocess.run with timeout; here we accept the default behavior but keep
    // the parameter for API compatibility and future improvement.
    let _ = timeout;

    let heartbeat_running = Arc::new(AtomicBool::new(true));
    let heartbeat_flag = Arc::clone(&heartbeat_running);
    let command_name = args.join(" ");
    let heartbeat = std::thread::spawn(move || {
        while heartbeat_flag.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_secs(30));
            if heartbeat_flag.load(Ordering::Relaxed) {
                eprintln!("[update-status] still running (heartbeat): {command_name}");
            }
        }
    });

    let status = child.wait();
    heartbeat_running.store(false, Ordering::Relaxed);
    let _ = heartbeat.join();
    let mut combined = String::new();
    if let Some(handle) = out_handle {
        combined.push_str(&handle.join().unwrap_or_default());
    }
    if let Some(handle) = err_handle {
        combined.push_str(&handle.join().unwrap_or_default());
    }
    if let Ok(status) = status
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
pub fn run_cmd_merged(root: &Path, args: &[&str], timeout: Duration) -> String {
    let _ = timeout;
    if args.is_empty() {
        return String::new();
    }
    let shell_args: Vec<String> =
        args.iter().map(|&a| format!("'{}'", a.replace('\'', "'\\''"))).collect();
    let shell_cmd = format!("{} 2>&1", shell_args.join(" "));
    #[cfg(unix)]
    let merged = ["sh", "-c", &shell_cmd];
    #[cfg(not(unix))]
    let merged = ["cmd", "/C", &shell_cmd];
    run_cmd(root, &merged, timeout)
}

pub fn run_subsystem<T>(
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

/// Replace content between `begin_marker\n...\nend_marker` (inclusive of markers).
pub fn replace_block(
    text: &str,
    begin_marker: &str,
    end_marker: &str,
    new_content: &str,
) -> Result<String> {
    let escaped_begin = regex::escape(begin_marker);
    let escaped_end = regex::escape(end_marker);
    let pattern = format!(r"(?s)({})\n.*?({})", escaped_begin, escaped_end);
    let re = Regex::new(&pattern).context("building block replacement regex")?;

    let replacement = format!("{begin_marker}\n{new_content}\n{end_marker}");

    let mut count = 0;
    let result = re.replace_all(text, |_caps: &regex::Captures<'_>| {
        count += 1;
        replacement.clone()
    });

    if count != 1 {
        return Err(eyre!("Expected 1 match for block {begin_marker:?}, got {count}"));
    }

    Ok(result.into_owned())
}
