//! Perl interpreter detection utilities for LSP runtime toolchain awareness.
//!
//! Extracted from `perl-dap::platform` to break the config→dap cycle
//! and serve as a stable, reusable service layer for both LSP and DAP consumers.

use anyhow::Result;
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

/// Resolve the Perl interpreter path, checking perlbrew and plenv before PATH.
///
/// Detection order:
/// 1. perlbrew -- check PERLBREW_PERL + PERLBREW_ROOT env vars.
/// 2. plenv -- check PLENV_VERSION + PLENV_ROOT env vars.
/// 3. System PATH -- delegate to system PATH search.
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

/// Search a PATH-separator-delimited string for a Perl binary, applying the
/// empty-entry and relative-entry RCE guard.
///
/// This is the testable inner core of [`resolve_perl_path`].  It accepts the
/// raw value of a `PATH`-like string rather than reading the process
/// environment, so callers can inject a fully controlled value in tests without
/// mutating global state (which races other parallel test threads).
///
/// # Security
///
/// Empty and relative entries are skipped.  An empty entry (`;;` or a trailing
/// `;`) causes `PathBuf::from("").join("perl.exe")` to produce a relative path
/// (`"perl.exe"`), which `.exists()` resolves against the process CWD — the
/// same binary-planting vector as the bare-name `Command::new` RCE
/// (#2764/#3028).  A relative entry is equally dangerous.  Only absolute,
/// non-empty directory entries are consulted.
fn scan_path_for_perl(path_env: &str) -> Option<PathBuf> {
    for path_dir in path_env.split(PATH_SEPARATOR) {
        // Skip empty entries (from `;;` or trailing `;`) and relative entries.
        // Both produce CWD-relative paths via `.join(PERL_EXECUTABLE)` and
        // could resolve to a planted binary in the workspace root.
        let dir = PathBuf::from(path_dir);
        if path_dir.is_empty() || !dir.is_absolute() {
            continue;
        }
        let perl_path = dir.join(PERL_EXECUTABLE);
        if perl_path.exists() && perl_path.is_file() {
            return Some(perl_path);
        }
    }
    None
}

/// Resolve the perl binary path by searching the system `PATH`.
///
/// Delegates the entry-filtering logic to [`scan_path_for_perl`], which skips
/// empty and relative `PATH` entries (the RCE guard, #2764/#3028), then falls
/// back to hard-coded Termux candidates.
///
/// # Errors
///
/// Returns an error when perl cannot be found on PATH.
pub fn resolve_perl_path() -> Result<PathBuf> {
    if let Ok(path_env) = env::var("PATH") {
        if let Some(path) = scan_path_for_perl(&path_env) {
            return Ok(path);
        }
    }

    for perl_path in termux_perl_candidates() {
        if perl_path.exists() && perl_path.is_file() {
            return Ok(perl_path);
        }
    }

    anyhow::bail!(perl_not_found_install_message())
}

/// Candidate Perl locations used by Termux environments when PATH is minimal.
fn termux_perl_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(prefix) = env::var("PREFIX") {
        if !prefix.is_empty() {
            candidates.push(PathBuf::from(prefix).join("bin").join(PERL_EXECUTABLE));
        }
    }
    candidates.push(PathBuf::from("/data/data/com.termux/files/usr/bin").join(PERL_EXECUTABLE));
    candidates
}

/// End-user remediation guidance shown when no Perl interpreter is available.
fn perl_not_found_install_message() -> &'static str {
    "perl binary not found on PATH. Install Perl via https://strawberryperl.com (Windows), \
`brew install perl` (macOS), your distro package manager, or `pkg install perl` on Termux, then add it to PATH."
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
/// [`std::env::temp_dir`].
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

