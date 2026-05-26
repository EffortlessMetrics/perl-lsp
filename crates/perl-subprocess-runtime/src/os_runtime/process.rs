use super::resolve_command_invocation;
use super::validation::validate_command_input;
use crate::{SubprocessError, SubprocessOutput};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn run_os_command(
    program: &str,
    args: &[&str],
    stdin: Option<&[u8]>,
    timeout_secs: Option<u64>,
) -> Result<SubprocessOutput, SubprocessError> {
    validate_command_input(program, args)?;
    let mut child = spawn_child(program, args, stdin)?;
    write_stdin(program, &mut child, stdin)?;
    wait_for_child(program, child, timeout_secs)
}

fn spawn_child(
    program: &str,
    args: &[&str],
    stdin: Option<&[u8]>,
) -> Result<Child, SubprocessError> {
    let (resolved_program, resolved_args) = resolve_command_invocation(program, args);
    let mut cmd = Command::new(&resolved_program);
    cmd.args(resolved_args.iter().map(String::as_str));
    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.spawn().map_err(|e| SubprocessError::new(format!("Failed to start {}: {}", program, e)))
}

fn write_stdin(
    program: &str,
    child: &mut Child,
    stdin: Option<&[u8]>,
) -> Result<(), SubprocessError> {
    if let Some(input) = stdin
        && let Some(mut child_stdin) = child.stdin.take()
    {
        child_stdin.write_all(input).map_err(|e| {
            SubprocessError::new(format!("Failed to write to {} stdin: {}", program, e))
        })?;
    }
    Ok(())
}

fn wait_for_child(
    program: &str,
    child: Child,
    timeout_secs: Option<u64>,
) -> Result<SubprocessOutput, SubprocessError> {
    match timeout_secs {
        None => wait_without_timeout(program, child),
        Some(secs) => wait_with_timeout(program, child, secs),
    }
}

fn wait_without_timeout(program: &str, child: Child) -> Result<SubprocessOutput, SubprocessError> {
    let output = child
        .wait_with_output()
        .map_err(|e| SubprocessError::new(format!("Failed to wait for {}: {}", program, e)))?;
    Ok(output.into())
}

fn wait_with_timeout(
    program: &str,
    mut child: Child,
    timeout_secs: u64,
) -> Result<SubprocessOutput, SubprocessError> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if child
            .try_wait()
            .map_err(|e| SubprocessError::new(format!("Failed to poll {}: {}", program, e)))?
            .is_some()
        {
            let output = child.wait_with_output().map_err(|e| {
                SubprocessError::new(format!("Failed to wait for {}: {}", program, e))
            })?;
            return Ok(output.into());
        }
        if Instant::now() >= deadline {
            terminate_timed_out_child(program, &mut child, timeout_secs)?;
            return Err(SubprocessError::new(format!(
                "subprocess timed out after {} seconds",
                timeout_secs
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn terminate_timed_out_child(
    program: &str,
    child: &mut Child,
    timeout_secs: u64,
) -> Result<(), SubprocessError> {
    if let Err(kill_err) = child.kill() {
        // Best effort: process may have already exited between `try_wait` and `kill`.
        let already_exited = child
            .try_wait()
            .map_err(|e| SubprocessError::new(format!("Failed to poll {}: {}", program, e)))?
            .is_some();
        if !already_exited {
            return Err(SubprocessError::new(format!(
                "subprocess timed out after {} seconds and failed to terminate {}: {}",
                timeout_secs, program, kill_err
            )));
        }
    }
    let _ = child.wait();
    Ok(())
}

impl From<std::process::Output> for SubprocessOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            stdout: output.stdout,
            stderr: output.stderr,
            status_code: output.status.code().unwrap_or(-1),
        }
    }
}
