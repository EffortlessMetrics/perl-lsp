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
    if let Some(features) = &features {
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
            TestSuite::Heredoc => {
                // Run heredoc-specific tests one by one
                spinner.set_message("Running heredoc tests");

                // We need to run each test file separately
                let heredoc_tests = vec![
                    "heredoc_missing_features_tests",
                    "heredoc_integration_tests",
                    "comprehensive_heredoc_tests",
                ];

                let mut all_passed = true;

                // Features are available from the function parameter

                for test_name in heredoc_tests {
                    spinner.set_message(format!("Running {}", test_name));

                    // Build args for this specific test
                    let mut test_args = vec!["test".to_string()];
                    if release {
                        test_args.push("--release".to_string());
                    }

                    // Add features
                    if let Some(ref feat) = features {
                        if !feat.is_empty() {
                            test_args.push("--features".to_string());
                            test_args.push(feat.join(","));
                        }
                    }

                    test_args.push("--test".to_string());
                    test_args.push(test_name.to_string());

                    if verbose {
                        test_args.push("--".to_string());
                        test_args.push("--nocapture".to_string());
                    }

                    // Convert to &str for cmd
                    let test_args_refs: Vec<&str> = test_args.iter().map(|s| s.as_str()).collect();

                    let status = cmd("cargo", &test_args_refs)
                        .run()
                        .context(format!("Failed to run {}", test_name))?;

                    if !status.status.success() {
                        all_passed = false;
                        spinner.finish_with_message(format!("❌ {} failed", test_name));
                        break;
                    }
                }

                if all_passed {
                    spinner.finish_with_message("✅ All heredoc tests passed");
                }
                return Ok(());
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
