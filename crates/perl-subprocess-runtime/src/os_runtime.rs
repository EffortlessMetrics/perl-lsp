use crate::{SubprocessError, SubprocessOutput, SubprocessRuntime};
use std::io::Write;
#[cfg(windows)]
use std::path::Path;
use std::process::{Command, Stdio};

/// Default implementation using `std::process::Command`.
pub struct OsSubprocessRuntime {
    timeout_secs: Option<u64>,
}

impl OsSubprocessRuntime {
    /// Create a new OS subprocess runtime with no timeout.
    pub fn new() -> Self {
        Self { timeout_secs: None }
    }

    /// Create a new OS subprocess runtime with the given wall-clock timeout.
    ///
    /// If the subprocess does not complete within `timeout_secs` seconds the
    /// call returns a `SubprocessError` with a "timed out" message and attempts
    /// to terminate the spawned process before returning.
    ///
    /// # Stdin size caveat
    ///
    /// Stdin data is written synchronously before the timeout poll loop begins.
    /// If the subprocess hangs before consuming stdin and the data exceeds the
    /// OS pipe buffer (~64 KiB on Linux), `run_command` will block in the write
    /// phase and the timeout will not fire. For typical Perl source files this
    /// is not a concern.
    ///
    /// # Panics
    ///
    /// Panics if `timeout_secs` is zero (a zero-second timeout would time out
    /// every command immediately and is almost certainly a caller bug).
    pub fn with_timeout(timeout_secs: u64) -> Self {
        assert!(timeout_secs > 0, "timeout_secs must be greater than zero");
        Self { timeout_secs: Some(timeout_secs) }
    }
}

