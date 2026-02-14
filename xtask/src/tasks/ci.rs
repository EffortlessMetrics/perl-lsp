//! Lean CI task for constrained environments
//!
//! This task runs a complete CI suite optimized for resource-constrained
//! environments like WSL or low-memory systems. It includes:
//! - Format checking
//! - Clippy linting
//! - Per-crate testing with constrained resources
//! - Documentation validation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

use crate::utils::{constrained_env_vars, project_root};

/// Run the full CI suite with resource constraints
pub fn run() -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .context("Failed to create progress spinner template")?,
    );

    // Change to project root
    let root = project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    spinner.set_message("Setting up constrained environment...");
    let env_vars = constrained_env_vars();

    // Apply environment variables for all subsequent commands
    // SAFETY: We're in a single-threaded xtask binary with no concurrent environment access
    for (key, value) in &env_vars {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    // Step 1: Format check
    spinner.set_message("🔧 Checking code formatting...");
    cmd("cargo", &["fmt", "--all", "--", "--check"]).run().context("Format check failed")?;
    spinner.println("✓ Format check passed");

    // Step 2: Clippy check
    spinner.set_message("🔧 Running clippy lints...");
    cmd("cargo", &["clippy", "--workspace", "--all-targets", "--", "-Dwarnings", "-Amissing_docs"])
        .run()
        .context("Clippy check failed")?;
    spinner.println("✓ Clippy check passed");

    // Step 3: Test each crate explicitly with constrained resources
    let test_crates = ["perl-lexer", "perl-parser", "perl-lsp"];

    spinner.set_message("🧪 Running constrained test suite...");
    for crate_name in &test_crates {
        spinner.set_message(format!("  Testing {}...", crate_name));
        cmd(
            "cargo",
            &[
                "test",
                "-p",
                crate_name,
                "--tests",
                "--",
                "--test-threads=1",
                "--no-fail-fast",
                "-q",
            ],
        )
        .run()
        .with_context(|| format!("{} tests failed", crate_name))?;
        spinner.println(format!("  ✓ {} tests passed", crate_name));
    }

    // Step 4: Documentation validation
    spinner.set_message("📚 Validating documentation...");
    cmd("cargo", &["doc", "-p", "perl-parser", "--no-deps"])
        .run()
        .context("Documentation build failed")?;
    spinner.println("✓ Documentation validation passed");

    spinner.finish_with_message("✅ All CI checks passed!");
    Ok(())
}

/// Run format and clippy checks only (no tests)
pub fn check_only() -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .context("Failed to create progress spinner template")?,
    );

    let root = project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    let env_vars = constrained_env_vars();
    // SAFETY: We're in a single-threaded xtask binary with no concurrent environment access
    for (key, value) in &env_vars {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    spinner.set_message("🔧 Checking code formatting...");
    cmd("cargo", &["fmt", "--all", "--", "--check"]).run().context("Format check failed")?;

    spinner.set_message("🔧 Running clippy lints...");
    cmd("cargo", &["clippy", "--workspace", "--all-targets", "--", "-Dwarnings", "-Amissing_docs"])
        .run()
        .context("Clippy check failed")?;

    spinner.finish_with_message("✅ All checks passed!");
    Ok(())
}
