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

use std::path::{Path, PathBuf};

use super::WorkspaceConfig;

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

    /// Borrow the resolved Perl binary path.
    pub fn perl_binary(&self) -> &Path {
        &self.perl_binary
    }

    /// Consume the profile, yielding the resolved Perl binary path.
    pub fn into_perl_binary(self) -> PathBuf {
        self.perl_binary
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
}
