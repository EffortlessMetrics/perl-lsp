//! Benchmark task implementation
//!
//! This module provides benchmarking capabilities using criterion.

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run(name: Option<String>, save: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    spinner.set_message("Running benchmarks");

    // Build arguments for cargo bench
    let mut args = vec!["bench"];
    
    if let Some(bench_name) = name {
        args.push("--bench");
        args.push(&bench_name);
    }

    // Execute benchmarks
    let status = cmd("cargo", &args).run().context("Failed to run benchmarks")?;

    if status.status.success() {
        spinner.finish_with_message("✅ Benchmarks completed");
    } else {
        spinner.finish_with_message("❌ Benchmarks failed");
        return Err(color_eyre::eyre::eyre!(
            "Benchmarks failed with status: {}",
            status.status
        ));
    }

    if save {
        spinner.set_message("Saving benchmark results");
        // Note: Criterion automatically saves results to target/criterion
        spinner.finish_with_message("✅ Benchmark results saved to target/criterion");
    }

    Ok(())
}
