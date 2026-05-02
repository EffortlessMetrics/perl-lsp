use color_eyre::eyre::{Context, Result};
use duct::cmd;

pub(super) fn run_fmt_check() -> Result<()> {
    cmd("cargo", &["fmt", "--all", "--", "--check"]).run().context("Format check failed")?;
    Ok(())
}

pub(super) fn run_clippy_check() -> Result<()> {
    cmd("cargo", &["clippy", "--workspace", "--all-targets", "--", "-Dwarnings", "-Amissing_docs"])
        .run()
        .context("Clippy check failed")?;
    Ok(())
}

pub(super) fn run_constrained_test(crate_name: &str) -> Result<()> {
    cmd(
        "cargo",
        &["test", "-p", crate_name, "--tests", "--", "--test-threads=1", "--no-fail-fast", "-q"],
    )
    .run()
    .with_context(|| format!("{} tests failed", crate_name))?;

    Ok(())
}

pub(super) fn run_docs_check() -> Result<()> {
    cmd("cargo", &["doc", "-p", "perl-parser", "--no-deps"])
        .run()
        .context("Documentation build failed")?;
    Ok(())
}
