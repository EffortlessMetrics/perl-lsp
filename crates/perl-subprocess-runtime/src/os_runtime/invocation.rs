#[cfg(windows)]
use super::windows::{resolve_windows_program, windows_quote_for_cmd, windows_requires_cmd_shell};

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