#[cfg(test)]
#[allow(clippy::panic, unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn home_dir_fallback_uses_temp_dir() {
        let original_home = std::env::var("HOME").ok();
        let original_userprofile = std::env::var("USERPROFILE").ok();

        // SAFETY: single-threaded test; no other threads reading these vars.
        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("USERPROFILE");
        }

        let result = home_dir();
        let expected = std::env::temp_dir();

        unsafe {
            if let Some(val) = original_home {
                std::env::set_var("HOME", val);
            }
            if let Some(val) = original_userprofile {
                std::env::set_var("USERPROFILE", val);
            }
        }

        assert_eq!(
            result, expected,
            "home_dir() fallback should be std::env::temp_dir(), got {result:?}"
        );
        assert!(!result.as_os_str().is_empty(), "home_dir() must return a non-empty path");
    }

    #[test]
    fn termux_candidates_include_prefix_bin_perl() {
        let original_prefix = std::env::var("PREFIX").ok();

        // SAFETY: test controls process env for the duration of this test.
        unsafe {
            std::env::set_var("PREFIX", "/data/data/com.termux/files/usr");
        }

        let candidates = termux_perl_candidates();

        if let Some(val) = original_prefix {
            // SAFETY: restore captured test environment value.
            unsafe {
                std::env::set_var("PREFIX", val);
            }
        } else {
            // SAFETY: restore environment to original unset state.
            unsafe {
                std::env::remove_var("PREFIX");
            }
        }

        assert!(
            candidates.iter().any(|p| p
                == &PathBuf::from("/data/data/com.termux/files/usr/bin").join(PERL_EXECUTABLE)),
            "Termux PREFIX candidate should include $PREFIX/bin/{PERL_EXECUTABLE}: {candidates:?}"
        );
    }

    #[test]
    fn install_message_mentions_termux_pkg_command() {
        let msg = perl_not_found_install_message();
        assert!(
            msg.contains("pkg install perl"),
            "install guidance should mention Termux package install command: {msg}"
        );
    }

    #[test]
    fn resolve_perl_path_returns_existing_binary_or_error() {
        match resolve_perl_path() {
            Ok(path) => {
                assert!(path.exists());
                assert!(path.is_file());
            }
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("perl") || msg.contains("PATH"),
                    "error should mention perl/PATH: {msg}"
                );
                assert!(
                    msg.contains("strawberryperl.com"),
                    "error should include install guidance: {msg}"
                );
            }
        }
    }

    // --- Empty-PATH-entry bypass regression (#3028 / #3222) ---
    //
    // An empty `;;` or trailing `;` PATH entry makes
    // `PathBuf::from("").join("perl.exe")` produce a relative candidate
    // ("perl.exe") that `.exists()` resolves against the process CWD.
    // If the LSP workspace root contains a planted `perl.exe`, the old code
    // would return it as the resolved interpreter — the same binary-planting
    // RCE as the bare-name `Command::new` fix.  These tests verify that
    // empty and relative PATH entries are skipped unconditionally.
    //
    // Both tests call the injectable `scan_path_for_perl` inner function with a
    // fully controlled PATH string.  No global env mutation means no race
    // condition between parallel test threads (#3222).

    /// A PATH that consists only of empty entries must NOT return any candidate
    /// — the RCE guard must fire.
    ///
    /// Uses the injectable `scan_path_for_perl` core — no env mutation, no race.
    #[test]
    fn resolve_perl_path_skips_empty_path_entry() {
        // An empty string: split by the separator produces one empty component.
        let result = scan_path_for_perl("");
        assert!(result.is_none(), "empty PATH string must yield None (RCE guard); got: {result:?}");

        // Platform-native double-separator (;; Windows / :: Unix): two empty
        // components and one empty trailing component — all skipped.
        let double_sep = if cfg!(windows) { ";;" } else { "::" };
        let result = scan_path_for_perl(double_sep);
        assert!(
            result.is_none(),
            "only-empty PATH ({double_sep:?}) must yield None (RCE guard); got: {result:?}"
        );
    }

    /// A PATH with empty entries alongside a real absolute directory must still
    /// find the binary — skipping empty entries must not block legitimate ones.
    ///
    /// Uses a tempdir-based fake perl binary so the test does not depend on a
    /// real `perl` binary being present on the host.  No env mutation — the
    /// path string is injected directly into `scan_path_for_perl` (#3222).
    #[test]
    fn resolve_perl_path_skips_empty_entry_but_finds_real_perl() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let perl_bin = dir.path().join(PERL_EXECUTABLE);
        std::fs::write(&perl_bin, b"")?;

        // Make the file executable on Unix so it passes an `is_file()` check
        // (it is always a regular file on Windows regardless of permissions).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&perl_bin)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&perl_bin, perms)?;
        }

        // Inject empty entries before the real directory — the guard must skip
        // them and still find the binary.
        let sep = if cfg!(windows) { ";" } else { ":" };
        let path_env = format!("{sep}{sep}{}", dir.path().display());

        let result = scan_path_for_perl(&path_env);
        assert!(
            result.is_some(),
            "must find perl even with empty entries before real dir; \
             path_env={path_env:?}"
        );
        let found = result.expect("checked above");
        assert!(
            found.is_absolute(),
            "resolved perl path must be absolute, not CWD-relative; got: {found:?}"
        );

        Ok(())
    }
}
