//! Format task implementation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run(check: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}").unwrap(),
    );

    let action = if check { "Checking" } else { "Formatting" };
    spinner.set_message(format!("{} code", action));

    let mut args = vec!["fmt", "--all"];
    if check {
        args.push("--");
        args.push("--check");
    }

    let status = cmd("cargo", &args).run().context("Failed to format code")?;

    if status.status.success() {
        spinner.finish_with_message(format!(
            "✅ Code {} successfully",
            if check { "check passed" } else { "formatted" }
        ));
    } else {
        spinner.finish_with_message(format!(
            "❌ Code {} failed",
            if check { "check" } else { "formatting" }
        ));
        return Err(color_eyre::eyre::eyre!(
            "Code {} failed",
            if check { "check" } else { "formatting" }
        ));
    }

    Ok(())
}
