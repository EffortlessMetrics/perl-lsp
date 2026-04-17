//! Cross-platform utilities for Perl path resolution and environment setup.

// Re-export format_command_args for backward compatibility (was in old platform re-export module)
pub use crate::command_args::format_command_args;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[cfg(windows)]
const PATH_SEPARATOR: char = ';';
#[cfg(not(windows))]
const PATH_SEPARATOR: char = ':';

#[cfg(windows)]
const PERL_EXECUTABLE: &str = "perl.exe";
#[cfg(not(windows))]
const PERL_EXECUTABLE: &str = "perl";

/// The result of Perl interpreter discovery.
///
/// Separates the "found on PATH" case from the "found via OS fallback" case
/// so callers can log or surface different messages to users.
#[derive(Debug, Clone, PartialEq)]
pub enum PerlInterpreterResult {
    /// Perl was found via the configured `perl-lsp.perl.path` setting.
    ConfiguredPath(PathBuf),
    /// Perl was found on PATH (the normal case).
    FoundOnPath(PathBuf),
    /// Perl was NOT on PATH but was found at a well-known OS install location.
    /// Carries the found path and a human-readable label (e.g. "Strawberry Perl").
    FoundViaFallback { path: PathBuf, label: String },
    /// No Perl interpreter found anywhere.
    /// Carries the list of locations searched, for use in an error message.
    NotFound { searched: Vec<String> },
}

/// Rank a Perl binary path for preference on Windows.
///
/// Returns a lower score for higher-priority interpreters.
/// Strawberry Perl ranks best (1), then ActiveState (2), then msys/Git Bash (100).
#[cfg(windows)]
fn windows_perl_rank(path: &std::path::Path) -> u8 {
    let s = path.to_string_lossy().to_ascii_lowercase();
    if s.contains("strawberry") {
        1
    } else if s.contains("perl64") || s.contains("activestate") || s.contains("activeperl") {
        2
    } else if s.contains(r"\git\usr\bin") || s.contains("/git/usr/bin") || s.contains("msys") {
        100
    } else {
        50
    }
}

/// Collect all Perl executables found on PATH, ranked for Windows preference.
///
/// On Windows, returns all matches sorted by [`windows_perl_rank`] so the caller
/// can pick the best one. On non-Windows, returns candidates in PATH order (no ranking).
fn find_all_perl_on_path(path_env: &str) -> Vec<PathBuf> {
    #[allow(unused_mut)]
    let mut found: Vec<PathBuf> = path_env
        .split(PATH_SEPARATOR)
        .map(|dir| PathBuf::from(dir).join(PERL_EXECUTABLE))
        .filter(|p| p.exists() && p.is_file())
        .collect();

    #[cfg(windows)]
    found.sort_by_key(|p| windows_perl_rank(p));

    found
}

/// Canonical OS-specific fallback paths to probe when Perl is not on PATH.
///
/// Returns `(path, label)` pairs. Probed in order; first existing file wins.
fn fallback_perl_paths() -> Vec<(PathBuf, &'static str)> {
    #[cfg(windows)]
    {
        vec![
            (PathBuf::from(r"C:\Strawberry\perl\bin\perl.exe"), "Strawberry Perl"),
            (PathBuf::from(r"C:\Perl64\bin\perl.exe"), "ActiveState Perl (64-bit)"),
            (
                {
                    let pf = env::var("ProgramFiles")
                        .unwrap_or_else(|_| r"C:\Program Files".to_string());
                    PathBuf::from(pf).join(r"Strawberry\perl\bin\perl.exe")
                },
                "Strawberry Perl (Program Files)",
            ),
        ]
    }
    #[cfg(target_os = "macos")]
    {
        vec![
            (PathBuf::from("/opt/homebrew/bin/perl"), "Homebrew Perl (Apple Silicon)"),
            (PathBuf::from("/usr/local/bin/perl"), "Homebrew Perl (Intel)"),
            (PathBuf::from("/usr/bin/perl"), "macOS system Perl"),
        ]
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        // Linux and others
        vec![
            (PathBuf::from("/usr/bin/perl"), "system Perl"),
            (PathBuf::from("/usr/local/bin/perl"), "local Perl"),
        ]
    }
}

