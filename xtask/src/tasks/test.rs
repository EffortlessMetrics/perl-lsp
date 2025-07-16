//! Test task implementation

use crate::types::TestSuite;
use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run(
    release: bool,
    suite: Option<TestSuite>,
    features: Option<Vec<String>>,
    verbose: bool,
    _coverage: bool,
) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    // Determine test profile
    let profile = if release { "release" } else { "debug" };
    spinner.set_message(format!("Running tests ({})", profile));

    // Test command
    let mut args = vec!["test"];
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
    }

    // Add verbose flag
    if verbose {
        args.push("--");
        args.push("--nocapture");
    }

    // Handle specific test suites
    if let Some(suite) = suite {
        match suite {
            TestSuite::Unit => {
                args.push("--lib");
                args.push("--");
                args.push("--test-threads=1");
            }
            TestSuite::Integration => {
                args.push("--test");
            }
            TestSuite::Property => {
                args.push("--lib");
                args.push("--");
                args.push("property");
            }
            TestSuite::Performance => {
                args.push("--bench");
            }
            TestSuite::All => {
                // Run all tests
            }
        }
    }

    // Execute tests
    let status = cmd("cargo", &args).run().context("Failed to run tests")?;

    if status.status.success() {
        spinner.finish_with_message(format!("✅ Tests passed ({})", profile));
    } else {
        spinner.finish_with_message("❌ Tests failed");
        return Err(color_eyre::eyre::eyre!(
            "Tests failed with status: {}",
            status.status
        ));
    }

    Ok(())
}
