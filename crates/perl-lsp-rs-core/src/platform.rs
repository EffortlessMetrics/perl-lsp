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

/// Resolve the perl binary path by searching the system `PATH`.
///
/// # Security
///
/// Empty and relative `PATH` entries are skipped.  An empty entry (`;;` or a
/// trailing `;`) causes `PathBuf::from("").join("perl.exe")` to produce a
/// relative path (`"perl.exe"`), which `.exists()` resolves against the process
/// CWD (the LSP workspace root) — the same binary-planting vector as the
/// bare-name `Command::new` RCE (#2764/#3028).  A relative entry is equally
/// dangerous and almost always a misconfiguration.  Only absolute, non-empty
/// directory entries are consulted.
///
/// # Errors
///
/// Returns an error when perl cannot be found on PATH.
pub fn resolve_perl_path() -> Result<PathBuf> {
    if let Ok(path_env) = env::var("PATH") {
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
                return Ok(perl_path);
            }
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

    // --- Empty-PATH-entry bypass regression (#3028) ---
    //
    // An empty `;;` or trailing `;` PATH entry makes
    // `PathBuf::from("").join("perl.exe")` produce a relative candidate
    // ("perl.exe") that `.exists()` resolves against the process CWD.
    // If the LSP workspace root contains a planted `perl.exe`, the old code
    // would return it as the resolved interpreter — the same binary-planting
    // RCE as the bare-name `Command::new` fix.  These tests verify that
    // empty and relative PATH entries are skipped unconditionally.

    /// A PATH that consists only of an empty entry must NOT return the CWD
    /// binary — the result must be `Err` (tool not found), never the relative
    /// candidate.
    ///
    /// This test mutates `PATH`, so it must hold the env lock for its duration.
    #[test]
    fn resolve_perl_path_skips_empty_path_entry() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

        let original = std::env::var("PATH").ok();

        // SAFETY: test controls process env for the duration of this test.
        unsafe {
            // A PATH with only an empty entry (equivalent to `;;` reduced to
            // just the empty string after splitting).
            std::env::set_var("PATH", "");
        }

        let result = resolve_perl_path();

        unsafe {
            match original {
                Some(val) => std::env::set_var("PATH", val),
                None => std::env::remove_var("PATH"),
            }
        }

        // With only an empty PATH entry, no absolute directory is searched —
        // the result must be Err (not a relative/CWD candidate).
        assert!(
            result.is_err(),
            "resolve_perl_path with an empty PATH entry must return Err, \
             not a CWD-relative candidate; got: {result:?}"
        );
    }

    /// A PATH with an empty entry alongside a real PATH entry must still find
    /// the real binary — skipping empty entries must not block legitimate ones.
    #[test]
    fn resolve_perl_path_skips_empty_entry_but_finds_real_perl() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

        let original = std::env::var("PATH").ok();

        // Only run this test when perl is actually on PATH — otherwise there is
        // nothing to find and the assertion is vacuous.
        let real_path = match resolve_perl_path() {
            Ok(p) => p,
            Err(_) => return, // no perl on host — skip
        };
        let real_dir = match real_path.parent() {
            Some(d) => d.to_path_buf(),
            None => return,
        };

        // Inject an empty entry before the real directory.
        let path_sep = if cfg!(windows) { ";" } else { ":" };
        let new_path = format!("{}{}{}", path_sep, path_sep, real_dir.display());

        unsafe {
            // SAFETY: test controls process env; ENV_LOCK serializes access.
            std::env::set_var("PATH", &new_path);
        }

        let result = resolve_perl_path();

        unsafe {
            match original {
                Some(val) => std::env::set_var("PATH", val),
                None => std::env::remove_var("PATH"),
            }
        }

        assert!(
            result.is_ok(),
            "resolve_perl_path must find perl even when PATH has empty entries; got: {result:?}"
        );
        let found = result.expect("checked above");
        assert!(
            found.is_absolute(),
            "resolved perl path must be absolute, not a CWD-relative candidate; got: {found:?}"
        );
    }
}
