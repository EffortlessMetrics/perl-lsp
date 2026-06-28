#[cfg(windows)]
use super::windows::{
    resolve_cmd_exe, resolve_windows_program, windows_quote_for_cmd, windows_requires_cmd_shell,
};
use crate::SubprocessError;

/// Resolve `(program, args)` ready for `Command::new`, failing closed on Windows
/// when the program cannot be resolved to a safe absolute path.
///
/// # Security: never fall back to the bare program name
///
/// On Windows, `Command::new(bare_name)` triggers CreateProcess's CWD-first
/// executable search, so a binary planted in the LSP workspace root would run in
/// preference to the real tool (binary-planting RCE — #2764).  The selector
/// `resolve_windows_program` already searches only absolute PATH directories and
/// excludes the CWD, returning `None` when nothing safe is found.  The hole this
/// closes is the *caller's* old `unwrap_or_else(|| program.to_string())`, which
/// restored the bare name on that `None` and re-armed the exact RCE the selector
/// had just prevented.  We now propagate an error instead — the tool is reported
/// missing rather than risking execution of a planted binary.  The same rule
/// applies to the `cmd.exe` used for `.bat`/`.cmd` wrappers (see
/// [`resolve_cmd_exe`]): it is resolved to an absolute path or refused.
pub(crate) fn resolve_command_invocation(
    program: &str,
    args: &[&str],
) -> Result<(String, Vec<String>), SubprocessError> {
    #[cfg(windows)]
    {
        let resolved_program = resolve_windows_program(program).ok_or_else(|| {
            SubprocessError::new(format!(
                "command not found in any absolute PATH directory \
                 (current directory excluded for security): {program}"
            ))
        })?;
        if windows_requires_cmd_shell(&resolved_program) {
            let cmd_exe = resolve_cmd_exe().ok_or_else(|| {
                SubprocessError::new(format!(
                    "cmd.exe not found via %SystemRoot%\\System32 or %ComSpec%; \
                     refusing to execute batch wrapper via a CWD-searchable bare \
                     name: {resolved_program}"
                ))
            })?;
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
            // The absolute cmd.exe path — never the bare "cmd.exe", which would
            // itself be subject to the CWD-first CreateProcess search.
            return Ok((cmd_exe, shell_args));
        }
        Ok((resolved_program, args.iter().map(|arg| (*arg).to_string()).collect()))
    }
    #[cfg(not(windows))]
    {
        // Non-Windows: CreateProcess CWD-search semantics do not apply; the OS
        // resolves `program` via PATH (or as an explicit path) without a
        // CWD-first lookup, so pass through unchanged.
        Ok((program.to_string(), args.iter().map(|arg| (*arg).to_string()).collect()))
    }
}
