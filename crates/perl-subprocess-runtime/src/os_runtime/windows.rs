use std::path::{Path, PathBuf};

use super::path_selection::select_path_candidate;

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

/// Resolve a program name to an absolute path by searching the `PATH`
/// environment variable directories.
///
/// # Dispatch rules
///
/// | Input form | Action |
/// |---|---|
/// | Absolute path (`C:\…`, `\\unc\…`) | Pass through unchanged — already resolved |
/// | Relative path with separator (`.\x.exe`, `sub\x`) | **Fail closed** (`None`) — CWD-relative |
/// | Bare name, no extension (`perltidy`) | Search PATH dirs × PATHEXT extensions |
/// | Bare name, with extension (`perltidy.exe`) | Search PATH dirs for that exact filename |
///
/// A *relative* path that merely contains a separator (`.\pwned.exe`,
/// `..\pwned.exe`, `sub\tool`, `/x`, `\tool`) is **not** pre-resolved: Windows
/// `CreateProcess` resolves it against the process CWD (the LSP workspace root),
/// so it is the same binary-planting vector as a bare name and must fail closed.
/// Only an absolute path is treated as a genuinely caller-resolved location.
///
/// Bare names with an extension are **not** passed through unchanged.
/// `Command::new("perltidy.exe")` would let Windows' CreateProcess CWD search
/// find a planted workspace binary — the same binary-planting RCE the PATH-only
/// search closes.  Both bare-name cases feed `select_path_candidate` which
/// enforces the absolute-only + CWD-exclusion invariants.
///
/// # Security invariant
///
/// The current working directory is **never** consulted.  An attacker who plants
/// `perltidy.exe` (or any extensioned tool name) in the LSP workspace root must
/// not be able to hijack tool invocations.
pub(crate) fn resolve_windows_program(program: &str) -> Option<String> {
    let program_path = Path::new(program);
    let has_separator = program.contains('\\') || program.contains('/');

    // A path-bearing program is treated as caller-resolved ONLY if it is
    // absolute.  A relative path that merely contains a separator
    // (".\\pwned.exe", "..\\pwned.exe", "sub\\tool", "/x", "\\tool") is still
    // resolved by CreateProcess against the CWD (workspace root) — the same
    // binary-planting vector — so it fails closed here.
    // Note: we do NOT early-return for bare names that happen to carry an
    // extension (e.g. "perltidy.exe").  Such names must go through the
    // PATH-only search below to prevent the CreateProcess CWD lookup.
    if has_separator {
        return if program_path.is_absolute() { Some(program.to_string()) } else { None };
    }

    let has_extension = program_path.extension().is_some();

    // Security: skip any PATH entry that is empty or not absolute.  An empty
    // entry (`;;` or trailing `;`) produces a relative path via `dir.join`,
    // which resolves against the CWD at `.is_file()` time — the same
    // binary-planting vector we are closing.  A relative PATH entry is equally
    // dangerous and is almost always a misconfiguration, not intentional.
    let path_dirs = path_dirs_from_env();

    let candidates: Vec<String> = if has_extension {
        // Bare name with extension: search each PATH dir for the exact filename.
        // Do not append PATHEXT — the caller already specified the extension.
        path_dirs
            .iter()
            .filter(|dir| !dir.as_os_str().is_empty() && dir.is_absolute())
            .filter_map(|dir| {
                let candidate = dir.join(program);
                if candidate.is_file() { candidate.to_str().map(str::to_string) } else { None }
            })
            .collect()
    } else {
        // Bare name without extension: try each PATH dir × each PATHEXT extension.
        let path_exts = pathext_from_env();
        path_dirs
            .iter()
            .filter(|dir| !dir.as_os_str().is_empty() && dir.is_absolute())
            .flat_map(|dir| {
                path_exts.iter().filter_map(|ext| {
                    let mut candidate = dir.join(program);
                    candidate.set_extension(ext.trim_start_matches('.'));
                    if candidate.is_file() { candidate.to_str().map(str::to_string) } else { None }
                })
            })
            .collect()
    };

    let cwd = std::env::current_dir().ok()?;
    let candidate_refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
    select_path_candidate(&candidate_refs, &cwd)
}

/// Returns the list of directories in the `PATH` environment variable.
fn path_dirs_from_env() -> Vec<PathBuf> {
    std::env::var_os("PATH").map(|val| std::env::split_paths(&val).collect()).unwrap_or_default()
}

/// Returns the list of extensions from `PATHEXT`, falling back to the standard
/// Windows default when the variable is absent.
fn pathext_from_env() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .map(str::to_string)
        .collect()
}

pub(crate) fn windows_requires_cmd_shell(program: &str) -> bool {
    Path::new(program)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("bat") || ext.eq_ignore_ascii_case("cmd"))
        .unwrap_or(false)
}

/// Resolve the absolute path to `cmd.exe`, used to execute `.bat`/`.cmd`
/// wrappers.
///
/// # Security invariant
///
/// Never returns the bare string `"cmd.exe"`.  `Command::new("cmd.exe")` would
/// let CreateProcess perform its CWD-first executable search, so a `cmd.exe`
/// planted in the LSP workspace root could hijack *every* batch-wrapper
/// invocation — the same binary-planting RCE that the PATH-only program search
/// closes for the tool itself.  Only an absolute, existing file under
/// `%SystemRoot%\System32` (canonical) or `%ComSpec%` (standard pointer) is
/// accepted; otherwise `None` (fail closed — refusing to run is safer than
/// running a planted shell).  Neither source is attacker-controlled in the
/// threat model (the attacker controls workspace files / CWD, not the user's
/// environment), but both are still required to be absolute existing files so a
/// misconfiguration cannot reintroduce a relative lookup.
pub(crate) fn resolve_cmd_exe() -> Option<String> {
    // Canonical location first: %SystemRoot%\System32\cmd.exe.
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        let candidate = Path::new(&system_root).join("System32").join("cmd.exe");
        if candidate.is_absolute() && candidate.is_file() {
            return candidate.to_str().map(str::to_string);
        }
    }
    // Fallback: %ComSpec%, accepted only when it is itself an absolute file.
    if let Some(comspec) = std::env::var_os("ComSpec") {
        let candidate = Path::new(&comspec);
        if candidate.is_absolute() && candidate.is_file() {
            return candidate.to_str().map(str::to_string);
        }
    }
    None
}
