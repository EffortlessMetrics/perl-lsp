//! Perl::Critic integration for code quality analysis
//!
//! This module provides integration with Perl::Critic for static code analysis
//! and policy enforcement in Perl code.

pub use perl_lsp_critic::{CriticConfig, Severity, Violation};
use perl_lsp_critic::{build_perlcritic_args, parse_perlcritic_output};
use perl_parser_core::{
    Node,
    position::{Position, Range},
};
use perl_subprocess_runtime::SubprocessRuntime;
#[cfg(not(feature = "lsp-compat"))]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "lsp-compat")]
use lsp_types;

/// Perl::Critic analyzer
pub struct CriticAnalyzer {
    /// Configuration settings for the analyzer
    config: CriticConfig,
    /// Cache of violations keyed by file path
    cache: HashMap<String, Vec<Violation>>,
    /// Subprocess runtime for executing perlcritic
    runtime: Arc<dyn SubprocessRuntime>,
}

impl CriticAnalyzer {
    /// Creates a new analyzer with the given configuration and runtime.
    pub fn new(config: CriticConfig, runtime: Arc<dyn SubprocessRuntime>) -> Self {
        Self { config, cache: HashMap::new(), runtime }
    }

    /// Creates a new analyzer with the OS subprocess runtime (non-WASM only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_os_runtime(config: CriticConfig) -> Self {
        use perl_subprocess_runtime::OsSubprocessRuntime;
        Self::new(config, Arc::new(OsSubprocessRuntime::new()))
    }

    /// Run Perl::Critic on a file
    pub fn analyze_file(&mut self, file_path: &Path) -> Result<Vec<Violation>, String> {
        let path_str = file_path.to_string_lossy().to_string();

        // Check cache
        if let Some(cached) = self.cache.get(&path_str) {
            return Ok(cached.clone());
        }

        // Build argument list
        let args = build_perlcritic_args(&self.config, &path_str);

        // Convert to &str slice for the runtime
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        // Execute command via runtime
        let output =
            self.runtime.run_command("perlcritic", &args_refs, None).map_err(|e| e.message)?;

        // Parse output
        let violations = parse_perlcritic_output(&output.stdout, &path_str);

        // Cache results
        self.cache.insert(path_str, violations.clone());

        Ok(violations)
    }
    /// Clear cache for a file
    pub fn invalidate_cache(&mut self, file_path: &str) {
        self.cache.remove(file_path);
    }

    /// Convert violations to diagnostics
    #[cfg(feature = "lsp-compat")]
    pub fn to_diagnostics(&self, violations: &[Violation]) -> Vec<lsp_types::Diagnostic> {
        violations
            .iter()
            .map(|v| {
                let lsp_range = lsp_types::Range::new(
                    lsp_types::Position::new(v.range.start.line, v.range.start.column),
                    lsp_types::Position::new(v.range.end.line, v.range.end.column),
                );
                lsp_types::Diagnostic {
                    range: lsp_range,
                    severity: Some(v.severity.to_diagnostic_severity()),
                    code: Some(lsp_types::NumberOrString::String(v.policy.clone())),
                    source: Some("perlcritic".to_string()),
                    message: v.description.clone(),
                    related_information: None,
                    tags: None,
                    code_description: None,
                    data: None,
                }
            })
            .collect()
    }

    /// Convert violations to violation summaries (for non-LSP contexts)
    #[cfg(not(feature = "lsp-compat"))]
    pub fn to_violation_summaries(&self, violations: &[Violation]) -> Vec<ViolationSummary> {
        violations
            .iter()
            .map(|v| ViolationSummary {
                policy: v.policy.clone(),
                description: v.description.clone(),
                severity: v.severity as u8,
                line: v.range.start.line as usize,
            })
            .collect()
    }
}

/// Violation summary for non-LSP contexts
#[cfg(not(feature = "lsp-compat"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationSummary {
    /// Policy name
    pub policy: String,
    /// Description
    pub description: String,
    /// Severity level (1-5)
    pub severity: u8,
    /// Line number
    pub line: usize,
}

