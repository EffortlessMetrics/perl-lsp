//! Code quality check task implementation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run(clippy: bool, fmt: bool, all: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    
    // Determine what to check
    let checks = if all {
        vec!["clippy", "fmt", "build"]
    } else {
        let mut checks = Vec::new();
        if clippy { checks.push("clippy"); }
        if fmt { checks.push("fmt"); }
        if checks.is_empty() { checks.push("build"); } // Default to build check
        checks
    };
    
    for check in checks {
        spinner.set_message(format!("Running {} check", check));
        
        let status = match check {
            "clippy" => {
                cmd("cargo", &["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])
                    .run()
                    .context("Clippy check failed")?
            }
            "fmt" => {
                cmd("cargo", &["fmt", "--all", "--", "--check"])
                    .run()
                    .context("Format check failed")?
            }
            "build" => {
                cmd("cargo", &["check", "--all-targets", "--all-features"])
                    .run()
                    .context("Build check failed")?
            }
            _ => continue,
        };
        
        if !status.status.success() {
            spinner.finish_with_message(format!("❌ {} check failed", check));
            return Err(color_eyre::eyre::eyre!("{} check failed", check));
        }
    }
    
    spinner.finish_with_message("✅ All checks passed");
    Ok(())
} 