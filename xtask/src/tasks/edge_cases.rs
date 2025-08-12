//! Edge case testing automation

use color_eyre::eyre::{Context, Result};
use std::process::Command;

/// Run edge case tests with optional benchmarks and coverage
pub fn run(bench: bool, coverage: bool, test: Option<String>) -> Result<()> {
    println!("🔍 Testing edge case handling...");

    let features = "pure-rust test-utils";

    // Run specific test if provided
    if let Some(test_name) = test {
        println!("Running specific edge case test: {}", test_name);
        let status = Command::new("cargo")
            .args(&[
                "test",
                "--features",
                features,
                &test_name,
                "--",
                "--nocapture",
            ])
            .status()
            .context("Failed to run specific edge case test")?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!("Edge case test failed"));
        }

        return Ok(());
    }

    // Run edge case unit tests
    println!("\n📝 Running edge case tests...");
    let status = Command::new("cargo")
        .args(&[
            "test",
            "--features",
            features,
            "edge_case_tests",
            "--",
            "--nocapture",
        ])
        .status()
        .context("Failed to run edge case tests")?;

    if !status.success() {
        return Err(color_eyre::eyre::eyre!("Edge case tests failed"));
    }

    // Run integration tests
    println!("\n🔗 Running integration tests...");
    let integration_tests = vec![
        "test_edge_case_integration",
        "test_recovery_mode_effectiveness",
        "test_encoding_aware_heredocs",
    ];

    for test in integration_tests {
        let status = Command::new("cargo")
            .args(&["test", "--features", features, test, "--", "--nocapture"])
            .status()
            .context(format!("Failed to run {}", test))?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!("Integration test {} failed", test));
        }
    }

    // Run benchmarks if requested
    if bench {
        println!("\n⚡ Running edge case benchmarks...");
        let status = Command::new("cargo")
            .args(&["bench", "--features", features, "edge_case_benchmarks"])
            .status()
            .context("Failed to run benchmarks")?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!("Benchmarks failed"));
        }
    }

    // Run examples
    println!("\n📚 Running edge case examples...");
    let examples = vec![
        "edge_case_demo",
        "anti_pattern_analysis",
        "tree_sitter_compatibility",
    ];

    for example in examples {
        let status = Command::new("cargo")
            .args(&["run", "--features", features, "--example", example])
            .status()
            .context(format!("Failed to run example {}", example))?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!("Example {} failed", example));
        }
    }

    // Generate coverage report if requested
    if coverage {
        println!("\n📊 Generating coverage report...");
        let status = Command::new("cargo")
            .args(&[
                "tarpaulin",
                "--features",
                features,
                "--out",
                "Html",
                "--output-dir",
                "target/coverage",
            ])
            .status()
            .context("Failed to generate coverage report")?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!("Coverage generation failed"));
        }

        println!("✅ Coverage report generated at target/coverage/index.html");
    }

    println!("\n✅ All edge case tests passed!");
    Ok(())
}
