//! Utility functions for xtask

use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::{Result, bail, eyre};

/// Get the project root directory using CARGO_MANIFEST_DIR.
/// This is more robust than current_dir() in CI environments.
pub fn project_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // xtask is in xtask/, so go up one level to get project root
    manifest_dir
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| eyre!("xtask should be in a subdirectory - invalid project structure"))
}

/// Run `cargo metadata --format-version 1` and return the raw JSON bytes.
///
/// Set `no_deps = true` to pass `--no-deps` (skips dependency resolution and
/// omits the `resolve` graph from the output — faster, but the caller will not
/// receive transitive-dep information).
///
/// Returns an error if `cargo metadata` exits non-zero.
pub fn run_cargo_metadata(no_deps: bool) -> Result<Vec<u8>> {
    let mut args = vec!["metadata", "--format-version", "1"];
    if no_deps {
        args.push("--no-deps");
    }
    let output = Command::new("cargo").args(&args).output()?;
    if !output.status.success() {
        bail!("cargo metadata failed:\n{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(output.stdout)
}

// ---------------------------------------------------------------------------
// Publish allowlist helpers (shared by publish_closure, count_ratchet,
// and publish_manifest_check)
// ---------------------------------------------------------------------------

/// Workspace-level metadata shape for publish allowlist access.
///
/// Covers the `[workspace.metadata]` table only — not the full cargo metadata
/// structure.  Used by `load_publish_allowlist()` and also re-exported so
/// `publish_closure` can layer its own `FullMetadata` on top without duplicating
/// the allowlist structs.
#[derive(serde::Deserialize)]
pub struct AllowlistMetadata {
    #[serde(rename = "metadata")]
    pub workspace_metadata: Option<WorkspacePublishMeta>,
}

/// `[workspace.metadata.publish]` section.
#[derive(serde::Deserialize)]
pub struct WorkspacePublishMeta {
    pub publish: Option<AllowList>,
}

/// `[workspace.metadata.publish.allow]` list.
#[derive(serde::Deserialize)]
pub struct AllowList {
    pub allow: Option<Vec<String>>,
}

/// Load `[workspace.metadata.publish.allow]` via `cargo metadata --no-deps`.
///
/// Returns the list of allowlisted crate names.
/// Errors if the section is absent or the list is empty.
pub fn load_publish_allowlist() -> color_eyre::eyre::Result<Vec<String>> {
    use color_eyre::eyre::{bail, eyre};
    let bytes = run_cargo_metadata(true)?;
    let meta: AllowlistMetadata =
        serde_json::from_slice(&bytes).map_err(|e| eyre!("Failed to parse cargo metadata: {e}"))?;
    let allow = meta
        .workspace_metadata
        .and_then(|m| m.publish)
        .and_then(|p| p.allow)
        .ok_or_else(|| eyre!("No [workspace.metadata.publish.allow] found in Cargo.toml"))?;
    if allow.is_empty() {
        bail!("Publish allowlist is empty");
    }
    Ok(allow)
}

/// Build constrained environment variables for resource-limited CI.
/// Merges with existing RUSTFLAGS instead of overwriting.
pub fn constrained_env_vars() -> Vec<(&'static str, String)> {
    let extra_flags = "-Copt-level=2 -Ccodegen-units=1 -Cdebuginfo=0";

    // Merge with existing RUSTFLAGS
    let rustflags = match std::env::var("RUSTFLAGS") {
        Ok(prev) if !prev.trim().is_empty() => format!("{} {}", prev, extra_flags),
        _ => extra_flags.to_string(),
    };

    vec![
        ("RUSTFLAGS", rustflags),
        ("CARGO_BUILD_JOBS", "2".to_string()),
        ("RUST_TEST_THREADS", "1".to_string()),
        ("RUST_BACKTRACE", "full".to_string()),
        ("CARGO_INCREMENTAL", "0".to_string()), // Smoother memory profile
        ("CARGO_TERM_COLOR", "always".to_string()), // Nicer CI logs
    ]
}
