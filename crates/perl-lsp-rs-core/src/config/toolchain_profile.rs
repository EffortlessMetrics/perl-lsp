//! Unified Perl executable (toolchain) profile.
//!
//! [`PerlToolchainProfile`] is the single source of truth for the resolved Perl
//! interpreter identity. It centralises the interpreter resolution logic
//! (explicit `perl_path` config → toolchain managers → `PATH`) that was
//! previously duplicated verbatim across the [`PerlOracleEnv`] `for_*`
//! constructors, so that every Perl-spawning seam agrees on *which* binary it is
//! invoking.
//!
//! ## Resolution contract
//!
//! [`PerlToolchainProfile::resolve`] reproduces the historical precedence used
//! by the oracle seams:
//!
//! 1. An explicit, non-empty `config.perl_path` is taken **verbatim** (the
//!    caller's choice always wins, and is never validated for existence here —
//!    callers surface their own actionable errors downstream).
//! 2. Otherwise the toolchain resolver
//!    ([`resolve_perl_path_with_toolchain`]) is consulted: perlbrew → plenv →
//!    `PATH`.
//! 3. If neither yields a binary, resolution returns `None`, exactly mirroring
//!    the previous `for_*` behaviour of bailing out (`return None`) when the
//!    interpreter cannot be located.
//!
//! [`PerlOracleEnv`]: super::PerlOracleEnv
//! [`resolve_perl_path_with_toolchain`]: crate::platform::resolve_perl_path_with_toolchain

// On WASM, `resolve` always returns `None` and a `PerlToolchainProfile` is
// therefore never constructed, leaving `perl_binary` provably unread. Silence
// the resulting dead-code lint rather than littering each field with `cfg`.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{LazyLock, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::SystemTime;

use super::WorkspaceConfig;

/// Identity fingerprint of a Perl binary file, used to invalidate the version
/// cache when the binary is replaced (e.g. a `perlbrew switch`) without the path
/// changing.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PerlBinaryFingerprint {
    len: u64,
    modified: Option<SystemTime>,
}

/// Process-wide cache of probed interpreter versions, keyed by binary path and
/// guarded by the binary's fingerprint. Shared by every `PerlToolchainProfile`
/// (LSP analysis seams and the DAP launch path) so the `perl -e 'print $]'`
/// probe runs at most once per distinct interpreter build.
#[cfg(not(target_arch = "wasm32"))]
static PERL_VERSION_CACHE: LazyLock<
    Mutex<HashMap<PathBuf, (PerlBinaryFingerprint, Option<String>)>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(not(target_arch = "wasm32"))]
fn perl_binary_fingerprint(perl_path: &Path) -> Option<PerlBinaryFingerprint> {
    let metadata = std::fs::metadata(perl_path).ok()?;
    let modified = metadata.modified().ok();
    Some(PerlBinaryFingerprint { len: metadata.len(), modified })
}

/// Resolved identity of the Perl interpreter for a workspace.
///
/// Construct with [`PerlToolchainProfile::resolve`]. The profile is the
/// canonical answer to "which `perl` binary should this workspace invoke?" and
/// is intended to be the shared seam that the [`PerlOracleEnv`] constructors —
/// and, in later increments, the DAP launch path — resolve through.
///
/// [`PerlOracleEnv`]: super::PerlOracleEnv
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerlToolchainProfile {
    /// Absolute or caller-supplied path to the Perl binary.
    ///
    /// When an explicit `perl_path` was configured it is stored verbatim;
    /// otherwise this is the path produced by the toolchain resolver. Prefer an
    /// absolute path to avoid `PATH`-based resolution ambiguity (perlbrew shims,
    /// plenv, etc.).
    perl_binary: PathBuf,
}

impl PerlToolchainProfile {
    /// Resolve the toolchain profile for a workspace configuration.
    ///
    /// Follows the precedence documented at the [module level](self): explicit
    /// non-empty `config.perl_path` wins, otherwise the toolchain resolver is
    /// consulted. Returns `None` when no interpreter can be located.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resolve(config: &WorkspaceConfig) -> Option<Self> {
        use crate::platform::resolve_perl_path_with_toolchain;

