use color_eyre::eyre::{Context, Result};
use duct::{Expression, cmd};

use crate::utils::constrained_env_vars;

pub(super) fn run_fmt_check() -> Result<()> {
    constrained_cmd("cargo", &["fmt", "--all", "--", "--check"])
        .run()
        .context("Format check failed")?;
    Ok(())
}

pub(super) fn run_clippy_check() -> Result<()> {
    constrained_cmd(
        "cargo",
        &["clippy", "--workspace", "--all-targets", "--", "-Dwarnings", "-Amissing_docs"],
    )
    .run()
    .context("Clippy check failed")?;
    Ok(())
}

pub(super) fn run_constrained_test(crate_name: &str) -> Result<()> {
    constrained_cmd(
        "cargo",
        &["test", "-p", crate_name, "--tests", "--", "--test-threads=1", "--no-fail-fast", "-q"],
    )
    .run()
    .with_context(|| format!("{} tests failed", crate_name))?;

    Ok(())
}

pub(super) fn run_docs_check() -> Result<()> {
    constrained_cmd("cargo", &["doc", "-p", "perl-parser", "--no-deps"])
        .run()
        .context("Documentation build failed")?;
    Ok(())
}

fn constrained_cmd(program: &str, args: &[&str]) -> Expression {
    constrained_env_vars()
        .into_iter()
        .fold(cmd(program, args), |expr, (key, value)| expr.env(key, value))
}