impl Default for OsSubprocessRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SubprocessRuntime for OsSubprocessRuntime {
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError> {
        validate_command_input(program, args)?;
        let (resolved_program, resolved_args) = resolve_command_invocation(program, args);
        let mut cmd = Command::new(&resolved_program);
        cmd.args(resolved_args.iter().map(String::as_str));
        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| SubprocessError::new(format!("Failed to start {}: {}", program, e)))?;
        if let Some(input) = stdin
            && let Some(mut child_stdin) = child.stdin.take()
        {
            child_stdin.write_all(input).map_err(|e| {
                SubprocessError::new(format!("Failed to write to {} stdin: {}", program, e))
            })?;
        }
        match self.timeout_secs {
            None => {
                let output = child.wait_with_output().map_err(|e| {
                    SubprocessError::new(format!("Failed to wait for {}: {}", program, e))
                })?;
                Ok(SubprocessOutput {
                    stdout: output.stdout,
                    stderr: output.stderr,
                    status_code: output.status.code().unwrap_or(-1),
                })
            }
            Some(secs) => {
                use std::time::{Duration, Instant};
                let deadline = Instant::now() + Duration::from_secs(secs);
                loop {
                    if child
                        .try_wait()
                        .map_err(|e| {
                            SubprocessError::new(format!("Failed to poll {}: {}", program, e))
                        })?
                        .is_some()
                    {
                        let output = child.wait_with_output().map_err(|e| {
                            SubprocessError::new(format!("Failed to wait for {}: {}", program, e))
                        })?;
                        return Ok(SubprocessOutput {
                            stdout: output.stdout,
                            stderr: output.stderr,
                            status_code: output.status.code().unwrap_or(-1),
                        });
                    }
                    if Instant::now() >= deadline {
                        if let Err(kill_err) = child.kill() {
                            // Best effort: process may have already exited between `try_wait`
                            // and `kill`.
                            let already_exited = child
                                .try_wait()
                                .map_err(|e| {
                                    SubprocessError::new(format!(
                                        "Failed to poll {}: {}",
                                        program, e
                                    ))
                                })?
                                .is_some();
                            if !already_exited {
                                return Err(SubprocessError::new(format!(
                                    "subprocess timed out after {} seconds and failed to terminate {}: {}",
                                    secs, program, kill_err
                                )));
                            }
                        }
                        let _ = child.wait();
                        return Err(SubprocessError::new(format!(
                            "subprocess timed out after {} seconds",
                            secs
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

fn validate_command_input(program: &str, args: &[&str]) -> Result<(), SubprocessError> {
    if program.trim().is_empty() {
        return Err(SubprocessError::new("program name must not be empty"));
    }
    if program.contains('\0') {
        return Err(SubprocessError::new("program name must not contain NUL bytes"));
    }
    if args.iter().any(|arg| arg.contains('\0')) {
        return Err(SubprocessError::new("arguments must not contain NUL bytes"));
    }
    Ok(())
}

pub(crate) fn resolve_command_invocation(program: &str, args: &[&str]) -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let resolved_program =
            resolve_windows_program(program).unwrap_or_else(|| program.to_string());
        if windows_requires_cmd_shell(&resolved_program) {
            let command_line = std::iter::once(resolved_program.as_str())
                .chain(args.iter().copied())
                .map(windows_quote_for_cmd)
                .collect::<Vec<_>>()
                .join(" ");
            // /D  - disable AutoRun registry commands.
            // /V:OFF - disable delayed expansion so that !VAR! patterns in
            //          arguments are not expanded even when the caller's
            //          environment has delayed expansion enabled.
            // /S  - strip the outer quotes from the /C argument and re-parse
            //       the remainder, which lets each individual token retain its
            //       own double-quoting.
            let shell_args = vec![
                "/D".to_string(),
                "/V:OFF".to_string(),
                "/S".to_string(),
                "/C".to_string(),
                command_line,
            ];
            return ("cmd.exe".to_string(), shell_args);
        }
        (resolved_program, args.iter().map(|arg| (*arg).to_string()).collect())
    }
    #[cfg(not(windows))]
    {
        (program.to_string(), args.iter().map(|arg| (*arg).to_string()).collect())
    }
}

#[cfg(windows)]
/// Quote a single argument for use inside a `cmd.exe /V:OFF /S /C "..."` command line.
///
/// ## cmd.exe quoting rules inside double-quoted regions
///
/// Once cmd.exe sees an opening `"` it enters a quoted region. Inside that region:
///
/// - Characters like `&`, `|`, `<`, `>`, `(`, and `)` are literal; they do not
///   need `^` escaping.
/// - `^` is also literal in a quoted region, so doubling it would change the
///   argument seen by the child process.
/// - `%` is still processed by the variable-substitution pass, which runs before
///   the shell-metachar pass and is not suppressed by quoting. Double it (`%%`)
///   to produce a literal `%`.
/// - `!` would be processed by the delayed-expansion pass when `/V:ON` is in
///   effect. We invoke cmd.exe with `/V:OFF` to suppress this entirely, so `!`
///   needs no escaping here.
/// - To embed a literal `"` inside a double-quoted cmd.exe token, use `""` (the
///   cmd.exe shell convention). The `\"` form is for `CommandLineToArgvW` (the
///   Win32 C-runtime argv parser), which is a different parser from the cmd.exe
///   shell command-line parser.
pub(crate) fn windows_quote_for_cmd(arg: &str) -> String {
    let mut escaped = String::with_capacity(arg.len() + 2);
    escaped.push('"');
    for ch in arg.chars() {
        match ch {
            '%' => escaped.push_str("%%"),
            '"' => escaped.push_str("\"\""),
            _ => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(windows)]
fn resolve_windows_program(program: &str) -> Option<String> {
    let program_path = Path::new(program);
    let has_separator = program.contains('\\') || program.contains('/');
    let has_extension = program_path.extension().is_some();
    if has_separator || has_extension {
        return Some(program.to_string());
    }
    let output = Command::new("where")
        .arg(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .max_by_key(|candidate| windows_program_priority(candidate))
        .map(String::from)
}

#[cfg(windows)]
pub(crate) fn windows_program_priority(candidate: &str) -> u8 {
    match Path::new(candidate)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        Some(ext) if ext == "exe" => 5,
        Some(ext) if ext == "com" => 4,
        Some(ext) if ext == "cmd" => 3,
        Some(ext) if ext == "bat" => 2,
        Some(_) => 1,
        None => 0,
    }
}

#[cfg(windows)]
fn windows_requires_cmd_shell(program: &str) -> bool {
    Path::new(program)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("bat") || ext.eq_ignore_ascii_case("cmd"))
        .unwrap_or(false)
}