/// Find the best available Perl interpreter with full cross-platform detection.
///
/// Detection order:
/// 1. If `configured_path` is `Some` and non-empty, validate and return
///    [`PerlInterpreterResult::ConfiguredPath`] (or [`PerlInterpreterResult::NotFound`]
///    if the configured path is broken). Does **not** fall back silently.
/// 2. Check toolchain managers (perlbrew, plenv) before PATH.
/// 3. Walk PATH; on Windows, prefer Strawberry/ActiveState over msys/Git Bash perls.
/// 4. If not on PATH, probe canonical OS install locations as a last resort.
/// 5. If still not found, return [`PerlInterpreterResult::NotFound`] with searched paths.
///
/// # Examples
///
/// ```rust
/// use perl_dap_platform::{find_perl_interpreter, PerlInterpreterResult};
/// match find_perl_interpreter(None) {
///     PerlInterpreterResult::FoundOnPath(p) => println!("Perl: {}", p.display()),
///     PerlInterpreterResult::NotFound { searched } => eprintln!("Not found. Searched: {:?}", searched),
///     _ => {}
/// }
/// ```
pub fn find_perl_interpreter(configured_path: Option<&str>) -> PerlInterpreterResult {
    // 1. Honour explicit config path — validate it exists, never silently fall back.
    if let Some(cfg) = configured_path.filter(|s| !s.is_empty()) {
        let p = PathBuf::from(cfg);
        if p.exists() && p.is_file() {
            return PerlInterpreterResult::ConfiguredPath(p);
        } else {
            return PerlInterpreterResult::NotFound {
                searched: vec![format!("configured path: {cfg}")],
            };
        }
    }

    let mut searched: Vec<String> = vec!["PATH".to_string()];

    // 2. Check toolchain managers (perlbrew, plenv) first.
    if let Some(path) = detect_perlbrew_perl() {
        return PerlInterpreterResult::FoundOnPath(path);
    }
    if let Some(path) = detect_plenv_perl() {
        return PerlInterpreterResult::FoundOnPath(path);
    }

    // 3. Walk PATH, ranking results on Windows.
    if let Ok(path_env) = env::var("PATH") {
        let ranked = find_all_perl_on_path(&path_env);
        if let Some(best) = ranked.into_iter().next() {
            return PerlInterpreterResult::FoundOnPath(best);
        }
    }

    // 4. Probe OS-specific fallback paths.
    for (path, label) in fallback_perl_paths() {
        searched.push(path.to_string_lossy().to_string());
        if path.exists() && path.is_file() {
            return PerlInterpreterResult::FoundViaFallback { path, label: label.to_string() };
        }
    }

    PerlInterpreterResult::NotFound { searched }
}

/// Resolve the perl binary path on the current platform.
///
/// Searches only the system `PATH`. For toolchain-aware resolution that
/// also checks perlbrew and plenv, use [`resolve_perl_path_with_toolchain`].
pub fn resolve_perl_path() -> Result<PathBuf> {
    let path_env = env::var("PATH").context("PATH environment variable not set")?;
    resolve_perl_path_from_path_env(&path_env)
}

pub(crate) fn resolve_perl_path_from_path_env(path_env: &str) -> Result<PathBuf> {
    for path_dir in path_env.split(PATH_SEPARATOR) {
        let perl_path = PathBuf::from(path_dir).join(PERL_EXECUTABLE);
        if perl_path.exists() && perl_path.is_file() {
            return Ok(perl_path);
        }
    }

    anyhow::bail!("perl binary not found on PATH. Please install Perl or add it to PATH.")
}

