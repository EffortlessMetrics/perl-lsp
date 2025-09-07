//! LSP test performance optimization utilities

use color_eyre::eyre::{Result, eyre};
use std::path::Path;
use std::fs;
use regex::Regex;
use indicatif::{ProgressBar, ProgressStyle};

/// Optimization suggestions for LSP behavioral tests
pub struct TestOptimization {
    pub test_file: String,
    pub issues_found: Vec<PerformanceIssue>,
    pub total_estimated_time_ms: u64,
}

/// A performance issue found in a test file
#[derive(Clone)]
pub struct PerformanceIssue {
    pub issue_type: IssueType,
    pub line_number: usize,
    pub current_timeout_ms: u64,
    pub suggested_timeout_ms: u64,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum IssueType {
    LongWaitForSymbol,
    LongWaitForIdle,
    LongRequestTimeout,
    #[allow(dead_code)]
    PollingLoop,
    UnreasonableDelay,
}

impl TestOptimization {
    /// Analyze a test file for performance issues
    pub fn analyze_file<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let content = fs::read_to_string(&file_path)?;
        let file_name = file_path.as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut issues = Vec::new();
        let mut total_time = 0u64;

        // Analyze each line for performance issues
        for (line_num, line) in content.lines().enumerate() {
            if let Some(issue) = Self::analyze_line(line, line_num + 1) {
                total_time += issue.current_timeout_ms;
                issues.push(issue);
            }
        }

        Ok(TestOptimization {
            test_file: file_name,
            issues_found: issues,
            total_estimated_time_ms: total_time,
        })
    }

