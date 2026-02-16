//! Utility functions for xtask

use std::path::PathBuf;

use color_eyre::eyre::{Result, eyre};

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
