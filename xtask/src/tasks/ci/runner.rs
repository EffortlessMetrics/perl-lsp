use color_eyre::eyre::{Context, Result};
use duct::cmd;

use crate::tasks::fmt as fmt_task;

pub(super) fn run_fmt_check() -> Result<()> {
    run_fmt_check_with(|| fmt_task::run(true, None))?;
    Ok(())
}

fn run_fmt_check_with<F>(mut run_fmt: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    run_fmt().context("Format check failed")?;
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

#[cfg(test)]
mod tests {
    use super::{run_fmt_check, run_fmt_check_with};
    use color_eyre::eyre::{Result, eyre};

    #[test]
    fn ci_runner_fmt_check_uses_injected_package_formatter() -> Result<()> {
        let mut called = false;
        run_fmt_check_with(|| {
            called = true;
            Ok(())
        })?;

        assert!(called);
        Ok(())
    }

    #[test]
    fn ci_runner_fmt_check_preserves_format_context() -> Result<()> {
        let err = run_fmt_check_with(|| Err(eyre!("formatter unavailable")));
        let message = match err {
            Ok(()) => return Err(eyre!("expected fmt check failure")),
            Err(err) => err.chain().map(|cause| cause.to_string()).collect::<Vec<_>>().join("\n"),
        };

        assert!(message.contains("Format check failed"));
        assert!(message.contains("formatter unavailable"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn ci_runner_fmt_check_routes_to_package_formatter() -> Result<()> {
        let fake_cargo = crate::test_support::FakeCargo::install()?;

        run_fmt_check()?;

        let invocations = fake_cargo.invocations();
        assert!(invocations.iter().any(|line| line == "metadata --format-version 1 --no-deps"));
        assert!(invocations.iter().any(|line| {
            line.starts_with("fmt --manifest-path ") && line.ends_with(" -- --check")
        }));
        Ok(())
    }
}