/// Resolve the Perl interpreter path, checking perlbrew and plenv before PATH.
///
/// Detection order:
/// 1. perlbrew -- check PERLBREW_PERL + PERLBREW_ROOT env vars.
/// 2. plenv -- check PLENV_VERSION + PLENV_ROOT env vars.
/// 3. System PATH -- delegate to resolve_perl_path().
///
/// # Errors
///
/// Returns an error only when all strategies fail to find a Perl binary.
pub fn resolve_perl_path_with_toolchain() -> Result<PathBuf> {
    if let Some(path) = detect_perlbrew_perl() {
        return Ok(path);
    }
    if let Some(path) = detect_plenv_perl() {
        return Ok(path);
    }
    resolve_perl_path()
}

/// Detect the active Perl interpreter managed by perlbrew.
///
/// Reads `PERLBREW_PERL` for the version name and `PERLBREW_ROOT` (or
/// `~/perl5/perlbrew` by default) for the installation root.
///
/// Returns `None` when env vars are absent or the binary path does not exist.
pub fn detect_perlbrew_perl() -> Option<PathBuf> {
    let version = env::var("PERLBREW_PERL").ok()?;
    if version.is_empty() {
        return None;
    }
    let root = perlbrew_root();
    let perl_bin = root.join("perls").join(&version).join("bin").join(PERL_EXECUTABLE);
    if perl_bin.exists() && perl_bin.is_file() { Some(perl_bin) } else { None }
}

/// Detect the active Perl interpreter managed by plenv.
///
/// Reads `PLENV_VERSION` for the version name and `PLENV_ROOT` (or
/// `~/.plenv` by default) for the installation root.
///
/// Returns `None` when env vars are absent or the binary path does not exist.
pub fn detect_plenv_perl() -> Option<PathBuf> {
    let version = env::var("PLENV_VERSION").ok()?;
    if version.is_empty() {
        return None;
    }
    let root = plenv_root();
    let perl_bin = root.join("versions").join(&version).join("bin").join(PERL_EXECUTABLE);
    if perl_bin.exists() && perl_bin.is_file() { Some(perl_bin) } else { None }
}

