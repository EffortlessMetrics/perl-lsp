//! Environment variable helpers for UX scenario tests.
//!
//! These helpers manipulate the *child process environment* (by building an
//! environment for `Command::spawn`) rather than the test-runner process.
//! This avoids the process-global mutation that `std::env::set_var` requires
//! and sidesteps the `unsafe` requirement.

use std::collections::HashMap;
use std::ffi::OsString;

/// A restricted PATH that can be passed to a child process.
///
/// Call `build_path()` to get the value to pass to `Command::env("PATH", ...)`.
#[derive(Debug, Clone, Default)]
pub struct RestrictedPath {
    dirs: Vec<String>,
}

impl RestrictedPath {
    /// Create an empty PATH (nothing on it — simulates a completely bare environment).
    pub fn empty() -> Self {
        Self { dirs: Vec::new() }
    }

    /// Create a PATH containing only the given directories.
    pub fn only(dirs: Vec<String>) -> Self {
        Self { dirs }
    }

    /// Create a PATH that contains everything EXCEPT entries containing `exclude_pattern`.
    /// Useful for "remove perltidy from PATH" without a full directory listing.
    pub fn current_excluding(exclude_pattern: &str) -> Self {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ';' } else { ':' };
        let dirs = current_path
            .split(sep)
            .filter(|entry| !entry.contains(exclude_pattern))
            .map(String::from)
            .collect();
        Self { dirs }
    }

    /// Get the PATH value as an `OsString` for `Command::env`.
    pub fn build_path(&self) -> OsString {
        let sep = if cfg!(windows) { ";" } else { ":" };
        OsString::from(self.dirs.join(sep))
    }

    /// True if this PATH contains no directories.
    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty()
    }
}

/// A set of environment overrides to apply to a child process.
///
/// Does not mutate the test runner process environment.
#[derive(Debug, Clone, Default)]
pub struct EnvOverride {
    set: HashMap<String, OsString>,
    unset: Vec<String>,
}

impl EnvOverride {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an environment variable for the child process.
    pub fn set(mut self, key: impl Into<String>, value: impl Into<OsString>) -> Self {
        self.set.insert(key.into(), value.into());
        self
    }

    /// Unset an environment variable in the child process.
    pub fn unset(mut self, key: impl Into<String>) -> Self {
        self.unset.push(key.into());
        self
    }

    /// Apply these overrides to a `std::process::Command`.
    pub fn apply(&self, cmd: &mut std::process::Command) {
        for (k, v) in &self.set {
            cmd.env(k, v);
        }
        for k in &self.unset {
            cmd.env_remove(k);
        }
    }
}

/// RAII guard that is currently a no-op (env changes are child-process-only).
/// Kept for API symmetry with older test helpers.
pub struct PathGuard;

impl PathGuard {
    /// Builds a `RestrictedPath` that excludes any PATH entry containing
    /// `tool_name`, so the tool appears to be missing for the child process.
    pub fn excluding_tool(tool_name: &str) -> (RestrictedPath, Self) {
        (RestrictedPath::current_excluding(tool_name), Self)
    }
}
