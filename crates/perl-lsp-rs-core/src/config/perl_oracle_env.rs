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
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use super::SYSTEM_INC_PROBE_TIMEOUT;
use super::WorkspaceConfig;

#[cfg(all(not(target_arch = "wasm32"), windows))]
const PERLDOC_EXECUTABLE_CANDIDATES: &[&str] =
    &["perldoc.bat", "perldoc.cmd", "perldoc.exe", "perldoc"];

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
const PERLDOC_EXECUTABLE_CANDIDATES: &[&str] = &["perldoc"];

#[cfg(not(target_arch = "wasm32"))]
fn perldoc_binary_near_perl(perl_binary: &Path) -> Option<PathBuf> {
    let dir = perl_binary.parent()?;
    if dir.as_os_str().is_empty() {
        return None;
    }

    for candidate_name in PERLDOC_EXECUTABLE_CANDIDATES {
        let candidate = dir.join(candidate_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(not(target_arch = "wasm32"))]
fn default_perldoc_binary() -> PathBuf {
    PathBuf::from("perldoc")
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_perldoc_binary(config: &WorkspaceConfig) -> PathBuf {
    if let Some(configured_perl) = config.perl_path.as_deref().filter(|path| !path.is_empty())
        && let Some(perldoc) = perldoc_binary_near_perl(Path::new(configured_perl))
    {
        return perldoc;
    }

    if let Ok(perl_binary) = crate::platform::resolve_perl_path_with_toolchain()
        && let Some(perldoc) = perldoc_binary_near_perl(&perl_binary)
    {
        return perldoc;
    }

    default_perldoc_binary()
}

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
    /// Apply this oracle's environment contract to an existing [`Command`].
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
    ///
    /// Prefer [`into_command`] for ordinary `std::process::Command` callers.
    /// This lower-level helper exists for call sites that wrap a standard
    /// command builder, such as `tokio::process::Command::as_std_mut`.
    ///
    /// [`into_command`]: PerlOracleEnv::into_command
    pub fn configure_command(&self, cmd: &mut Command) {
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
    }

    /// Build a [`Command`] with ALL non-allowlisted env vars stripped.
    pub fn into_command(&self) -> Command {
        let mut cmd = Command::new(&self.perl_binary);
        self.configure_command(&mut cmd);

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
            timeout: SYSTEM_INC_PROBE_TIMEOUT,
            allow_perl5lib: config.use_perl5lib,
            allow_perl5opt: false,
            allow_local_lib: false,
            extra_env: BTreeMap::new(),
        })
    }

    /// Constructor for module-resolution system `@INC` probes.
    ///
    /// Module resolution should only ask the interpreter for startup `@INC`
    /// when the user has opted into system includes. When enabled, its
    /// subprocess contract matches the startup probe:
    ///
    /// - `allow_perl5lib`: mirrors `config.use_perl5lib`.
    /// - `allow_perl5opt`: always `false` so ambient runtime flags cannot
    ///   alter module lookup.
    /// - `allow_local_lib`: always `false`; explicit config owns lookup.
    /// - `timeout`: `SYSTEM_INC_PROBE_TIMEOUT`.
    /// - `cwd` and `perl_binary`: resolved by the startup `@INC` probe helper.
    ///
    /// Returns `None` when `use_system_inc` is disabled or the Perl binary
    /// cannot be resolved.
    pub fn for_module_resolution(config: &WorkspaceConfig) -> Option<Self> {
        if !config.use_system_inc {
            return None;
        }

        Self::for_startup_inc_probe(config)
    }

    /// Constructor for language-level Perl invocations that run on behalf of the user
    /// but must not be affected by ambient runtime injection.
    ///
    /// Use this for LSP-triggered actions such as `perl.debugFile` that launch Perl
    /// directly but are not system probes and not free-form user scripts.
    ///
    /// Semantics:
    ///
    /// - `allow_perl5lib`: mirrors `config.use_perl5lib` — the user's explicit opt-in
    ///   to include `PERL5LIB` in the child environment.  This lets a user's libraries
    ///   be found when they have configured the LSP to honour `PERL5LIB`, but prevents
    ///   ambient contamination when they have not.
    /// - `allow_perl5opt`: always `false` — `PERL5OPT` injects module flags into every
    ///   Perl invocation and can cause language probes to misbehave or load untested
    ///   code.  The user's Perl file must be runnable without ambient `-M` injections.
    /// - `allow_local_lib`: always `false` — use the explicit `perl_path` config rather
    ///   than ambient `local::lib` activation to locate the interpreter.
    /// - `timeout`: 30 seconds (generous ceiling for interactive/user-facing actions).
    /// - `cwd`: caller-supplied (typically the parent directory of the file being
    ///   processed).
    /// - `extra_env`: empty.
    ///
    /// Returns `None` if the Perl binary cannot be resolved.  The caller should
    /// surface an actionable error to the user instead of falling back to ambient
    /// `perl` lookup.
    pub fn for_language_probe(config: &WorkspaceConfig, cwd: PathBuf) -> Option<Self> {
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
    /// Returns `None` if the Perl binary cannot be resolved. Editor-runtime
    /// callers should surface an actionable error instead of falling back to
    /// ambient `perl` lookup.
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

    /// Constructor for `perldoc` hover documentation lookups.
    ///
    /// `perldoc` is not the editor's semantic source of truth; it is a bridge for
    /// user-requested documentation. It still runs inside the editor session, so
    /// ambient Perl state must be explicit:
    ///
    /// - `allow_perl5lib`: mirrors `config.use_perl5lib` so project-local docs
    ///   are visible only when the user opted into `PERL5LIB`.
    /// - `allow_perl5opt`: always `false` because `PERL5OPT` injects runtime
    ///   flags into every Perl-family subprocess.
    /// - `allow_local_lib`: always `false`; the perldoc binary is resolved from
    ///   the configured Perl toolchain when possible.
    /// - `LC_ALL`: forced to `C` for deterministic plain-text output.
    /// - `timeout`: 10 seconds, matching the existing hover budget.
    pub fn for_perldoc(config: &WorkspaceConfig, cwd: PathBuf) -> Self {
        let mut extra_env = BTreeMap::new();
        extra_env.insert("LC_ALL".to_string(), "C".to_string());

        Self {
            perl_binary: resolve_perldoc_binary(config),
            cwd,
            timeout: Duration::from_secs(10),
            allow_perl5lib: config.use_perl5lib,
            allow_perl5opt: false,
            allow_local_lib: false,
            extra_env,
        }
    }

    /// Constructor for the Perl::LanguageServer DAP bridge process.
    ///
    /// The DAP bridge launches a long-running Perl::LanguageServer process on
    /// behalf of a debug session. Unlike LSP analysis probes, debug sessions may
    /// legitimately opt into ambient Perl runtime variables, so this constructor
    /// accepts the debug configuration's passthrough decisions explicitly.
    ///
    /// Env contract:
    ///
    /// - `allow_perl5lib`: caller-supplied debug passthrough choice.
    /// - `allow_perl5opt`: caller-supplied debug passthrough choice.
    /// - `allow_local_lib`: always `false`; bridge passthrough is limited to
    ///   the two debug-approved Perl variables until a separate config contract
    ///   declares more.
    /// - `PATH`: preserved by [`into_command`] / [`configure_command`] so the
    ///   bridge process and debuggee can resolve helper commands.
    /// - `timeout`: 30 seconds as a startup budget marker; the bridge process
    ///   itself is managed by the adapter lifecycle after spawn.
    /// - `cwd`: caller-supplied and explicit.
    /// - `extra_env`: empty.
    ///
    /// [`configure_command`]: PerlOracleEnv::configure_command
    /// [`into_command`]: PerlOracleEnv::into_command
    pub fn for_dap_bridge(
        perl_binary: PathBuf,
        cwd: PathBuf,
        allow_perl5lib: bool,
        allow_perl5opt: bool,
    ) -> Self {
        Self {
            perl_binary,
            cwd,
            timeout: Duration::from_secs(30),
            allow_perl5lib,
            allow_perl5opt,
            allow_local_lib: false,
            extra_env: BTreeMap::new(),
        }
    }

    /// Constructor for Perl version probes.
    ///
    /// Used when an already-resolved Perl binary needs to be interrogated for its
    /// version (e.g. `perl -e 'print $]'`). The binary path is caller-supplied so
    /// no config lookup is needed, and the constructor is infallible.
    ///
    /// Env contract (deny-all-ambient policy):
    ///
    /// - `allow_perl5lib`: always `false` — version probes must be deterministic;
    ///   user `PERL5LIB` state must not affect the reported version.
    /// - `allow_perl5opt`: always `false` — `PERL5OPT` injects runtime flags that
    ///   could alter the probe output unpredictably.
    /// - `allow_local_lib`: always `false` — `local::lib` activation is not part
    ///   of the version probe contract.
    /// - `timeout`: 5 seconds (generous for a simple `print $]`).
    /// - `cwd`: caller-supplied.
    /// - `extra_env`: empty; callers may extend via `oracle.extra_env.insert(...)`.
    pub fn for_version_probe(perl_binary: PathBuf, cwd: PathBuf) -> Self {
        Self {
            perl_binary,
            cwd,
            timeout: Duration::from_secs(5),
            allow_perl5lib: false,
            allow_perl5opt: false,
            allow_local_lib: false,
            extra_env: BTreeMap::new(),
        }
    }

    /// Constructor for DAP test fixture Perl probes.
    ///
    /// Checks whether `perl` is available on `PATH` and, if so, returns a
    /// `PerlOracleEnv` with the DAP test fixture env contract:
    ///
    /// - `allow_perl5lib`: `false` — DAP tests must be hermetic; ambient
    ///   `PERL5LIB` must not change which tests pass or skip.
    /// - `allow_perl5opt`: `false` — runtime injection flags must not affect
    ///   test results.
    /// - `allow_local_lib`: `false` — `local::lib` activation is not part of
    ///   the test fixture contract.
    /// - `timeout`: 5 seconds (sufficient for unit/integration test probes).
    /// - `cwd`: current working directory of the test process.
    /// - `extra_env`: empty.
    ///
    /// Returns `None` when `perl` is not on `PATH`, enabling skip-when-missing
    /// semantics: `if oracle.is_none() { return; }`.
    ///
    /// The existence check itself preserves PATH resolution but strips Perl
    /// ambient variables that could change skip/pass behavior; the
    /// deny-all-ambient policy applies to every subsequent invocation via
    /// [`into_command`].
    ///
    /// [`into_command`]: PerlOracleEnv::into_command
    pub fn for_dap_test_fixture() -> Option<Self> {
        let available = Command::new("perl")
            .arg("--version")
            .env_remove("PERL5LIB")
            .env_remove("PERL5OPT")
            .env_remove("PERL_LOCAL_LIB_ROOT")
            .env_remove("PERL_LOCAL_LIB_PREFIX")
            .output()
            .is_ok();
        if !available {
            return None;
        }

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Some(Self {
            perl_binary: PathBuf::from("perl"),
            cwd,
            timeout: Duration::from_secs(5),
            allow_perl5lib: false,
            allow_perl5opt: false,
            allow_local_lib: false,
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
    pub fn for_module_resolution(_config: &WorkspaceConfig) -> Option<Self> {
        None
    }

    /// Returns `None` on WASM (no subprocess support).
    pub fn for_language_probe(_config: &WorkspaceConfig, _cwd: std::path::PathBuf) -> Option<Self> {
        None
    }

    /// Returns `None` on WASM (no subprocess support).
    pub fn for_execute_command(
        _config: &WorkspaceConfig,
        _cwd: std::path::PathBuf,
    ) -> Option<Self> {
        None
    }

    /// Returns a no-op stub on WASM (no subprocess support).
    pub fn for_perldoc(_config: &WorkspaceConfig, _cwd: std::path::PathBuf) -> Self {
        PerlOracleEnv
    }

    /// Returns a no-op stub on WASM (no subprocess support).
    pub fn for_version_probe(_perl_binary: std::path::PathBuf, _cwd: std::path::PathBuf) -> Self {
        PerlOracleEnv
    }

    /// Returns a no-op stub on WASM (no subprocess support).
    pub fn for_dap_bridge(
        _perl_binary: std::path::PathBuf,
        _cwd: std::path::PathBuf,
        _allow_perl5lib: bool,
        _allow_perl5opt: bool,
    ) -> Self {
        PerlOracleEnv
    }

    /// Returns `None` on WASM (no subprocess support).
    pub fn for_dap_test_fixture() -> Option<Self> {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(unsafe_code)] // required for std::env::set_var/remove_var in Rust 2024 (unsafe fn)
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex, MutexGuard};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    static ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn env_lock() -> TestResult<MutexGuard<'static, ()>> {
        ENV_MUTEX
            .lock()
            .map_err(|_| std::io::Error::other("perl oracle env test mutex poisoned").into())
    }

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

    // ── for_perldoc tests ─────────────────────────────────────────────────────

    /// `for_perldoc` mirrors `config.use_perl5lib` while denying `PERL5OPT`
    /// and `local::lib`, and pins locale for stable plain-text docs.
    #[test]
    fn for_perldoc_respects_config_flags() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let mut config = WorkspaceConfig::default();
        config.use_perl5lib = true;
        let env = PerlOracleEnv::for_perldoc(&config, cwd.clone());
        assert!(env.allow_perl5lib, "perldoc should honor explicit use_perl5lib=true");
        assert!(!env.allow_perl5opt, "perldoc must strip PERL5OPT");
        assert!(!env.allow_local_lib, "perldoc must strip local::lib activation vars");
        assert_eq!(env.extra_env.get("LC_ALL").map(String::as_str), Some("C"));

        config.use_perl5lib = false;
        let env = PerlOracleEnv::for_perldoc(&config, cwd);
        assert!(!env.allow_perl5lib, "perldoc should honor explicit use_perl5lib=false");
        assert!(!env.allow_perl5opt, "perldoc must strip PERL5OPT");
        assert!(!env.allow_local_lib, "perldoc must strip local::lib activation vars");
        assert_eq!(env.extra_env.get("LC_ALL").map(String::as_str), Some("C"));
    }

    #[test]
    fn for_perldoc_prefers_configured_perl_sibling() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_name = if cfg!(windows) { "perl.exe" } else { "perl" };
        let perldoc_name =
            PERLDOC_EXECUTABLE_CANDIDATES.first().ok_or("missing perldoc executable candidate")?;
        let perl_path = temp.path().join(perl_name);
        let perldoc_path = temp.path().join(perldoc_name);
        std::fs::write(&perl_path, "")?;
        std::fs::write(&perldoc_path, "")?;

        let config = WorkspaceConfig {
            perl_path: Some(perl_path.to_string_lossy().into_owned()),
            ..WorkspaceConfig::default()
        };
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let env = PerlOracleEnv::for_perldoc(&config, cwd);

        assert_eq!(
            env.perl_binary, perldoc_path,
            "perldoc should resolve from the configured Perl toolchain directory"
        );
        Ok(())
    }

    #[test]
    fn for_perldoc_does_not_use_missing_configured_sibling() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_name = if cfg!(windows) { "perl.exe" } else { "perl" };
        let perl_path = temp.path().join(perl_name);
        let missing_perldoc_path = temp.path().join(
            PERLDOC_EXECUTABLE_CANDIDATES.first().ok_or("missing perldoc executable candidate")?,
        );
        std::fs::write(&perl_path, "")?;

        let config = WorkspaceConfig {
            perl_path: Some(perl_path.to_string_lossy().into_owned()),
            ..WorkspaceConfig::default()
        };
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let env = PerlOracleEnv::for_perldoc(&config, cwd);

        assert_ne!(
            env.perl_binary, missing_perldoc_path,
            "missing configured-toolchain perldoc must not be used as an explicit binary"
        );
        Ok(())
    }

    /// The perldoc oracle strips poisoned ambient env by default, while still
    /// allowing `PERL5LIB` only when the user explicitly enabled it.
    #[test]
    fn for_perldoc_strips_poisoned_env_and_gates_perl5lib() -> TestResult {
        let _env_guard = env_lock()?;
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };
        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        unsafe { std::env::set_var("PERL5LIB", &poison_path) };
        unsafe { std::env::set_var("PERL5OPT", "-Mstrict") };
        unsafe { std::env::set_var("PERL_LOCAL_LIB_ROOT", &poison_path) };
        unsafe { std::env::set_var("PERL_LOCAL_LIB_PREFIX", &poison_path) };

        let result = (|| -> TestResult<(String, String)> {
            let denied_config = WorkspaceConfig {
                perl_path: Some(perl.to_string_lossy().into_owned()),
                use_perl5lib: false,
                ..WorkspaceConfig::default()
            };
            let denied = PerlOracleEnv::for_perldoc(&denied_config, cwd.clone());
            let mut cmd = std::process::Command::new(&perl);
            denied.configure_command(&mut cmd);
            cmd.args([
                "-e",
                "print join('|', $ENV{PERL5LIB}//'UNSET', $ENV{PERL5OPT}//'UNSET', \
                 $ENV{PERL_LOCAL_LIB_ROOT}//'UNSET', $ENV{PERL_LOCAL_LIB_PREFIX}//'UNSET', \
                 $ENV{LC_ALL}//'UNSET')",
            ]);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let denied_out = cmd.output()?;

            let allowed_config = WorkspaceConfig {
                perl_path: Some(perl.to_string_lossy().into_owned()),
                use_perl5lib: true,
                ..WorkspaceConfig::default()
            };
            let allowed = PerlOracleEnv::for_perldoc(&allowed_config, cwd);
            let mut cmd = std::process::Command::new(&perl);
            allowed.configure_command(&mut cmd);
            cmd.args([
                "-e",
                "print join('|', $ENV{PERL5LIB}//'UNSET', $ENV{PERL5OPT}//'UNSET', \
                 $ENV{PERL_LOCAL_LIB_ROOT}//'UNSET', $ENV{PERL_LOCAL_LIB_PREFIX}//'UNSET', \
                 $ENV{LC_ALL}//'UNSET')",
            ]);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let allowed_out = cmd.output()?;

            Ok((
                String::from_utf8_lossy(&denied_out.stdout).into_owned(),
                String::from_utf8_lossy(&allowed_out.stdout).into_owned(),
            ))
        })();

        unsafe { std::env::remove_var("PERL5LIB") };
        unsafe { std::env::remove_var("PERL5OPT") };
        unsafe { std::env::remove_var("PERL_LOCAL_LIB_ROOT") };
        unsafe { std::env::remove_var("PERL_LOCAL_LIB_PREFIX") };

        let (denied_stdout, allowed_stdout) = result?;
        assert_eq!(
            denied_stdout.trim(),
            "UNSET|UNSET|UNSET|UNSET|C",
            "perldoc must strip poisoned env when PERL5LIB is disabled; got: {denied_stdout:?}",
        );
        assert!(
            allowed_stdout.starts_with(&poison_path),
            "perldoc should pass PERL5LIB through only when opted in; got: {allowed_stdout:?}",
        );
        assert!(
            allowed_stdout.ends_with("|UNSET|UNSET|UNSET|C"),
            "perldoc must still strip PERL5OPT/local::lib and set LC_ALL; got: {allowed_stdout:?}",
        );

        Ok(())
    }

    // ── for_language_probe tests ──────────────────────────────────────────────

    /// `for_language_probe` mirrors `config.use_perl5lib` → `allow_perl5lib`
    /// and always denies `PERL5OPT` and `local::lib`.
    #[test]
    fn for_language_probe_respects_config_flags() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let mut config = WorkspaceConfig::default();
        config.use_perl5lib = true;
        let env = PerlOracleEnv::for_language_probe(&config, cwd.clone());
        if let Some(e) = env {
            assert!(e.allow_perl5lib, "allow_perl5lib must mirror config.use_perl5lib=true");
            assert!(!e.allow_perl5opt, "allow_perl5opt must always be false for language probe");
            assert!(!e.allow_local_lib, "allow_local_lib must always be false for language probe");
        }

        config.use_perl5lib = false;
        let env = PerlOracleEnv::for_language_probe(&config, cwd);
        if let Some(e) = env {
            assert!(!e.allow_perl5lib, "allow_perl5lib must mirror config.use_perl5lib=false");
            assert!(!e.allow_perl5opt, "allow_perl5opt must always be false for language probe");
            assert!(!e.allow_local_lib, "allow_local_lib must always be false for language probe");
        }
    }

    /// `for_language_probe` strips `PERL5OPT` even when set in the parent env —
    /// poisoned-env regression guard for the #8685 seam.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn for_language_probe_strips_perl5opt() -> TestResult {
        let _env_guard = env_lock()?;
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut config = WorkspaceConfig::default();
        config.perl_path = Some(perl.to_string_lossy().into_owned());

        let oracle = PerlOracleEnv::for_language_probe(&config, cwd)
            .ok_or("for_language_probe returned None unexpectedly")?;

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
            "PERL5OPT must be stripped by for_language_probe; got: {stdout:?}",
        );
        Ok(())
    }

    /// `for_language_probe` strips `PERL5LIB` when `use_perl5lib=false` —
    /// poisoned-env regression guard for the #8685 seam.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn for_language_probe_strips_perl5lib_when_disabled() -> TestResult {
        let _env_guard = env_lock()?;
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut config = WorkspaceConfig::default();
        config.use_perl5lib = false;
        config.perl_path = Some(perl.to_string_lossy().into_owned());

        let oracle = PerlOracleEnv::for_language_probe(&config, cwd)
            .ok_or("for_language_probe returned None unexpectedly")?;

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

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
            "PERL5LIB must be stripped when use_perl5lib=false; got: {stdout:?}",
        );
        Ok(())
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
            assert!(!e.allow_perl5opt, "allow_perl5opt must always be false for startup probe");
            assert!(!e.allow_local_lib, "allow_local_lib must always be false for startup probe");
        }
    }

    /// `for_module_resolution` maps system-include and PERL5LIB config to the
    /// module-resolution oracle contract.
    #[test]
    fn for_module_resolution_respects_config_flags() -> TestResult {
        let disabled_config = WorkspaceConfig {
            perl_path: Some("perl".to_string()),
            use_system_inc: false,
            use_perl5lib: true,
            ..WorkspaceConfig::default()
        };
        assert!(
            PerlOracleEnv::for_module_resolution(&disabled_config).is_none(),
            "module resolution oracle must be disabled when use_system_inc=false"
        );

        let enabled_config = WorkspaceConfig { use_system_inc: true, ..disabled_config };
        let env = PerlOracleEnv::for_module_resolution(&enabled_config)
            .ok_or("for_module_resolution returned None unexpectedly")?;
        assert!(env.allow_perl5lib, "allow_perl5lib should mirror use_perl5lib=true");
        assert!(!env.allow_perl5opt, "module resolution must strip PERL5OPT");
        assert!(!env.allow_local_lib, "module resolution must strip local::lib env vars");

        let no_perl5lib_config = WorkspaceConfig { use_perl5lib: false, ..enabled_config };
        let env = PerlOracleEnv::for_module_resolution(&no_perl5lib_config)
            .ok_or("for_module_resolution returned None unexpectedly")?;
        assert!(!env.allow_perl5lib, "allow_perl5lib should mirror use_perl5lib=false");

        Ok(())
    }

    /// `for_module_resolution` strips PERL5OPT and respects PERL5LIB opt-in.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn for_module_resolution_strips_perl5opt_and_gates_perl5lib() -> TestResult {
        let _env_guard = env_lock()?;
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let config = WorkspaceConfig {
            perl_path: Some(perl.to_string_lossy().to_string()),
            use_system_inc: true,
            use_perl5lib: true,
            ..WorkspaceConfig::default()
        };

        unsafe { std::env::set_var("PERL5LIB", &poison_path) };
        unsafe { std::env::set_var("PERL5OPT", "-Mstrict") };
        let mut oracle = PerlOracleEnv::for_module_resolution(&config)
            .ok_or("for_module_resolution returned None unexpectedly")?;
        oracle.cwd = cwd.clone();
        let mut cmd = oracle.into_command();
        cmd.args([
            "-e",
            "print (($ENV{PERL5LIB} // 'UNSET') . \"\\n\" . ($ENV{PERL5OPT} // 'UNSET'))",
        ]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.lines().next().is_some_and(|line| line.contains(&poison_path)),
            "PERL5LIB must pass through when use_perl5lib=true; got: {stdout:?}"
        );
        assert!(
            stdout.lines().nth(1).is_some_and(|line| line == "UNSET"),
            "PERL5OPT must be stripped for module resolution; got: {stdout:?}"
        );

        let no_perl5lib_config = WorkspaceConfig { use_perl5lib: false, ..config };
        let mut oracle = PerlOracleEnv::for_module_resolution(&no_perl5lib_config)
            .ok_or("for_module_resolution returned None unexpectedly")?;
        oracle.cwd = cwd;
        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print $ENV{PERL5LIB} // 'UNSET'"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;

        unsafe { std::env::remove_var("PERL5LIB") };
        unsafe { std::env::remove_var("PERL5OPT") };

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.trim(),
            "UNSET",
            "PERL5LIB must be stripped when use_perl5lib=false; got: {stdout:?}",
        );

        Ok(())
    }

    /// `for_dap_bridge` mirrors the debug configuration passthrough flags.
    #[test]
    fn for_dap_bridge_respects_passthrough_flags() {
        let perl_binary = PathBuf::from("perl");
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let denied = PerlOracleEnv::for_dap_bridge(perl_binary.clone(), cwd.clone(), false, false);
        assert!(!denied.allow_perl5lib, "DAP bridge must deny PERL5LIB when disabled");
        assert!(!denied.allow_perl5opt, "DAP bridge must deny PERL5OPT when disabled");
        assert!(!denied.allow_local_lib, "DAP bridge must not allow local::lib by default");
        assert!(denied.extra_env.is_empty(), "DAP bridge extra_env starts empty");

        let allowed = PerlOracleEnv::for_dap_bridge(perl_binary, cwd, true, true);
        assert!(allowed.allow_perl5lib, "DAP bridge must allow PERL5LIB when opted in");
        assert!(allowed.allow_perl5opt, "DAP bridge must allow PERL5OPT when opted in");
        assert!(!allowed.allow_local_lib, "DAP bridge local::lib remains denied");
    }

    /// `for_dap_bridge` blocks poisoned ambient Perl env unless debug config opts in.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn for_dap_bridge_gates_perl5lib_and_perl5opt() -> TestResult {
        let _env_guard = env_lock()?;
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        unsafe { std::env::set_var("PERL5LIB", &poison_path) };
        unsafe { std::env::set_var("PERL5OPT", "-Mstrict") };

        let result = (|| -> TestResult<(String, String)> {
            let denied = PerlOracleEnv::for_dap_bridge(perl.clone(), cwd.clone(), false, false);
            let mut cmd = denied.into_command();
            cmd.args(["-e", "print join('|', $ENV{PERL5LIB}//'UNSET', $ENV{PERL5OPT}//'UNSET')"]);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let denied_out = cmd.output()?;

            let allowed = PerlOracleEnv::for_dap_bridge(perl, cwd, true, true);
            let mut cmd = allowed.into_command();
            cmd.args(["-e", "print join('|', $ENV{PERL5LIB}//'UNSET', $ENV{PERL5OPT}//'UNSET')"]);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let allowed_out = cmd.output()?;

            Ok((
                String::from_utf8_lossy(&denied_out.stdout).into_owned(),
                String::from_utf8_lossy(&allowed_out.stdout).into_owned(),
            ))
        })();

        unsafe { std::env::remove_var("PERL5LIB") };
        unsafe { std::env::remove_var("PERL5OPT") };

        let (denied_stdout, allowed_stdout) = result?;
        assert_eq!(
            denied_stdout.trim(),
            "UNSET|UNSET",
            "DAP bridge must strip poisoned env when passthrough is disabled; got: {denied_stdout:?}",
        );
        assert!(
            allowed_stdout.contains(&poison_path),
            "DAP bridge must pass PERL5LIB through when opted in; got: {allowed_stdout:?}",
        );
        assert!(
            allowed_stdout.contains("-Mstrict"),
            "DAP bridge must pass PERL5OPT through when opted in; got: {allowed_stdout:?}",
        );

        Ok(())
    }

    /// `for_dap_test_fixture` uses the deny-all-ambient fixture contract.
    #[test]
    fn for_dap_test_fixture_denies_ambient_perl_env() -> TestResult {
        let _env_guard = env_lock()?;
        let Some(oracle) = PerlOracleEnv::for_dap_test_fixture() else {
            return Ok(());
        };

        assert_eq!(oracle.perl_binary, PathBuf::from("perl"));
        assert!(!oracle.allow_perl5lib, "DAP test fixtures must strip PERL5LIB");
        assert!(!oracle.allow_perl5opt, "DAP test fixtures must strip PERL5OPT");
        assert!(!oracle.allow_local_lib, "DAP test fixtures must strip local::lib env");
        assert!(oracle.extra_env.is_empty(), "DAP test fixture extra_env starts empty");
        Ok(())
    }

    /// `for_dap_test_fixture` strips parent-process Perl env from actual Perl
    /// fixture invocations.
    #[test]
    fn for_dap_test_fixture_strips_poisoned_env_from_invocation() -> TestResult {
        let _env_guard = env_lock()?;
        let Some(oracle) = PerlOracleEnv::for_dap_test_fixture() else {
            return Ok(());
        };

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

        unsafe { std::env::set_var("PERL5LIB", &poison_path) };
        unsafe { std::env::set_var("PERL5OPT", "-Mstrict") };
        unsafe { std::env::set_var("PERL_LOCAL_LIB_ROOT", &poison_path) };

        let result = (|| -> TestResult<String> {
            let mut cmd = oracle.into_command();
            cmd.args([
                "-e",
                "print join('|', $ENV{PERL5LIB}//'UNSET', $ENV{PERL5OPT}//'UNSET', $ENV{PERL_LOCAL_LIB_ROOT}//'UNSET')",
            ]);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let out = cmd.output()?;
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        })();

        unsafe { std::env::remove_var("PERL5LIB") };
        unsafe { std::env::remove_var("PERL5OPT") };
        unsafe { std::env::remove_var("PERL_LOCAL_LIB_ROOT") };

        let stdout = result?;
        assert_eq!(
            stdout.trim(),
            "UNSET|UNSET|UNSET",
            "DAP test fixture Perl invocations must strip ambient Perl env; got: {stdout:?}",
        );
        Ok(())
    }

    /// `for_startup_inc_probe` with `usePerl5lib=false` must strip PERL5LIB:
    /// regression guard for the #8493 incident.
    #[test]
    fn for_startup_inc_probe_strips_when_use_perl5lib_false() -> TestResult {
        let _env_guard = env_lock()?;
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
        let _env_guard = env_lock()?;
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
        let _env_guard = env_lock()?;
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
        let _env_guard = env_lock()?;
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
        let _env_guard = env_lock()?;
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
        let _env_guard = env_lock()?;
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

    // ── for_version_probe tests ───────────────────────────────────────────────

    /// `for_version_probe` always denies PERL5LIB, PERL5OPT, and local::lib.
    #[test]
    fn for_version_probe_denies_all_ambient_vars() {
        let perl_binary = PathBuf::from("perl");
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let oracle = PerlOracleEnv::for_version_probe(perl_binary, cwd);

        assert!(!oracle.allow_perl5lib, "allow_perl5lib must be false for version probe");
        assert!(!oracle.allow_perl5opt, "allow_perl5opt must be false for version probe");
        assert!(!oracle.allow_local_lib, "allow_local_lib must be false for version probe");
        assert!(oracle.extra_env.is_empty(), "extra_env must be empty by default");
    }

    /// `for_version_probe` uses the caller-supplied binary and cwd verbatim.
    #[test]
    fn for_version_probe_uses_caller_supplied_binary_and_cwd() {
        let binary = PathBuf::from("/usr/bin/perl");
        let cwd = PathBuf::from("/tmp/test-cwd");
        let oracle = PerlOracleEnv::for_version_probe(binary.clone(), cwd.clone());
        assert_eq!(oracle.perl_binary, binary);
        assert_eq!(oracle.cwd, cwd);
    }

    /// `for_version_probe` strips PERL5LIB and PERL5OPT from the subprocess
    /// (canonical acceptance test for the #8688 incident).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn for_version_probe_strips_poisoned_env() -> TestResult {
        let _env_guard = env_lock()?;
        let perl = match perl_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let oracle = PerlOracleEnv::for_version_probe(perl, cwd);

        let poison_dir = tempfile::tempdir()?;
        let poison_path = poison_dir.path().to_string_lossy().into_owned();

        unsafe { std::env::set_var("PERL5LIB", &poison_path) };
        unsafe { std::env::set_var("PERL5OPT", "-Mevil") };

        let mut cmd = oracle.into_command();
        cmd.args(["-e", "print join('|', $ENV{PERL5LIB}//'UNSET', $ENV{PERL5OPT}//'UNSET')"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let out = cmd.output()?;

        unsafe { std::env::remove_var("PERL5LIB") };
        unsafe { std::env::remove_var("PERL5OPT") };

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains(&poison_path),
            "PERL5LIB poison must NOT appear in version probe subprocess; got: {stdout:?}",
        );
        assert!(
            !stdout.contains("-Mevil"),
            "PERL5OPT poison must NOT appear in version probe subprocess; got: {stdout:?}",
        );
        assert_eq!(
            stdout.trim(),
            "UNSET|UNSET",
            "version probe subprocess must see both PERL5LIB and PERL5OPT as unset; got: {stdout:?}",
        );
        Ok(())
    }
}