    /// Analyze a single line for performance issues
    fn analyze_line(line: &str, line_number: usize) -> Option<PerformanceIssue> {
        // Look for wait_for_symbol calls
        if let Some(captures) = Regex::new(r#"wait_for_symbol\([^,]+,\s*[^,]*,\s*Duration::from_millis\((\d+)\)"#)
            .ok()?
            .captures(line)
        {
            let timeout_ms: u64 = captures.get(1)?.as_str().parse().ok()?;
            if timeout_ms > 1000 {
                return Some(PerformanceIssue {
                    issue_type: IssueType::LongWaitForSymbol,
                    line_number,
                    current_timeout_ms: timeout_ms,
                    suggested_timeout_ms: 500.min(timeout_ms / 2),
                    description: format!("wait_for_symbol with {}ms timeout is too long", timeout_ms),
                });
            }
        }

        // Look for wait_for_idle calls
        if let Some(captures) = Regex::new(r#"wait_for_idle\(Duration::from_millis\((\d+)\)"#)
            .ok()?
            .captures(line)
        {
            let timeout_ms: u64 = captures.get(1)?.as_str().parse().ok()?;
            if timeout_ms > 500 {
                return Some(PerformanceIssue {
                    issue_type: IssueType::LongWaitForIdle,
                    line_number,
                    current_timeout_ms: timeout_ms,
                    suggested_timeout_ms: 200.min(timeout_ms / 3),
                    description: format!("wait_for_idle with {}ms timeout is too long", timeout_ms),
                });
            }
        }

        // Look for request_with_timeout calls
        if let Some(captures) = Regex::new(r#"request_with_timeout\([^,]+,\s*[^,]+,\s*Duration::from_millis\((\d+)\)"#)
            .ok()?
            .captures(line)
        {
            let timeout_ms: u64 = captures.get(1)?.as_str().parse().ok()?;
            if timeout_ms > 1000 {
                return Some(PerformanceIssue {
                    issue_type: IssueType::LongRequestTimeout,
                    line_number,
                    current_timeout_ms: timeout_ms,
                    suggested_timeout_ms: 500.min(timeout_ms / 2),
                    description: format!("request timeout of {}ms is too long", timeout_ms),
                });
            }
        }

        // Look for thread::sleep calls
        if let Some(captures) = Regex::new(r#"thread::sleep\(Duration::from_millis\((\d+)\)"#)
            .ok()?
            .captures(line)
        {
            let sleep_ms: u64 = captures.get(1)?.as_str().parse().ok()?;
            if sleep_ms > 100 {
                return Some(PerformanceIssue {
                    issue_type: IssueType::UnreasonableDelay,
                    line_number,
                    current_timeout_ms: sleep_ms,
                    suggested_timeout_ms: 50.min(sleep_ms / 2),
                    description: format!("Sleep of {}ms is unnecessarily long", sleep_ms),
                });
            }
        }

        None
    }

    /// Generate optimization report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("## Performance Analysis: {}\n\n", self.test_file));
        report.push_str(&format!("**Total Estimated Time**: {}ms ({:.1}s)\n", 
                                self.total_estimated_time_ms, 
                                self.total_estimated_time_ms as f64 / 1000.0));
        report.push_str(&format!("**Issues Found**: {}\n\n", self.issues_found.len()));

        if self.issues_found.is_empty() {
            report.push_str("✅ No performance issues detected!\n");
            return report;
        }

        report.push_str("### Issues Detected:\n\n");
        
        for issue in &self.issues_found {
            report.push_str(&format!(
                "**Line {}**: {}\n- Current: {}ms → Suggested: {}ms\n- Savings: {}ms\n\n",
                issue.line_number,
                issue.description,
                issue.current_timeout_ms,
                issue.suggested_timeout_ms,
                issue.current_timeout_ms - issue.suggested_timeout_ms
            ));
        }

        let total_savings: u64 = self.issues_found.iter()
            .map(|i| i.current_timeout_ms - i.suggested_timeout_ms)
            .sum();

        report.push_str(&format!(
            "### Summary:\n- **Total Potential Savings**: {}ms ({:.1}s)\n- **Optimized Runtime**: {}ms ({:.1}s)\n",
            total_savings,
            total_savings as f64 / 1000.0,
            self.total_estimated_time_ms - total_savings,
            (self.total_estimated_time_ms - total_savings) as f64 / 1000.0
        ));

        report
    }

    /// Apply optimizations to the test file
    pub fn apply_optimizations<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let content = fs::read_to_string(&file_path)?;
        let mut optimized_content = content.clone();

        // Apply optimizations in reverse line order to avoid line number shifts
        let mut issues = self.issues_found.clone();
        issues.sort_by(|a, b| b.line_number.cmp(&a.line_number));

        for issue in issues {
            optimized_content = Self::apply_single_optimization(&optimized_content, &issue);
        }

        fs::write(&file_path, optimized_content)?;
        Ok(())
    }

    /// Apply a single optimization to the content
    fn apply_single_optimization(content: &str, issue: &PerformanceIssue) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if i + 1 == issue.line_number {
                let optimized_line = match issue.issue_type {
                    IssueType::LongWaitForSymbol => {
                        line.replace(
                            &format!("Duration::from_millis({})", issue.current_timeout_ms),
                            &format!("Duration::from_millis({})", issue.suggested_timeout_ms)
                        )
                    }
                    IssueType::LongWaitForIdle => {
                        line.replace(
                            &format!("Duration::from_millis({})", issue.current_timeout_ms),
                            &format!("Duration::from_millis({})", issue.suggested_timeout_ms)
                        )
                    }
                    IssueType::LongRequestTimeout => {
                        line.replace(
                            &format!("Duration::from_millis({})", issue.current_timeout_ms),
                            &format!("Duration::from_millis({})", issue.suggested_timeout_ms)
                        )
                    }
                    IssueType::UnreasonableDelay => {
                        line.replace(
                            &format!("Duration::from_millis({})", issue.current_timeout_ms),
                            &format!("Duration::from_millis({})", issue.suggested_timeout_ms)
                        )
                    }
                    IssueType::PollingLoop => line.to_string(), // More complex, skip for now
                };
                result.push(optimized_line);
            } else {
                result.push(line.to_string());
            }
        }

        result.join("\n")
    }
}

