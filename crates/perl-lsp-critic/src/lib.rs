//! Shared Perl::Critic domain models and parsers.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_parser_core::position::{Position, Range};
use serde::{Deserialize, Serialize};

#[cfg(feature = "lsp-compat")]
use lsp_types::DiagnosticSeverity;

/// Severity levels for Perl::Critic violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Cosmetic issues (severity 5)
    Gentle = 5,
    /// Minor issues (severity 4)
    Stern = 4,
    /// Important issues (severity 3)
    Harsh = 3,
    /// Serious issues (severity 2)
    Cruel = 2,
    /// Critical issues (severity 1)
    Brutal = 1,
}

impl Severity {
    /// Converts a numeric severity (1-5) to a `Severity` variant.
    pub fn from_number(n: u8) -> Self {
        match n {
            1 => Self::Brutal,
            2 => Self::Cruel,
            3 => Self::Harsh,
            4 => Self::Stern,
            5 => Self::Gentle,
            _ => Self::Harsh,
        }
    }

    /// Converts this severity to an LSP diagnostic severity.
    #[cfg(feature = "lsp-compat")]
    pub fn to_diagnostic_severity(&self) -> DiagnosticSeverity {
        match self {
            Self::Brutal | Self::Cruel => DiagnosticSeverity::ERROR,
            Self::Harsh => DiagnosticSeverity::WARNING,
            Self::Stern | Self::Gentle => DiagnosticSeverity::INFORMATION,
        }
    }

    /// Converts this severity to a numeric severity level.
    #[cfg(not(feature = "lsp-compat"))]
    pub fn to_severity_level(&self) -> u8 {
        *self as u8
    }
}

/// A Perl::Critic violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// The policy name that was violated.
    pub policy: String,
    /// A brief description of the violation.
    pub description: String,
    /// A detailed explanation of why this policy exists.
    pub explanation: String,
    /// The severity level of this violation.
    pub severity: Severity,
    /// The source location where the violation occurred.
    pub range: Range,
    /// The file path where the violation was found.
    pub file: String,
}

/// Configuration for Perl::Critic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticConfig {
    /// Minimum severity level to report (1-5).
    pub severity: u8,
    /// Path to perlcriticrc file.
    pub profile: Option<String>,
    /// Policies to explicitly include in analysis.
    pub include: Vec<String>,
    /// Policies to explicitly exclude from analysis.
    pub exclude: Vec<String>,
    /// Theme to use.
    pub theme: Option<String>,
    /// Enable verbose output.
    pub verbose: bool,
    /// Color output.
    pub color: bool,
}

impl Default for CriticConfig {
    fn default() -> Self {
        Self {
            severity: 3,
            profile: None,
            include: Vec::new(),
            exclude: Vec::new(),
            theme: None,
            verbose: false,
            color: false,
        }
    }
}

/// Builds perlcritic CLI arguments including a parse-stable verbose format.
pub fn build_perlcritic_args(config: &CriticConfig, file_path: &str) -> Vec<String> {
    let mut args = Vec::new();
    args.push(format!("--severity={}", config.severity));

    if let Some(profile) = &config.profile {
        args.push(format!("--profile={profile}"));
    }
    if let Some(theme) = &config.theme {
        args.push(format!("--theme={theme}"));
    }

    for policy in &config.include {
        args.push(format!("--include={policy}"));
    }
    for policy in &config.exclude {
        args.push(format!("--exclude={policy}"));
    }

    // Tab separator prevents ambiguous parsing when policy/message contain ':'.
    args.push("--verbose=%f:%l:%c:%s:%p\t%m\\n".to_string());
    // Prevent argument injection from filenames beginning with '-'.
    args.push("--".to_string());
    args.push(file_path.to_string());
    args
}

/// Parse perlcritic output into violations.
pub fn parse_perlcritic_output(output: &[u8], file_path: &str) -> Vec<Violation> {
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| parse_violation_line(line, file_path))
        .collect()
}

/// Returns a generic policy explanation text.
pub fn policy_explanation(policy: &str) -> String {
    format!("See perldoc Perl::Critic::Policy::{policy}")
}

fn parse_violation_line(line: &str, file_path: &str) -> Option<Violation> {
    if line.trim().is_empty() {
        return None;
    }

    let (left, message) = line.split_once('\t')?;
    let parts: Vec<&str> = left.splitn(5, ':').collect();
    if parts.len() != 5 {
        return None;
    }

    let line_num = parts[1].parse::<u32>().unwrap_or(1);
    let column = parts[2].parse::<u32>().unwrap_or(1);
    let severity = parts[3].parse::<u8>().unwrap_or(3);
    let policy = parts[4].to_string();

    Some(Violation {
        policy: policy.clone(),
        description: message.to_string(),
        explanation: policy_explanation(&policy),
        severity: Severity::from_number(severity),
        range: Range {
            start: Position { byte: 0, line: line_num - 1, column: column - 1 },
            end: Position { byte: 0, line: line_num - 1, column },
        },
        file: file_path.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_policy_with_namespace_and_colon_message() {
        let line = b"test.pl:5:1:3:TestingAndDebugging::RequireUseStrict\tMissing: use strict\n";
        let violations = parse_perlcritic_output(line, "test.pl");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "TestingAndDebugging::RequireUseStrict");
        assert_eq!(violations[0].description, "Missing: use strict");
    }

    #[test]
    fn args_include_safe_separator_and_verbose_format() {
        let args = build_perlcritic_args(&CriticConfig::default(), "test.pl");
        assert!(args.iter().any(|a| a == "--verbose=%f:%l:%c:%s:%p\t%m\\n"));
        let sep_pos = args.iter().position(|a| a == "--").unwrap_or(usize::MAX);
        let file_pos = args.iter().position(|a| a == "test.pl").unwrap_or(0);
        assert!(sep_pos < file_pos);
    }
}