/// Return the perlbrew root directory (`PERLBREW_ROOT` or `~/perl5/perlbrew`).
fn perlbrew_root() -> PathBuf {
    if let Ok(root) = env::var("PERLBREW_ROOT") {
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    home_dir().join("perl5").join("perlbrew")
}

/// Return the plenv root directory (`PLENV_ROOT` or `~/.plenv`).
fn plenv_root() -> PathBuf {
    if let Ok(root) = env::var("PLENV_ROOT") {
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    home_dir().join(".plenv")
}

/// Return the user home directory, falling back to the OS temp directory.
///
/// Checks `HOME` (Unix) then `USERPROFILE` (Windows) before falling back to
/// [`std::env::temp_dir`]. The old fallback of `PathBuf::from("/tmp")` broke
/// on Windows where `/tmp` does not exist.
fn home_dir() -> PathBuf {
    if let Ok(home) = env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    if let Ok(profile) = env::var("USERPROFILE") {
        if !profile.is_empty() {
            return PathBuf::from(profile);
        }
    }
    std::env::temp_dir()
}

/// Normalize a file path for cross-platform compatibility.
pub fn normalize_path(path: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Some(path_str) = path.to_str()
            && path_str.starts_with("/mnt/")
            && path_str.len() > 6
        {
            let drive_letter = &path_str[5..6];
            let rest = &path_str[6..];
            let windows_path =
                format!("{}:{}", drive_letter.to_uppercase(), rest.replace('/', "\\"));
            return PathBuf::from(windows_path);
        }
    }

    #[cfg(windows)]
    {
        if let Some(path_str) = path.to_str() {
            if path_str.len() >= 2
                && path_str.chars().nth(1) == Some(':')
                && let Some(first_char) = path_str.chars().next()
            {
                let drive_letter = first_char.to_uppercase();
                let rest = &path_str[1..];
                return PathBuf::from(format!("{}{}", drive_letter, rest));
            }

            if path_str.starts_with("\\\\") {
                return path.to_path_buf();
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(canonical) = path.canonicalize() {
            return canonical;
        }
    }

    path.to_path_buf()
}

/// Setup environment variables for Perl execution.
pub fn setup_environment(include_paths: &[PathBuf]) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if !include_paths.is_empty() {
        let perl5lib = include_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(&PATH_SEPARATOR.to_string());

        env.insert("PERL5LIB".to_string(), perl5lib);
    }

    env
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use perl_tdd_support::{must, must_err};

    #[test]
    fn test_resolve_perl_path() {
        if let Ok(path) = resolve_perl_path() {
            assert!(path.exists());
            assert!(path.is_file());
        }
    }

    #[test]
    fn test_normalize_path_basic() {
        let normalized = normalize_path(&PathBuf::from("script.pl"));
        assert!(!normalized.as_os_str().is_empty());
    }

    #[test]
    fn test_setup_environment_empty() {
        let env = setup_environment(&[]);
        assert!(!env.contains_key("PERL5LIB"));
    }

    #[test]
    fn test_setup_environment_with_paths() {
        let env =
            setup_environment(&[PathBuf::from("/workspace/lib"), PathBuf::from("/custom/lib")]);
        assert!(env.contains_key("PERL5LIB"));
    }

    #[test]
    fn resolve_from_path_env_finds_perl_in_first_dir() {
        use std::fs;
        let tempdir = must(tempfile::tempdir());
        let bin = tempdir.path().join(PERL_EXECUTABLE);
        must(fs::write(&bin, ""));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = must(fs::metadata(&bin)).permissions();
            perms.set_mode(0o755);
            must(fs::set_permissions(&bin, perms));
        }
        let path_str = tempdir.path().to_string_lossy().to_string();
        let result = resolve_perl_path_from_path_env(&path_str);
        assert_eq!(must(result), bin);
    }

    #[test]
    fn resolve_from_path_env_empty_path_returns_error() {
        let result = resolve_perl_path_from_path_env("");
        assert!(result.is_err());
        let msg = format!("{}", must_err(result));
        assert!(
            msg.contains("perl") || msg.contains("PATH"),
            "error should mention perl/PATH: {msg}"
        );
    }

    #[test]
    fn resolve_from_path_env_no_perl_on_path_returns_error() {
        let tempdir = must(tempfile::tempdir());
        let path_str = tempdir.path().to_string_lossy().to_string();
        let result = resolve_perl_path_from_path_env(&path_str);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn normalize_path_wsl_mnt_translated_to_windows_style() {
        let wsl_path = std::path::Path::new("/mnt/c/Users/user/script.pl");
        let normalized = normalize_path(wsl_path);
        let s = normalized.to_string_lossy();
        assert!(
            s.starts_with("C:\\") || s.starts_with("C:/"),
            "expected Windows-style path, got: {s}"
        );
        assert!(s.contains("Users"), "path content preserved: {s}");
    }

    #[test]
    fn normalize_path_non_wsl_unix_path_unchanged_on_linux() {
        let path = std::path::Path::new("/usr/local/bin/perl");
        let normalized = normalize_path(path);
        assert!(
            !normalized.to_string_lossy().contains('\\'),
            "non-WSL path should not be Windows-escaped"
        );
    }

    // ── find_perl_interpreter tests ────────────────────────────────────────

    #[test]
    fn find_perl_interpreter_configured_path_valid_returns_configured() {
        use std::fs;
        let tempdir = must(tempfile::tempdir());
        let fake_perl = tempdir.path().join(PERL_EXECUTABLE);
        must(fs::write(&fake_perl, ""));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = must(fs::metadata(&fake_perl)).permissions();
            perms.set_mode(0o755);
            must(fs::set_permissions(&fake_perl, perms));
        }
        let path_str = fake_perl.to_string_lossy().to_string();
        let result = find_perl_interpreter(Some(&path_str));
        assert!(
            matches!(result, PerlInterpreterResult::ConfiguredPath(_)),
            "expected ConfiguredPath, got: {result:?}"
        );
    }

    #[test]
    fn find_perl_interpreter_configured_path_missing_returns_not_found() {
        let result = find_perl_interpreter(Some("/nonexistent/path/to/perl"));
        match result {
            PerlInterpreterResult::NotFound { searched } => {
                assert!(
                    searched.iter().any(|s| s.contains("configured")),
                    "searched list should mention configured path: {searched:?}"
                );
            }
            other => panic!("expected NotFound, got: {other:?}"),
        }
    }

    #[test]
    fn find_perl_interpreter_empty_config_falls_back_to_path_detection() {
        // Empty string should be treated as "not configured"
        let result = find_perl_interpreter(Some(""));
        // Should not return ConfiguredPath for empty string
        assert!(
            !matches!(result, PerlInterpreterResult::ConfiguredPath(_)),
            "empty config should fall back to path detection"
        );
    }

    #[test]
    fn find_perl_interpreter_none_config_performs_detection() {
        // With no config, should return Found* or NotFound (never ConfiguredPath)
        let result = find_perl_interpreter(None);
        assert!(
            !matches!(result, PerlInterpreterResult::ConfiguredPath(_)),
            "None config should not return ConfiguredPath"
        );
    }

    #[test]
    fn find_perl_interpreter_not_found_includes_searched_paths() {
        // When configured path is broken, NotFound should list what was searched
        let result = find_perl_interpreter(Some("/absolutely/not/a/real/path/perl"));
        if let PerlInterpreterResult::NotFound { searched } = result {
            assert!(!searched.is_empty(), "searched list should not be empty");
        }
        // If Perl is found on the system, the above test doesn't apply — that's fine
    }

    #[test]
    #[cfg(windows)]
    fn windows_perl_rank_strawberry_is_best() {
        let strawberry = std::path::Path::new(r"C:\Strawberry\perl\bin\perl.exe");
        let msys = std::path::Path::new(r"C:\Program Files\Git\usr\bin\perl.exe");
        assert!(
            windows_perl_rank(strawberry) < windows_perl_rank(msys),
            "Strawberry should rank better than msys perl"
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_perl_rank_activestate_beats_msys() {
        let active = std::path::Path::new(r"C:\Perl64\bin\perl.exe");
        let msys = std::path::Path::new(r"C:\Program Files\Git\usr\bin\perl.exe");
        assert!(
            windows_perl_rank(active) < windows_perl_rank(msys),
            "ActiveState should rank better than msys perl"
        );
    }

    #[test]
    fn home_dir_fallback_uses_temp_dir() {
        // When both HOME and USERPROFILE are absent, home_dir() must return
        // std::env::temp_dir() — not a hardcoded PathBuf::from("/tmp") which
        // does not exist on Windows.
        let original_home = std::env::var("HOME").ok();
        let original_userprofile = std::env::var("USERPROFILE").ok();

        // SAFETY: single-threaded test; no other threads reading these vars.
        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("USERPROFILE");
        }

        let result = home_dir();
        let expected = std::env::temp_dir();

        // Restore env vars.
        unsafe {
            if let Some(val) = original_home {
                std::env::set_var("HOME", val);
            }
            if let Some(val) = original_userprofile {
                std::env::set_var("USERPROFILE", val);
            }
        }

        // The fallback must match std::env::temp_dir(), not a hardcoded path.
        assert_eq!(
            result, expected,
            "home_dir() fallback should be std::env::temp_dir(), got {result:?}"
        );
        assert!(!result.as_os_str().is_empty(), "home_dir() must return a non-empty path");
    }
}
