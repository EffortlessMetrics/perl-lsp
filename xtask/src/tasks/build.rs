//! Build task implementation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run(
    release: bool,
    features: Option<Vec<String>>,
    c_scanner: bool,
    rust_scanner: bool,
) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    // Determine build profile
    let profile = if release { "release" } else { "debug" };
    spinner.set_message(format!("Building tree-sitter-perl ({})", profile));

    // Build command
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }

    // Store strings that need to live long enough
    let mut feature_strings = Vec::new();

    // Add features
    if let Some(features) = features {
        if !features.is_empty() {
            let features_str = features.join(",");
            feature_strings.push(features_str);
            args.push("--features");
            args.push(feature_strings.last().unwrap());
        }
    } else {
        // Default features based on scanner preference
        if c_scanner && !rust_scanner {
            args.push("--features");
            args.push("c-scanner");
        } else if rust_scanner && !c_scanner {
            args.push("--features");
            args.push("rust-scanner");
        } else {
            // Default: both scanners
            args.push("--features");
            args.push("rust-scanner,c-scanner");
        }
    }

    // Execute build
    let status = cmd("cargo", &args)
        .run()
        .context("Failed to build project")?;

    if status.status.success() {
        spinner.finish_with_message(format!("✅ Built tree-sitter-perl ({})", profile));
    } else {
        spinner.finish_with_message("❌ Build failed");
        return Err(color_eyre::eyre::eyre!(
            "Build failed with status: {}",
            status.status
        ));
    }

    Ok(())
}