/// Run performance optimization on LSP test files
pub fn optimize_lsp_tests() -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    spinner.set_message("Scanning LSP test files for performance issues...");

    let lsp_tests_dir = Path::new("crates/perl-lsp/tests");
    if !lsp_tests_dir.exists() {
        return Err(eyre!("LSP tests directory not found: {}", lsp_tests_dir.display()));
    }

    let mut total_issues = 0;
    let mut total_savings_ms = 0u64;
    let mut optimizations = Vec::new();

    // Find all behavioral test files
    for entry in fs::read_dir(lsp_tests_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Focus on behavioral tests which are most likely to have performance issues
            if file_name.contains("behavioral") || 
               file_name.contains("integration") ||
               file_name.contains("e2e") {
                
                spinner.set_message(format!("Analyzing {}", file_name));
                
                match TestOptimization::analyze_file(&path) {
                    Ok(optimization) => {
                        if !optimization.issues_found.is_empty() {
                            total_issues += optimization.issues_found.len();
                            let savings: u64 = optimization.issues_found.iter()
                                .map(|i| i.current_timeout_ms - i.suggested_timeout_ms)
                                .sum();
                            total_savings_ms += savings;
                            optimizations.push((path, optimization));
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to analyze {}: {}", file_name, e);
                    }
                }
            }
        }
    }

    spinner.finish_with_message(format!("Analysis complete: {} files processed", optimizations.len()));

    if optimizations.is_empty() {
        println!("✅ No performance issues found in LSP test files!");
        return Ok(());
    }

    println!("\n📊 **Performance Analysis Summary**");
    println!("Files analyzed: {}", optimizations.len());
    println!("Total issues found: {}", total_issues);
    println!("Potential time savings: {}ms ({:.1}s)", total_savings_ms, total_savings_ms as f64 / 1000.0);

    // Generate reports
    println!("\n📋 **Detailed Reports**:\n");
    for (path, optimization) in &optimizations {
        println!("{}", optimization.generate_report());
        
        // Ask if user wants to apply optimizations
        println!("Apply optimizations to {}? (y/n)", path.file_name().unwrap().to_str().unwrap());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        
        if input.trim().to_lowercase().starts_with('y') {
            match optimization.apply_optimizations(path) {
                Ok(()) => println!("✅ Optimizations applied to {}", path.display()),
                Err(e) => eprintln!("❌ Failed to apply optimizations: {}", e),
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_wait_for_symbol_issue() {
        let line = r#"        .wait_for_symbol("My::Module", Some(workspace.uri("lib/My/Module.pm").as_str()), Duration::from_millis(3000))"#;
        let issue = TestOptimization::analyze_line(line, 95).unwrap();
        
        assert_eq!(issue.current_timeout_ms, 3000);
        assert!(issue.suggested_timeout_ms < 3000);
        assert!(matches!(issue.issue_type, IssueType::LongWaitForSymbol));
    }

    #[test]
    fn test_detect_wait_for_idle_issue() {
        let line = r#"    harness.wait_for_idle(Duration::from_millis(1000));"#;
        let issue = TestOptimization::analyze_line(line, 78).unwrap();
        
        assert_eq!(issue.current_timeout_ms, 1000);
        assert!(issue.suggested_timeout_ms < 1000);
        assert!(matches!(issue.issue_type, IssueType::LongWaitForIdle));
    }

    #[test]
    fn test_no_issue_for_reasonable_timeouts() {
        let line = r#"    harness.wait_for_idle(Duration::from_millis(200));"#;
        let issue = TestOptimization::analyze_line(line, 1);
        assert!(issue.is_none());
    }

    #[test]
    fn test_apply_optimization() {
        let content = r#"fn test() {
    harness.wait_for_idle(Duration::from_millis(1000));
    let result = something();
}"#;

        let issue = PerformanceIssue {
            issue_type: IssueType::LongWaitForIdle,
            line_number: 2,
            current_timeout_ms: 1000,
            suggested_timeout_ms: 333,
            description: "test".to_string(),
        };

        let optimized = TestOptimization::apply_single_optimization(content, &issue);
        assert!(optimized.contains("Duration::from_millis(333)"));
        assert!(!optimized.contains("Duration::from_millis(1000)"));
    }
}