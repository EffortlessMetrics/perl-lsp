//! Benchmark task implementation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run(name: Option<String>, _save: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    
    spinner.set_message("Running benchmarks");
    
    let mut args = vec!["bench"];
    if let Some(name) = name {
        let name_str = name;
        args.push("--bench");
        args.push(&name_str);
    }
    
    let status = cmd("cargo", &args)
        .run()
        .context("Failed to run benchmarks")?;
    
    if status.status.success() {
        spinner.finish_with_message("✅ Benchmarks completed");
    } else {
        spinner.finish_with_message("❌ Benchmarks failed");
        return Err(color_eyre::eyre::eyre!("Benchmarks failed"));
    }
    
    Ok(())
} 