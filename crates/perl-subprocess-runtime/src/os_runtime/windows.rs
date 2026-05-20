use std::path::Path;
use std::process::{Command, Stdio};

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

pub(crate) fn resolve_windows_program(program: &str) -> Option<String> {
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

pub(crate) fn windows_requires_cmd_shell(program: &str) -> bool {
    Path::new(program)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("bat") || ext.eq_ignore_ascii_case("cmd"))
        .unwrap_or(false)
}
