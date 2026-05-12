//! Controlled subprocess environment for Perl oracle seams.
//!
//! `PerlOracleEnv` is the canonical way to spawn a Perl subprocess from
//! within `perl-lsp`. It implements an explicit deny-all-ambient policy:
//! every environment variable that can reach the subprocess must be
//! explicitly listed in the allow-set. This prevents ambient state (e.g.
//! `PERL5LIB`, `PERL5OPT`, `HOME`, `local::lib` activation variables) from
//! silently undermining the user's workspace configuration.
//!
//! ## Architecture
//!
//! See `docs/architecture/perl-subprocess-seams.md` for the full seam model
//! and internalization-path classification.
//!
//! The 2026-05-11 #8493 incident (the startup `@INC` probe inherited
//! `PERL5LIB` from the LSP process environment even when `usePerl5lib=false`)
//! is the canonical motivation for this module.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use perl_lsp_rs_core::config::{PerlOracleEnv, WorkspaceConfig};
//!
//! let config = WorkspaceConfig::default();
//! if let Some(oracle) = PerlOracleEnv::for_startup_inc_probe(&config) {
//!     let mut cmd = oracle.into_command();
//!     cmd.args(["-e", "print join(\"\\n\", @INC)"]);
//! }
//! ```

#[cfg(not(target_arch = "wasm32"))]
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use super::WorkspaceConfig;

