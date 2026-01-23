//! Test runner with coverage reporting for LSP features
//!
//! This module provides comprehensive test execution and coverage analysis
//! for all LSP features, ensuring complete end-to-end testing.

#![allow(clippy::collapsible_if)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use colored::*;
use std::collections::HashMap;
use std::time::Instant;

/// LSP Feature coverage tracking
#[derive(Debug, Clone)]
pub struct FeatureCoverage {
    pub name: String,
    pub tested: bool,
    pub test_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub coverage_percent: f64,
}

/// Test result tracking
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u128,
    pub error: Option<String>,
}

/// Main test runner for LSP features
pub struct LspTestRunner {
    features: HashMap<String, FeatureCoverage>,
    test_results: Vec<TestResult>,
    start_time: Instant,
}

impl Default for LspTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl LspTestRunner {
    pub fn new() -> Self {
        let mut features = HashMap::new();

        // Initialize all 25+ LSP features
        let feature_list = vec![
            "Initialization",
            "Diagnostics",
            "Completion",
            "Definition",
            "References",
            "Hover",
            "SignatureHelp",
            "DocumentSymbol",
            "WorkspaceSymbol",
            "CodeAction",
            "CodeLens",
            "DocumentFormatting",
            "RangeFormatting",
            "Rename",
            "FoldingRange",
            "SelectionRange",
            "SemanticTokens",
            "CallHierarchy",
            "InlayHint",
            "ExecuteCommand",
            "LinkedEditingRange",
            "Moniker",
            "TypeHierarchy",
            "InlineValue",
            "DocumentHighlight",
            "DocumentLink",
            "DocumentColor",
            "ColorPresentation",
            "Declaration",
            "Implementation",
            "TypeDefinition",
        ];

        for feature in feature_list {
            features.insert(
                feature.to_string(),
                FeatureCoverage {
                    name: feature.to_string(),
                    tested: false,
                    test_count: 0,
                    pass_count: 0,
                    fail_count: 0,
                    coverage_percent: 0.0,
                },
            );
        }

        Self { features, test_results: Vec::new(), start_time: Instant::now() }
    }

    /// Run a single test and track results
    pub fn run_test<F>(&mut self, name: &str, feature: &str, test_fn: F) -> bool
    where
        F: FnOnce() -> Result<(), String>,
    {
        let start = Instant::now();
        let result = test_fn();
        let duration = start.elapsed().as_millis();

        let passed = result.is_ok();

        // Update feature coverage
        if let Some(feature_cov) = self.features.get_mut(feature) {
            feature_cov.tested = true;
            feature_cov.test_count += 1;
            if passed {
                feature_cov.pass_count += 1;
            } else {
                feature_cov.fail_count += 1;
            }
            feature_cov.coverage_percent =
                (feature_cov.pass_count as f64 / feature_cov.test_count as f64) * 100.0;
        }

        // Store test result
        self.test_results.push(TestResult {
            name: name.to_string(),
            passed,
            duration_ms: duration,
            error: result.err(),
        });

        // Print immediate feedback
        if passed {
            println!("{} {} ({} ms)", "✓".green(), name, duration);
        } else {
            println!("{} {} ({} ms)", "✗".red(), name, duration);
            if let Some(ref err) = self.test_results.last().unwrap().error {
                println!("  {}", err.red());
            }
        }

        passed
    }

    /// Run a test suite for a specific feature
    pub fn run_feature_suite<F>(&mut self, feature: &str, suite_fn: F)
    where
        F: FnOnce(&mut Self),
    {
        println!("\n{}", format!("Testing {}", feature).bold().blue());
        println!("{}", "─".repeat(50));
        suite_fn(self);
    }

