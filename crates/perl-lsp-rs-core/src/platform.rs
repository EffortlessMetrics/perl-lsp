//! Perl interpreter detection utilities for LSP runtime toolchain awareness.
//!
//! Extracted from `perl-dap::platform` to break the config→dap cycle
//! and serve as a stable, reusable service layer for both LSP and DAP consumers.

use anyhow::{Context, Result};
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
/// # Errors
///
/// Returns an error when perl cannot be found on PATH.
pub fn resolve_perl_path() -> Result<PathBuf> {
    let path_env = env::var("PATH").context("PATH environment variable not set")?;
    for path_dir in path_env.split(PATH_SEPARATOR) {
        let perl_path = PathBuf::from(path_dir).join(PERL_EXECUTABLE);
        if perl_path.exists() && perl_path.is_file() {
            return Ok(perl_path);
        }
    }
    anyhow::bail!("perl binary not found on PATH. Please install Perl or add it to PATH.")
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
            }
        }
    }
}