/// Controlled subprocess environment for a single Perl oracle seam.
///
/// `PerlOracleEnv` enforces a deny-all-ambient policy: the subprocess
/// command produced by [`into_command`] starts from an empty environment
/// and adds back only the explicitly allowlisted variables.
///
/// Construct with one of the named constructors (e.g.
/// [`for_startup_inc_probe`]) and then call [`into_command`] to get a
/// `std::process::Command` ready for the subprocess.
///
/// [`into_command`]: PerlOracleEnv::into_command
/// [`for_startup_inc_probe`]: PerlOracleEnv::for_startup_inc_probe
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct PerlOracleEnv {
    /// Absolute path to the Perl binary. Prefer an absolute path to avoid
    /// `PATH`-based resolution ambiguity (perlbrew shims, plenv, etc.).
    pub perl_binary: PathBuf,
    /// Working directory for the subprocess. Explicit; never inherited.
    pub cwd: PathBuf,
    /// Wall-clock timeout for the subprocess.
    pub timeout: Duration,
    /// Whether to pass `PERL5LIB` through to the subprocess.
    ///
    /// When `false` (default for most oracle seams), `PERL5LIB` is stripped
    /// even if it is set in the parent process environment.
    pub allow_perl5lib: bool,
    /// Whether to pass `PERL5OPT` through to the subprocess.
    ///
    /// Defaults to `false`. `PERL5OPT` injects command-line options into
    /// every Perl invocation and can cause oracle results to differ from a
    /// clean interpreter run.
    pub allow_perl5opt: bool,
    /// Whether to pass `local::lib` activation variables through.
    ///
    /// Controls `PERL_LOCAL_LIB_ROOT` (and implicitly
    /// `PERL_LOCAL_LIB_PREFIX`). Defaults to `false`.
    pub allow_local_lib: bool,
    /// Call-site-specific environment additions.
    ///
    /// Applied after the allow/deny pass, so these entries are unconditionally
    /// present in the subprocess environment regardless of the `allow_*` flags.
    /// Useful for per-invocation overrides (e.g. a controlled `HOME` value for
    /// a test fixture).
    pub extra_env: BTreeMap<String, String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl PerlOracleEnv {
    /// Build a [`Command`] with ALL non-allowlisted env vars stripped.
    ///
    /// The sequence is:
    /// 1. `Command::env_clear()` — drop the entire parent environment.
    /// 2. Re-add `PATH` unconditionally (needed for interpreter resolution
    ///    even when `perl_binary` is absolute, e.g. for `require`/`use` hooks
    ///    that fork sub-processes).
    /// 3. Re-add allowlisted Perl env vars if their `allow_*` flag is set.
    /// 4. Apply `extra_env` unconditionally (call-site overrides).
    ///
    /// The working directory is set to `self.cwd` explicitly so the subprocess
    /// never inherits the LSP process's cwd.
    pub fn into_command(&self) -> Command {
        let mut cmd = Command::new(&self.perl_binary);

        // 1. Clear entire parent environment — deny-all-ambient policy.
        cmd.env_clear();

        // 2. PATH: preserved so the interpreter can resolve its own helpers
        //    and module hooks that fork sub-processes. Without PATH many system
        //    Perl installations silently break.
        if let Some(path_val) = std::env::var_os("PATH") {
            cmd.env("PATH", path_val);
        }

        // 3. Conditionally allowlisted Perl env vars.
        if self.allow_perl5lib {
            if let Some(val) = std::env::var_os("PERL5LIB") {
                cmd.env("PERL5LIB", val);
            }
        }
        if self.allow_perl5opt {
            if let Some(val) = std::env::var_os("PERL5OPT") {
                cmd.env("PERL5OPT", val);
            }
        }
        if self.allow_local_lib {
            if let Some(val) = std::env::var_os("PERL_LOCAL_LIB_ROOT") {
                cmd.env("PERL_LOCAL_LIB_ROOT", val);
            }
            if let Some(val) = std::env::var_os("PERL_LOCAL_LIB_PREFIX") {
                cmd.env("PERL_LOCAL_LIB_PREFIX", val);
            }
        }

        // 4. Call-site-specific additions (unconditional).
        for (k, v) in &self.extra_env {
            cmd.env(k, v);
        }

        // Explicit cwd — never inherit.
        cmd.current_dir(&self.cwd);

        cmd
    }

    /// Constructor for the startup `@INC` probe.
    ///
    /// Reads relevant settings from `config`:
    ///
    /// - `perl_binary`: resolved from `config.perl_path` or falls back to
    ///   the toolchain resolver.
    /// - `allow_perl5lib`: mirrors `config.use_perl5lib` — the user's
    ///   explicit choice about whether `PERL5LIB` should affect `@INC`.
    /// - `allow_perl5opt`: always `false` (PERL5OPT is not relevant to the
    ///   `@INC` probe contract).
    /// - `allow_local_lib`: always `false` for the startup probe; `local::lib`
    ///   activation is not part of the declared seam contract.
    /// - `timeout`: defaults to 1 second (matches `SYSTEM_INC_PROBE_TIMEOUT`).
    /// - `cwd`: current working directory of the LSP process (best-effort;
    ///   the startup probe does not depend on cwd).
    /// - `extra_env`: empty.
    ///
    /// Returns `None` if the Perl binary cannot be resolved. The caller
    /// (`fetch_perl_inc`) already handles the `None` case by returning
    /// `Vec::new()`.
    pub fn for_startup_inc_probe(config: &WorkspaceConfig) -> Option<Self> {
        use crate::platform::resolve_perl_path_with_toolchain;

        let perl_binary = match config.perl_path.as_deref().filter(|p| !p.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => match resolve_perl_path_with_toolchain() {
                Ok(path) => path,
                Err(_) => return None,
            },
        };

        // Fall back to the process cwd; the startup probe does not depend on
        // it so any stable directory is fine.
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Some(Self {
            perl_binary,
            cwd,
            timeout: Duration::from_millis(1000),
            allow_perl5lib: config.use_perl5lib,
            allow_perl5opt: false,
            allow_local_lib: false,
            extra_env: BTreeMap::new(),
        })
    }

    /// Constructor for user-triggered `executeCommand` invocations (`perl.runFile`,
    /// `perl.runTestSub`).
    ///
    /// Unlike the startup `@INC` probe, these are user-explicit commands whose
    /// scripts may legitimately rely on `PERL5OPT` and `local::lib`. The env
    /// contract therefore differs:
    ///
    /// - `allow_perl5lib`: mirrors `config.use_perl5lib` (user's explicit choice).
    /// - `allow_perl5opt`: always `true` — user scripts may use `-M` pragmas.
    /// - `allow_local_lib`: always `true` — user's `local::lib` setup should be
    ///   available when they run their own scripts.
    /// - `timeout`: 30 seconds (matches the existing execute-command budget).
    /// - `cwd`: falls back to the LSP process cwd; callers may pass a more
    ///   specific directory (e.g., a workspace root).
    /// - `extra_env`: empty.
    ///
    /// Returns `None` if the Perl binary cannot be resolved. The caller should
    /// fall back to a plain `Command::new("perl")` or surface an error.
    pub fn for_execute_command(config: &WorkspaceConfig, cwd: PathBuf) -> Option<Self> {
        use crate::platform::resolve_perl_path_with_toolchain;

        let perl_binary = match config.perl_path.as_deref().filter(|p| !p.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => match resolve_perl_path_with_toolchain() {
                Ok(path) => path,
                Err(_) => return None,
            },
        };

        Some(Self {
            perl_binary,
            cwd,
            timeout: Duration::from_secs(30),
            allow_perl5lib: config.use_perl5lib,
            allow_perl5opt: true,
            allow_local_lib: true,
            extra_env: BTreeMap::new(),
        })
    }
}

// ── WASM stub ─────────────────────────────────────────────────────────────────

/// Stub for WASM targets where subprocess spawning is not available.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
pub struct PerlOracleEnv;

#[cfg(target_arch = "wasm32")]
impl PerlOracleEnv {
    /// Returns `None` on WASM (no subprocess support).
    pub fn for_startup_inc_probe(_config: &WorkspaceConfig) -> Option<Self> {
        None
    }

    /// Returns `None` on WASM (no subprocess support).
    pub fn for_execute_command(
        _config: &WorkspaceConfig,
        _cwd: std::path::PathBuf,
    ) -> Option<Self> {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(unsafe_code)] // required for std::env::set_var/remove_var in Rust 2024 (unsafe fn)
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Helper: build a minimal `PerlOracleEnv` for unit tests that don't
    /// need a real Perl binary.
    fn dummy_env(
        allow_perl5lib: bool,
        allow_perl5opt: bool,
        allow_local_lib: bool,
    ) -> PerlOracleEnv {
        PerlOracleEnv {
            perl_binary: PathBuf::from("perl"),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            timeout: Duration::from_millis(1000),
            allow_perl5lib,
            allow_perl5opt,
            allow_local_lib,
            extra_env: BTreeMap::new(),
        }
    }

    /// Inspect the env vars that `into_command()` would pass to the subprocess.
    ///
    /// `Command` doesn't expose a direct getter for its envs on stable Rust, so
    /// we extract them via the Debug representation — but that's fragile.
    /// Instead we spawn a real Perl subprocess that prints its env and check the
    /// output.  Tests that don't need a real Perl binary use `dummy_env` and
    /// assert on the struct fields.
    fn perl_path() -> Option<std::path::PathBuf> {
        crate::platform::resolve_perl_path_with_toolchain().ok()
    }

    // ── struct-level flag tests (no subprocess needed) ────────────────────────

    /// `for_execute_command` maps config flags correctly:
    /// - `allow_perl5lib` = `config.use_perl5lib`
    /// - `allow_perl5opt` = always `true`
    /// - `allow_local_lib` = always `true`
    #[test]
    fn for_execute_command_respects_config_flags() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let mut config = WorkspaceConfig::default();
        config.use_perl5lib = true;
        let env = PerlOracleEnv::for_execute_command(&config, cwd.clone());
        if let Some(e) = env {
            assert!(e.allow_perl5lib, "allow_perl5lib must mirror config.use_perl5lib=true");
            assert!(e.allow_perl5opt, "allow_perl5opt must always be true for execute-command");
            assert!(e.allow_local_lib, "allow_local_lib must always be true for execute-command");
        }

        config.use_perl5lib = false;
        let env = PerlOracleEnv::for_execute_command(&config, cwd);
        if let Some(e) = env {
            assert!(!e.allow_perl5lib, "allow_perl5lib must mirror config.use_perl5lib=false");
            assert!(e.allow_perl5opt, "allow_perl5opt must always be true for execute-command");
            assert!(e.allow_local_lib, "allow_local_lib must always be true for execute-command");
        }
    }

    /// `for_startup_inc_probe` maps `config.use_perl5lib` → `allow_perl5lib`.
    #[test]
    fn for_startup_inc_probe_respects_config_flags() {
        let mut config = WorkspaceConfig::default();

        config.use_perl5lib = true;
        let env = PerlOracleEnv::for_startup_inc_probe(&config);
        if let Some(e) = env {
            assert!(e.allow_perl5lib, "allow_perl5lib should be true when use_perl5lib=true");
            assert!(!e.allow_perl5opt, "allow_perl5opt must always be false for startup probe");
            assert!(!e.allow_local_lib, "allow_local_lib must always be false for startup probe");
        }

        config.use_perl5lib = false;
        let env = PerlOracleEnv::for_startup_inc_probe(&config);
        if let Some(e) = env {
            assert!(!e.allow_perl5lib, "allow_perl5lib should be false when use_perl5lib=false");
        }
    }

    /// `for_startup_inc_probe` with `usePerl5lib=false` must strip PERL5LIB:
    /// regression guard for the #8493 incident.
    #[test]
    fn for_startup_inc_probe_strips_when_use_perl5lib_false() -> TestResult {
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()), // no perl — skip
        };

        // Override perl_binary so the Oracle actually runs.
        let mut config = WorkspaceConfig::default();
        config.use_perl5lib = false;
        config.perl_path = Some(perl.to_string_lossy().into_owned());

        let oracle = PerlOracleEnv::for_startup_inc_probe(&config)
            .ok_or("for_startup_inc_probe returned None unexpectedly")?;

        // Set PERL5LIB in the parent process and assert the subprocess does NOT
        // inherit it when allow_perl5lib=false.
        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

        // SAFETY: test-only; RUST_TEST_THREADS=2 keeps test parallelism bounded.
        // We restore immediately after the subprocess spawns.
        unsafe { std::env::set_var("PERL5LIB", &poison_path) };

        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print $ENV{PERL5LIB} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;
        let stdout = String::from_utf8_lossy(&out.stdout);

        unsafe { std::env::remove_var("PERL5LIB") };

        assert!(
            !stdout.contains(&poison_path),
            "PERL5LIB poison ({poison_path}) must NOT appear in subprocess output \
             when allow_perl5lib=false; got: {stdout:?}",
        );
        assert!(
            stdout.trim() == "UNSET",
            "subprocess should see PERL5LIB as unset when allow_perl5lib=false; got: {stdout:?}",
        );
        Ok(())
    }

    // ── subprocess-level poisoned-env tests (require Perl) ───────────────────

    /// PERL5LIB is stripped by default (`allow_perl5lib=false`).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn perl_oracle_env_strips_perl5lib_by_default() -> TestResult {
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

        let mut oracle = dummy_env(false, false, false);
        oracle.perl_binary = perl;

        unsafe { std::env::set_var("PERL5LIB", &poison_path) };
        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print $ENV{PERL5LIB} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;
        unsafe { std::env::remove_var("PERL5LIB") };

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            "UNSET",
            "PERL5LIB must be stripped when allow_perl5lib=false; got: {stdout:?}",
        );
        Ok(())
    }

    /// PERL5LIB passes through when `allow_perl5lib=true`.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn perl_oracle_env_allows_perl5lib_when_opted_in() -> TestResult {
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

        let mut oracle = dummy_env(true, false, false);
        oracle.perl_binary = perl;

        unsafe { std::env::set_var("PERL5LIB", &poison_path) };
        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print $ENV{PERL5LIB} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;
        unsafe { std::env::remove_var("PERL5LIB") };

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(&poison_path),
            "PERL5LIB must be passed through when allow_perl5lib=true; got: {stdout:?}",
        );
        Ok(())
    }

    /// PERL5OPT is always stripped (no `allow_perl5opt` flag is true).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn perl_oracle_env_strips_perl5opt() -> TestResult {
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let mut oracle = dummy_env(false, false, false);
        oracle.perl_binary = perl;

        unsafe { std::env::set_var("PERL5OPT", "-Mstrict") };
        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print $ENV{PERL5OPT} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;
        unsafe { std::env::remove_var("PERL5OPT") };

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            "UNSET",
            "PERL5OPT must be stripped when allow_perl5opt=false; got: {stdout:?}",
        );
        Ok(())
    }

    /// HOME is stripped (not in allow-set).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn perl_oracle_env_strips_home() -> TestResult {
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

        let mut oracle = dummy_env(false, false, false);
        oracle.perl_binary = perl;

        unsafe { std::env::set_var("HOME", &poison_path) };
        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print $ENV{HOME} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;
        unsafe { std::env::remove_var("HOME") };

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains(&poison_path),
            "HOME poison must NOT appear in subprocess output; got: {stdout:?}",
        );
        Ok(())
    }

    /// PERL_LOCAL_LIB_ROOT is stripped by default.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn perl_oracle_env_strips_local_lib() -> TestResult {
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

        let mut oracle = dummy_env(false, false, false);
        oracle.perl_binary = perl;

        unsafe { std::env::set_var("PERL_LOCAL_LIB_ROOT", &poison_path) };
        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print $ENV{PERL_LOCAL_LIB_ROOT} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;
        unsafe { std::env::remove_var("PERL_LOCAL_LIB_ROOT") };

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            "UNSET",
            "PERL_LOCAL_LIB_ROOT must be stripped when allow_local_lib=false; got: {stdout:?}",
        );
        Ok(())
    }
}