    /// Generate coverage report
    pub fn generate_report(&self) {
        let total_duration = self.start_time.elapsed();

        println!("\n{}", "═".repeat(60).bold());
        println!("{}", "LSP TEST COVERAGE REPORT".bold().cyan());
        println!("{}", "═".repeat(60).bold());

        // Feature coverage summary
        println!("\n{}", "Feature Coverage:".bold());
        println!("{}", "─".repeat(60));

        let mut tested_features = 0;
        let mut _total_tests = 0;
        let mut _total_passed = 0;

        for (name, coverage) in &self.features {
            if coverage.tested {
                tested_features += 1;
                _total_tests += coverage.test_count;
                _total_passed += coverage.pass_count;

                let status = if coverage.fail_count == 0 {
                    "✓".green()
                } else if coverage.pass_count > 0 {
                    "⚠".yellow()
                } else {
                    "✗".red()
                };

                println!(
                    "{} {:<25} {:>3}/{:<3} tests passed ({:>5.1}%)",
                    status,
                    name,
                    coverage.pass_count,
                    coverage.test_count,
                    coverage.coverage_percent
                );
            }
        }

        // Untested features
        let untested: Vec<_> =
            self.features.values().filter(|f| !f.tested).map(|f| f.name.as_str()).collect();

        if !untested.is_empty() {
            println!("\n{}", "Untested Features:".bold().yellow());
            for feature in untested {
                println!("  {} {}", "○".yellow(), feature);
            }
        }

        // Test execution summary
        println!("\n{}", "Test Execution Summary:".bold());
        println!("{}", "─".repeat(60));

        let passed_tests = self.test_results.iter().filter(|t| t.passed).count();
        let failed_tests = self.test_results.len() - passed_tests;

        println!("Total Tests Run: {}", self.test_results.len());
        println!("Tests Passed: {} {}", passed_tests, "✓".green());
        println!(
            "Tests Failed: {} {}",
            failed_tests,
            if failed_tests > 0 { "✗".red() } else { "✓".green() }
        );
        println!("Features Tested: {}/{}", tested_features, self.features.len());
        println!(
            "Overall Coverage: {:.1}%",
            (tested_features as f64 / self.features.len() as f64) * 100.0
        );
        println!("Total Duration: {:.2}s", total_duration.as_secs_f64());

        // Performance analysis
        if !self.test_results.is_empty() {
            println!("\n{}", "Performance Analysis:".bold());
            println!("{}", "─".repeat(60));

            let mut sorted_results = self.test_results.clone();
            sorted_results.sort_by_key(|r| r.duration_ms);

            println!(
                "Fastest Test: {} ({} ms)",
                sorted_results.first().unwrap().name,
                sorted_results.first().unwrap().duration_ms
            );

            println!(
                "Slowest Test: {} ({} ms)",
                sorted_results.last().unwrap().name,
                sorted_results.last().unwrap().duration_ms
            );

            let avg_duration = sorted_results.iter().map(|r| r.duration_ms).sum::<u128>()
                / sorted_results.len() as u128;

            println!("Average Duration: {} ms", avg_duration);
        }

        // Failed tests detail
        let failed: Vec<_> = self.test_results.iter().filter(|t| !t.passed).collect();

        if !failed.is_empty() {
            println!("\n{}", "Failed Tests:".bold().red());
            println!("{}", "─".repeat(60));

            for test in failed {
                println!("{} {}", "✗".red(), test.name);
                if let Some(ref err) = test.error {
                    println!("  Error: {}", err);
                }
            }
        }

        // Final verdict
        println!("\n{}", "═".repeat(60).bold());

        if failed_tests == 0 && tested_features == self.features.len() {
            println!("{}", "✅ ALL TESTS PASSED - 100% COVERAGE".bold().green());
        } else if failed_tests == 0 {
            println!(
                "{}",
                format!(
                    "✅ ALL TESTS PASSED - {:.1}% COVERAGE",
                    (tested_features as f64 / self.features.len() as f64) * 100.0
                )
                .bold()
                .green()
            );
        } else {
            println!(
                "{}",
                format!(
                    "⚠️  {} TESTS FAILED - {:.1}% COVERAGE",
                    failed_tests,
                    (tested_features as f64 / self.features.len() as f64) * 100.0
                )
                .bold()
                .yellow()
            );
        }

        println!("{}", "═".repeat(60).bold());
    }