#[cfg(feature = "lsp-compat")]
impl CriticAnalyzer {
    /// Dummy impl to close the bracket
    fn _dummy(&self) {}

    /// Get quick fix for a violation
    pub fn get_quick_fix(&self, violation: &Violation, _content: &str) -> Option<QuickFix> {
        match violation.policy.as_str() {
            "Variables::ProhibitUnusedVariables" => Some(QuickFix {
                title: "Remove unused variable".to_string(),
                edit: TextEdit { range: violation.range, new_text: String::new() },
            }),
            "Subroutines::ProhibitUnusedPrivateSubroutines" => Some(QuickFix {
                title: "Remove unused subroutine".to_string(),
                edit: TextEdit { range: violation.range, new_text: String::new() },
            }),
            "TestingAndDebugging::RequireUseStrict" => Some(QuickFix {
                title: "Add 'use strict'".to_string(),
                edit: TextEdit {
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 0, line: 0, column: 0 },
                    },
                    new_text: "use strict;\n".to_string(),
                },
            }),
            "TestingAndDebugging::RequireUseWarnings" => Some(QuickFix {
                title: "Add 'use warnings'".to_string(),
                edit: TextEdit {
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 0, line: 0, column: 0 },
                    },
                    new_text: "use warnings;\n".to_string(),
                },
            }),
            _ => None,
        }
    }
}

/// A quick fix for a violation
#[derive(Debug, Clone)]
pub struct QuickFix {
    /// Human-readable title describing the fix action
    pub title: String,
    /// The text edit to apply as a fix
    pub edit: TextEdit,
}

/// A text edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// The range of text to replace
    pub range: Range,
    /// The replacement text (empty string for deletion)
    pub new_text: String,
}

/// Built-in policy analyzer that works without external perlcritic
pub struct BuiltInAnalyzer {
    /// Collection of registered policy implementations
    policies: Vec<Box<dyn Policy>>,
}

/// Trait for implementing policies
pub trait Policy: Send + Sync {
    /// Returns the fully qualified policy name.
    fn name(&self) -> &str;
    /// Returns the severity level for violations of this policy.
    fn severity(&self) -> Severity;
    /// Analyzes the AST and source content, returning any violations found.
    fn analyze(&self, ast: &Node, content: &str) -> Vec<Violation>;
}

// Example built-in policies

/// Require 'use strict'
struct RequireUseStrict;

impl Policy for RequireUseStrict {
    fn name(&self) -> &str {
        "TestingAndDebugging::RequireUseStrict"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        // Check if 'use strict' is present
        if !content.contains("use strict") {
            vec![Violation {
                policy: self.name().to_string(),
                description: "Code does not use strict".to_string(),
                explanation: "Always use strict to catch common mistakes".to_string(),
                severity: self.severity(),
                range: Range {
                    start: Position { byte: 0, line: 0, column: 0 },
                    end: Position { byte: 0, line: 0, column: 0 },
                },
                file: String::new(),
            }]
        } else {
            vec![]
        }
    }
}

/// Require 'use warnings'
struct RequireUseWarnings;

impl Policy for RequireUseWarnings {
    fn name(&self) -> &str {
        "TestingAndDebugging::RequireUseWarnings"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        if !content.contains("use warnings") {
            vec![Violation {
                policy: self.name().to_string(),
                description: "Code does not use warnings".to_string(),
                explanation: "Always use warnings to catch potential issues".to_string(),
                severity: self.severity(),
                range: Range {
                    start: Position { byte: 0, line: 0, column: 0 },
                    end: Position { byte: 0, line: 0, column: 0 },
                },
                file: String::new(),
            }]
        } else {
            vec![]
        }
    }
}

impl Default for BuiltInAnalyzer {
    fn default() -> Self {
        Self { policies: vec![Box::new(RequireUseStrict), Box::new(RequireUseWarnings)] }
    }
}

impl BuiltInAnalyzer {
    /// Creates a new analyzer with default built-in policies.
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze AST with built-in policies
    pub fn analyze(&self, ast: &Node, content: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for policy in &self.policies {
            violations.extend(policy.analyze(ast, content));
        }

        violations
    }