        let perl_binary = match config.perl_path.as_deref().filter(|p| !p.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => resolve_perl_path_with_toolchain().ok()?,
        };

        Some(Self { perl_binary })
    }

    /// Resolve the toolchain profile for a workspace configuration.
    ///
    /// On WASM targets there is no subprocess support, so resolution always
    /// returns `None`, mirroring the [`PerlOracleEnv`] WASM stubs.
    ///
    /// [`PerlOracleEnv`]: super::PerlOracleEnv
    #[cfg(target_arch = "wasm32")]
    pub fn resolve(_config: &WorkspaceConfig) -> Option<Self> {
        None
    }

    /// Construct a profile from an already-resolved Perl binary path.
    ///
    /// Useful for callers (e.g. the DAP launch path) that have located the
    /// interpreter through their own resolution and want the profile's
    /// companion behaviour — such as cached [`version`](Self::version) probing —
    /// without re-running [`resolve`](Self::resolve).
    pub fn from_binary(perl_binary: PathBuf) -> Self {
        Self { perl_binary }
    }

    /// Borrow the resolved Perl binary path.
    pub fn perl_binary(&self) -> &Path {
        &self.perl_binary
    }

    /// Consume the profile, yielding the resolved Perl binary path.
    pub fn into_perl_binary(self) -> PathBuf {
        self.perl_binary
    }

    /// Probe and return the interpreter version string — the value of Perl's
    /// `$]` (e.g. `"5.038002"`).
    ///
    /// The probe runs `perl -e 'print $]'` through
    /// [`PerlOracleEnv::for_version_probe`], which denies ambient `PERL5LIB` /
    /// `PERL5OPT` / `local::lib` so the reported version is deterministic
    /// regardless of the editor's environment (the #8688 contract).
    ///
    /// Absolute, stat-able interpreters are cached process-wide, keyed by the
    /// binary path and guarded by the binary's fingerprint (length + mtime), so
    /// the subprocess runs at most once per distinct interpreter build. A bare
    /// command name (e.g. `perl_path = "perl"`, resolved on `PATH` by the
    /// subprocess) cannot be stat'd from the cwd and therefore has no
    /// fingerprint — it is still probed, but uncached. Returns `None` only when
    /// the probe itself fails or its output is not valid UTF-8.
    ///
    /// [`PerlOracleEnv::for_version_probe`]: super::PerlOracleEnv::for_version_probe
    #[cfg(not(target_arch = "wasm32"))]
    pub fn version(&self) -> Option<String> {
        // A bare command name has no fingerprint (cannot be stat'd from the
        // cwd). We still probe it — `for_version_probe` resolves the name on
        // PATH — but skip the cache, since there is no fingerprint to guard an
        // entry against an interpreter swap.
        let fingerprint = perl_binary_fingerprint(&self.perl_binary);

        if let Some(ref fingerprint) = fingerprint
            && let Ok(cache) = PERL_VERSION_CACHE.lock()
            && let Some((cached_fingerprint, cached_version)) = cache.get(&self.perl_binary)
            && cached_fingerprint == fingerprint
        {
            return cached_version.clone();
        }

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let oracle = super::PerlOracleEnv::for_version_probe(self.perl_binary.clone(), cwd);
        let detected_version =
            oracle.into_command().arg("-e").arg("print $]").output().ok().and_then(|out| {
                if out.status.success() { String::from_utf8(out.stdout).ok() } else { None }
            });

        if let Some(fingerprint) = fingerprint
            && let Ok(mut cache) = PERL_VERSION_CACHE.lock()
        {
            cache.insert(self.perl_binary.clone(), (fingerprint, detected_version.clone()));
        }

        detected_version
    }

    /// Probe the interpreter version string.
    ///
    /// On WASM there is no subprocess support, so this always returns `None`.
    #[cfg(target_arch = "wasm32")]
    pub fn version(&self) -> Option<String> {
        None
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// An explicit, non-empty `perl_path` is taken verbatim and wins over the
    /// toolchain resolver — without any existence check (callers validate).
    #[test]
    fn resolve_prefers_explicit_perl_path_verbatim() -> TestResult {
        let config = WorkspaceConfig {
            // Deliberately non-existent: resolution must not validate existence.
            perl_path: Some("/nonexistent/custom/perl".to_string()),
            ..WorkspaceConfig::default()
        };

        let profile = PerlToolchainProfile::resolve(&config)
            .ok_or("explicit perl_path must always resolve")?;
        assert_eq!(profile.perl_binary(), Path::new("/nonexistent/custom/perl"));
        Ok(())
    }

    /// An empty-string `perl_path` is treated as unset and falls through to the
    /// toolchain resolver (matching `.filter(|p| !p.is_empty())`).
    #[test]
    fn resolve_treats_empty_perl_path_as_unset() -> TestResult {
        let config =
            WorkspaceConfig { perl_path: Some(String::new()), ..WorkspaceConfig::default() };

        // We can't assert the resolved value (depends on the host toolchain),
        // but it must NOT be the empty path: the empty string was rejected.
        if let Some(profile) = PerlToolchainProfile::resolve(&config) {
            assert_ne!(profile.perl_binary(), Path::new(""));
        }
        Ok(())
    }

    /// `into_perl_binary` returns the same path that `perl_binary` borrows.
    #[test]
    fn into_perl_binary_matches_borrow() -> TestResult {
        let config = WorkspaceConfig {
            perl_path: Some("/explicit/perl".to_string()),
            ..WorkspaceConfig::default()
        };

        let profile = PerlToolchainProfile::resolve(&config)
            .ok_or("explicit perl_path must always resolve")?;
        let borrowed = profile.perl_binary().to_path_buf();
        assert_eq!(profile.into_perl_binary(), borrowed);
        Ok(())
    }

    /// `from_binary` stores the supplied path verbatim.
    #[test]
    fn from_binary_stores_path_verbatim() {
        let profile = PerlToolchainProfile::from_binary(PathBuf::from("/usr/bin/perl"));
        assert_eq!(profile.perl_binary(), Path::new("/usr/bin/perl"));
    }

    /// `version` returns `None` when the binary cannot be stat'd (no
    /// fingerprint), without attempting a subprocess.
    #[test]
    fn version_is_none_for_nonexistent_binary() {
        let profile =
            PerlToolchainProfile::from_binary(PathBuf::from("/nonexistent/definitely/not/perl"));
        assert_eq!(profile.version(), None);
    }

    /// `version` probes a real interpreter and returns a numeric `$]` string,
    /// and a second call returns the same value (served from cache). Skips when
    /// no Perl is available on the host.
    #[test]
    fn version_probes_and_caches_real_interpreter() -> TestResult {
        let perl = match crate::platform::resolve_perl_path_with_toolchain().ok() {
            Some(p) => p,
            None => return Ok(()), // no perl on host — skip
        };

        let profile = PerlToolchainProfile::from_binary(perl);
        let first = profile.version();
        if let Some(ref v) = first {
            assert!(
                v.trim().chars().next().is_some_and(|c| c.is_ascii_digit()),
                "version should start with a digit (value of $]); got: {v:?}"
            );
        }

        // Second call must agree with the first (fingerprint cache hit).
        assert_eq!(profile.version(), first, "cached version must match the first probe");
        Ok(())
    }

    /// A bare command name (`"perl"`) cannot be stat'd from the cwd, so it has
    /// no fingerprint — `version` must still probe it via PATH (uncached) rather
    /// than short-circuiting to `None`. Regression guard for the #1978 review.
    #[test]
    fn version_probes_bare_command_name_via_path() -> TestResult {
        // Deterministic precondition: only assert when a bare `perl` actually
        // runs from PATH (mirrors what `for_version_probe` will do).
        let perl_on_path = std::process::Command::new("perl")
            .arg("-e")
            .arg("print 1")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if !perl_on_path {
            return Ok(());
        }

        let profile = PerlToolchainProfile::from_binary(PathBuf::from("perl"));
        assert!(
            profile.version().is_some(),
            "bare-name `perl` must probe via PATH and return a version, not None"
        );
        Ok(())
    }
}