    /// Generate JUnit XML report for CI integration
    pub fn generate_junit_xml(&self, filename: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(filename)?;

        writeln!(file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(
            file,
            "<testsuites name=\"LSP E2E Tests\" tests=\"{}\" failures=\"{}\" time=\"{:.3}\">",
            self.test_results.len(),
            self.test_results.iter().filter(|t| !t.passed).count(),
            self.start_time.elapsed().as_secs_f64()
        )?;

        // Group tests by feature
        for (feature, coverage) in &self.features {
            if coverage.tested {
                writeln!(
                    file,
                    "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" time=\"0\">",
                    feature, coverage.test_count, coverage.fail_count
                )?;

                for test in &self.test_results {
                    if test.name.contains(feature) {
                        writeln!(
                            file,
                            "    <testcase name=\"{}\" time=\"{:.3}\">",
                            test.name,
                            test.duration_ms as f64 / 1000.0
                        )?;

                        if !test.passed {
                            if let Some(ref err) = test.error {
                                writeln!(
                                    file,
                                    "      <failure message=\"Test failed\">{}</failure>",
                                    err
                                )?;
                            }
                        }

                        writeln!(file, "    </testcase>")?;
                    }
                }

                writeln!(file, "  </testsuite>")?;
            }
        }

        writeln!(file, "</testsuites>")?;

        Ok(())
    }

    /// Generate markdown report for documentation
    pub fn generate_markdown_report(&self, filename: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(filename)?;

        writeln!(file, "# LSP E2E Test Coverage Report\n")?;
        writeln!(
            file,
            "Generated: {}\n",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        )?;

        writeln!(file, "## Summary\n")?;

        let passed_tests = self.test_results.iter().filter(|t| t.passed).count();
        let tested_features = self.features.values().filter(|f| f.tested).count();

        writeln!(file, "- **Total Tests**: {}", self.test_results.len())?;
        writeln!(file, "- **Passed**: {} ✅", passed_tests)?;
        writeln!(file, "- **Failed**: {} ❌", self.test_results.len() - passed_tests)?;
        writeln!(file, "- **Features Tested**: {}/{}", tested_features, self.features.len())?;
        writeln!(
            file,
            "- **Coverage**: {:.1}%\n",
            (tested_features as f64 / self.features.len() as f64) * 100.0
        )?;

        writeln!(file, "## Feature Coverage\n")?;
        writeln!(file, "| Feature | Status | Tests | Passed | Coverage |")?;
        writeln!(file, "|---------|--------|-------|--------|----------|")?;

        for coverage in self.features.values() {
            if coverage.tested {
                let status = if coverage.fail_count == 0 { "✅" } else { "⚠️" };
                writeln!(
                    file,
                    "| {} | {} | {} | {} | {:.1}% |",
                    coverage.name,
                    status,
                    coverage.test_count,
                    coverage.pass_count,
                    coverage.coverage_percent
                )?;
            } else {
                writeln!(file, "| {} | ⭕ | 0 | 0 | 0% |", coverage.name)?;
            }
        }

        if self.test_results.iter().any(|t| !t.passed) {
            writeln!(file, "\n## Failed Tests\n")?;

            for test in &self.test_results {
                if !test.passed {
                    writeln!(file, "### ❌ {}\n", test.name)?;
                    if let Some(ref err) = test.error {
                        writeln!(file, "```\n{}\n```\n", err)?;
                    }
                }
            }
        }

        writeln!(file, "\n## Performance\n")?;

        if !self.test_results.is_empty() {
            let total_ms: u128 = self.test_results.iter().map(|r| r.duration_ms).sum();
            let avg_ms = total_ms / self.test_results.len() as u128;

            writeln!(file, "- **Total Duration**: {:.2}s", total_ms as f64 / 1000.0)?;
            writeln!(file, "- **Average Test Duration**: {}ms", avg_ms)?;

            let mut sorted = self.test_results.clone();
            sorted.sort_by_key(|r| r.duration_ms);

            writeln!(
                file,
                "- **Fastest Test**: {} ({}ms)",
                sorted.first().unwrap().name,
                sorted.first().unwrap().duration_ms
            )?;

            writeln!(
                file,
                "- **Slowest Test**: {} ({}ms)",
                sorted.last().unwrap().name,
                sorted.last().unwrap().duration_ms
            )?;
        }

        Ok(())
    }
}

// Colored output support
mod colored {
    pub trait Colorize {
        fn red(&self) -> String;
        fn green(&self) -> String;
        fn yellow(&self) -> String;
        fn blue(&self) -> String;
        fn cyan(&self) -> String;
        fn bold(&self) -> String;
    }

    impl Colorize for &str {
        fn red(&self) -> String {
            format!("\x1b[31m{}\x1b[0m", self)
        }

        fn green(&self) -> String {
            format!("\x1b[32m{}\x1b[0m", self)
        }

        fn yellow(&self) -> String {
            format!("\x1b[33m{}\x1b[0m", self)
        }

        fn blue(&self) -> String {
            format!("\x1b[34m{}\x1b[0m", self)
        }

        fn cyan(&self) -> String {
            format!("\x1b[36m{}\x1b[0m", self)
        }

        fn bold(&self) -> String {
            format!("\x1b[1m{}\x1b[0m", self)
        }
    }

    impl Colorize for String {
        fn red(&self) -> String {
            self.as_str().red()
        }

        fn green(&self) -> String {
            self.as_str().green()
        }

        fn yellow(&self) -> String {
            self.as_str().yellow()
        }

        fn blue(&self) -> String {
            self.as_str().blue()
        }

        fn cyan(&self) -> String {
            self.as_str().cyan()
        }

        fn bold(&self) -> String {
            self.as_str().bold()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_initialization() {
        let runner = LspTestRunner::new();
        assert!(runner.features.len() > 25);
        assert_eq!(runner.test_results.len(), 0);
    }

    #[test]
    fn test_runner_execution() {
        let mut runner = LspTestRunner::new();

        let passed = runner.run_test("test_completion", "Completion", || Ok(()));

        assert!(passed);
        assert_eq!(runner.test_results.len(), 1);
        assert!(runner.features.get("Completion").unwrap().tested);
    }
}