    /// Get quick fix for a violation
    pub fn get_quick_fix(&self, violation: &Violation, _content: &str) -> Option<QuickFix> {
        match violation.policy.as_str() {
            "TestingAndDebugging::RequireUseStrict" => Some(QuickFix {
                title: "Add 'use strict'".to_string(),
                edit: TextEdit {
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 0, line: 0, column: 0 },
                    },
                    new_text: "use strict;\n".to_string(),
                },
            }),
            "TestingAndDebugging::RequireUseWarnings" => Some(QuickFix {
                title: "Add 'use warnings'".to_string(),
                edit: TextEdit {
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 0, line: 0, column: 0 },
                    },
                    new_text: "use warnings;\n".to_string(),
                },
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_severity_levels() {
        assert_eq!(Severity::from_number(1), Severity::Brutal);
        assert_eq!(Severity::from_number(5), Severity::Gentle);
    }

    #[test]
    fn test_builtin_policies() {
        let analyzer = BuiltInAnalyzer::new();
        let ast = Node::new(
            perl_parser_core::NodeKind::Error {
                message: "test".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            perl_parser_core::SourceLocation { start: 0, end: 10 },
        );

        // Test without strict/warnings
        let violations = analyzer.analyze(&ast, "print 'hello';\n");
        assert_eq!(violations.len(), 2);

        // Test with strict/warnings
        let violations = analyzer.analyze(&ast, "use strict;\nuse warnings;\nprint 'hello';\n");
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_analyzer_with_mock_runtime() {
        use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        let mock_output =
            b"test.pl:5:1:3:TestingAndDebugging::RequireUseStrict\tCode does not use strict\n";
        runtime.add_response(MockResponse::success(mock_output.to_vec()));

        let config = CriticConfig::default();
        let mut analyzer = CriticAnalyzer::new(config, runtime.clone());

        let result = analyzer.analyze_file(Path::new("test.pl"));
        let violations = must(result);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "TestingAndDebugging::RequireUseStrict");
        assert_eq!(violations[0].range.start.line, 4); // 0-indexed

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, "perlcritic");
        assert!(invocations[0].args.contains(&"--severity=3".to_string()));
        // Ensure argument separator is used for security
        assert!(invocations[0].args.contains(&"--".to_string()));
        // Ensure the separator comes before the file path
        let sep_pos = must_some(invocations[0].args.iter().position(|a| a == "--"));
        let file_pos = must_some(invocations[0].args.iter().position(|a| a == "test.pl"));
        assert!(sep_pos < file_pos, "-- separator must come before file path");
    }

    #[test]
    fn test_analyzer_caching() {
        use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"".to_vec()));

        let config = CriticConfig::default();
        let mut analyzer = CriticAnalyzer::new(config, runtime.clone());

        // First call should invoke runtime
        let result1 = analyzer.analyze_file(Path::new("test.pl"));
        assert!(result1.is_ok());

        // Second call should use cache
        let result2 = analyzer.analyze_file(Path::new("test.pl"));
        assert!(result2.is_ok());

        // Only one invocation should have occurred
        assert_eq!(runtime.invocations().len(), 1);
    }

    #[test]
    fn test_analyzer_config_args() {
        use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"".to_vec()));

        let config = CriticConfig {
            severity: 1,
            profile: Some("/path/to/.perlcriticrc".to_string()),
            theme: Some("pbp".to_string()),
            include: vec!["RequireUseStrict".to_string()],
            exclude: vec!["ProhibitMagicNumbers".to_string()],
            ..Default::default()
        };
        let mut analyzer = CriticAnalyzer::new(config, runtime.clone());

        let _ = analyzer.analyze_file(Path::new("test.pl"));

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert!(invocations[0].args.contains(&"--severity=1".to_string()));
        assert!(invocations[0].args.contains(&"--profile=/path/to/.perlcriticrc".to_string()));
        assert!(invocations[0].args.contains(&"--theme=pbp".to_string()));
        assert!(invocations[0].args.contains(&"--include=RequireUseStrict".to_string()));
        assert!(invocations[0].args.contains(&"--exclude=ProhibitMagicNumbers".to_string()));
    }
}
