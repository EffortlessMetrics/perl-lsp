//! Clean task implementation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;

pub fn run(all: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    
    spinner.set_message("Cleaning build artifacts");
    
    // Clean cargo artifacts
    let status = cmd("cargo", &["clean"])
        .run()
        .context("Failed to clean cargo artifacts")?;
    
    if !status.status.success() {
        spinner.finish_with_message("❌ Clean failed");
        return Err(color_eyre::eyre::eyre!("Clean failed"));
    }
    
    // Clean additional artifacts if requested
    if all {
        spinner.set_message("Cleaning all artifacts");
        
        // Remove generated files
        let generated_files = [
            "src/bindings.rs",
            "target",
            "Cargo.lock",
        ];
        
        for file in &generated_files {
            if fs::metadata(file).is_ok() {
                if fs::metadata(file)?.is_dir() {
                    fs::remove_dir_all(file)
                        .context(format!("Failed to remove directory: {}", file))?;
                } else {
                    fs::remove_file(file)
                        .context(format!("Failed to remove file: {}", file))?;
                }
            }
        }
    }
    
    spinner.finish_with_message("✅ Clean completed");
    Ok(())
} 