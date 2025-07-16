//! Documentation task implementation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run(open: bool, all_features: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    
    spinner.set_message("Building documentation");
    
    let mut args = vec!["doc"];
    if all_features {
        args.push("--all-features");
    }
    if open {
        args.push("--open");
    }
    
    let status = cmd("cargo", &args)
        .run()
        .context("Failed to build documentation")?;
    
    if status.status.success() {
        spinner.finish_with_message("✅ Documentation built");
    } else {
        spinner.finish_with_message("❌ Documentation build failed");
        return Err(color_eyre::eyre::eyre!("Documentation build failed"));
    }
    
    Ok(())
} 