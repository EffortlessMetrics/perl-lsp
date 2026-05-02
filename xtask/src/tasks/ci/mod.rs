//! Lean CI task for constrained environments
//!
//! This task runs a complete CI suite optimized for resource-constrained
//! environments like WSL or low-memory systems. It includes:
//! - Format checking
//! - Clippy linting
//! - Per-crate testing with constrained resources
//! - Documentation validation

use color_eyre::eyre::Result;

mod context;
mod runner;
mod spinner;

const TEST_CRATES: [&str; 3] = ["perl-lexer", "perl-parser", "perl-lsp-rs"];

/// Run the full CI suite with resource constraints
pub fn run() -> Result<()> {
    let spinner = spinner::create_spinner()?;

    spinner.set_message("Setting up constrained environment...");
    context::prepare_environment()?;

    spinner.set_message("🔧 Checking code formatting...");
    runner::run_fmt_check()?;
    spinner.println("✓ Format check passed");

    spinner.set_message("🔧 Running clippy lints...");
    runner::run_clippy_check()?;
    spinner.println("✓ Clippy check passed");

    spinner.set_message("🧪 Running constrained test suite...");
    for crate_name in TEST_CRATES {
        spinner.set_message(format!("  Testing {}...", crate_name));
        runner::run_constrained_test(crate_name)?;
        spinner.println(format!("  ✓ {} tests passed", crate_name));
    }

    spinner.set_message("📚 Validating documentation...");
    runner::run_docs_check()?;
    spinner.println("✓ Documentation validation passed");

    spinner.finish_with_message("✅ All CI checks passed!");
    Ok(())
}

/// Run format and clippy checks only (no tests)
pub fn check_only() -> Result<()> {
    let spinner = spinner::create_spinner()?;

    context::prepare_environment()?;

    spinner.set_message("🔧 Checking code formatting...");
    runner::run_fmt_check()?;

    spinner.set_message("🔧 Running clippy lints...");
    runner::run_clippy_check()?;

    spinner.finish_with_message("✅ All checks passed!");
    Ok(())
}
