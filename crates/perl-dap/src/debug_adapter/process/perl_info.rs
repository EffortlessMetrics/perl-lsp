//! Perl interpreter discovery and version reporting for process launch diagnostics.

use crate::platform::PerlInterpreterResult;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PerlBinaryFingerprint {
    len: u64,
    modified: Option<SystemTime>,
}

static PERL_VERSION_CACHE: LazyLock<
    Mutex<HashMap<PathBuf, (PerlBinaryFingerprint, Option<String>)>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn perl_binary_fingerprint(perl_path: &Path) -> Option<PerlBinaryFingerprint> {
    let metadata = std::fs::metadata(perl_path).ok()?;
    let modified = metadata.modified().ok();
    Some(PerlBinaryFingerprint { len: metadata.len(), modified })
}

fn detect_perl_version_cached(perl_path: &Path) -> Option<String> {
    let fingerprint = perl_binary_fingerprint(perl_path)?;

    if let Ok(cache) = PERL_VERSION_CACHE.lock()
        && let Some((cached_fingerprint, cached_version)) = cache.get(perl_path)
        && *cached_fingerprint == fingerprint
    {
        return cached_version.clone();
    }

    // PerlOracleEnv denies PERL5LIB/PERL5OPT so the version number is
    // deterministic regardless of the editor's ambient environment (#8688).
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let oracle =
        perl_lsp_rs_core::config::PerlOracleEnv::for_version_probe(perl_path.to_path_buf(), cwd);
    let detected_version =
        oracle.into_command().arg("-e").arg("print $]").output().ok().and_then(|out| {
            if out.status.success() { String::from_utf8(out.stdout).ok() } else { None }
        });

    if let Ok(mut cache) = PERL_VERSION_CACHE.lock() {
        cache.insert(perl_path.to_path_buf(), (fingerprint, detected_version.clone()));
    }

    detected_version
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
