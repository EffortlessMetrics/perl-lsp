//! Perl interpreter discovery and version reporting for process launch diagnostics.

use crate::platform::PerlInterpreterResult;
use std::path::Path;

/// Probe the interpreter version (value of `$]`), cached per binary fingerprint.
///
/// Delegates to [`PerlToolchainProfile::version`], the single source of truth
/// for interpreter identity shared by the LSP analysis seams and the DAP launch
/// path. The probe denies ambient `PERL5LIB`/`PERL5OPT` so the version is
/// deterministic regardless of the editor's environment (the #8688 contract).
///
/// [`PerlToolchainProfile::version`]: perl_lsp_rs_core::config::PerlToolchainProfile::version
fn detect_perl_version_cached(perl_path: &Path) -> Option<String> {
    perl_lsp_rs_core::config::PerlToolchainProfile::from_binary(perl_path.to_path_buf()).version()
}

/// Try to detect the Perl interpreter available on the system and return a human-readable
/// summary string.
///
/// Uses `resolve_perl_path_with_toolchain()` to find Perl (checks perlbrew, plenv, then PATH),
/// then runs `perl -e 'print $]'` to get the version number.  Returns a string describing what
/// was found, or a "not found" / install-hint message suitable for inclusion in error messages.
pub(super) fn detect_perl_info() -> String {
    match crate::platform::find_perl_interpreter_cached(None) {
        PerlInterpreterResult::ConfiguredPath(path)
        | PerlInterpreterResult::FoundOnPath(path)
        | PerlInterpreterResult::FoundViaFallback { path, .. } => {
            match detect_perl_version_cached(&path) {
                Some(v) if !v.trim().is_empty() => {
                    format!("Found Perl at {} (version {})", path.display(), v.trim())
                }
                _ => format!("Found Perl at {}", path.display()),
            }
        }
        PerlInterpreterResult::NotFound { .. } => {
            #[cfg(windows)]
            {
                "Perl was not found on PATH. Install Perl from https://strawberryperl.com \
                 or set a custom path."
                    .to_string()
            }
            #[cfg(not(windows))]
            {
                "Perl was not found on PATH. Install Perl via your package manager \
                 (e.g. `apt install perl` or `brew install perl`) or set a custom path."
                    .to_string()
            }
        }
    }
}
