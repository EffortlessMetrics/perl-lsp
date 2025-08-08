//! Perl::Critic integration for code quality analysis
//!
//! This module provides integration with Perl::Critic for static code analysis
//! and policy enforcement in Perl code.

use std::collections::HashMap;
use std::process::Command;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::ast::Node;
use crate::position::{Position, Range};

/// Severity levels for Perl::Critic violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Gentle = 5,      // Cosmetic issues
    Stern = 4,       // Minor issues
    Harsh = 3,       // Important issues
    Cruel = 2,       // Serious issues
    Brutal = 1,      // Critical issues
}

impl Severity {
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
    
    pub fn to_diagnostic_severity(&self) -> crate::diagnostics::DiagnosticSeverity {
        match self {
            Self::Brutal | Self::Cruel => crate::diagnostics::DiagnosticSeverity::Error,
            Self::Harsh => crate::diagnostics::DiagnosticSeverity::Warning,
            Self::Stern | Self::Gentle => crate::diagnostics::DiagnosticSeverity::Information,
        }
    }
}

/// A Perl::Critic violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub policy: String,
    pub description: String,
    pub explanation: String,
    pub severity: Severity,
    pub range: Range,
    pub file: String,
}

/// Configuration for Perl::Critic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticConfig {
    /// Minimum severity level to report (1-5)
    pub severity: u8,
    /// Path to perlcriticrc file
    pub profile: Option<String>,
    /// Include/exclude specific policies
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    /// Theme to use
    pub theme: Option<String>,
    /// Enable verbose output
    pub verbose: bool,
    /// Color output
    pub color: bool,
}

impl Default for CriticConfig {
    fn default() -> Self {
        Self {
            severity: 3,  // Harsh and above
            profile: None,
            include: Vec::new(),
            exclude: Vec::new(),
            theme: None,
            verbose: false,
            color: false,
        }
    }
}

/// Perl::Critic analyzer
pub struct CriticAnalyzer {
    config: CriticConfig,
    cache: HashMap<String, Vec<Violation>>,
}

impl CriticAnalyzer {
    pub fn new(config: CriticConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
        }
    }
    
    /// Run Perl::Critic on a file
    pub fn analyze_file(&mut self, file_path: &Path) -> Result<Vec<Violation>, String> {
        let path_str = file_path.to_string_lossy().to_string();
        
        // Check cache
        if let Some(cached) = self.cache.get(&path_str) {
            return Ok(cached.clone());
        }
        
        // Build perlcritic command
        let mut cmd = Command::new("perlcritic");
        
        // Add severity
        cmd.arg(format!("--severity={}", self.config.severity));
        
        // Add profile if specified
        if let Some(ref profile) = self.config.profile {
            cmd.arg(format!("--profile={}", profile));
        }
        
        // Add theme if specified
        if let Some(ref theme) = self.config.theme {
            cmd.arg(format!("--theme={}", theme));
        }
        
        // Add includes
        for policy in &self.config.include {
            cmd.arg(format!("--include={}", policy));
        }
        
        // Add excludes
        for policy in &self.config.exclude {
            cmd.arg(format!("--exclude={}", policy));
        }
        
        // Use verbose format for parsing
        cmd.arg("--verbose=%f:%l:%c:%s:%p:%m\\n");
        
        // Add file path
        cmd.arg(file_path);
        
        // Execute command
        let output = cmd.output().map_err(|e| format!("Failed to run perlcritic: {}", e))?;
        
        // Parse output
        let violations = self.parse_output(&output.stdout, &path_str)?;
        
        // Cache results
        self.cache.insert(path_str, violations.clone());
        
        Ok(violations)
    }
    
    /// Parse perlcritic output
    fn parse_output(&self, output: &[u8], file_path: &str) -> Result<Vec<Violation>, String> {
        let output_str = String::from_utf8_lossy(output);
        let mut violations = Vec::new();
        
        for line in output_str.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            // Parse format: file:line:column:severity:policy:message
            let parts: Vec<&str> = line.splitn(6, ':').collect();
            if parts.len() != 6 {
                continue;
            }
            
            let line_num: u32 = parts[1].parse().unwrap_or(1);
            let column: u32 = parts[2].parse().unwrap_or(1);
            let severity: u8 = parts[3].parse().unwrap_or(3);
            let policy = parts[4].to_string();
            let message = parts[5].to_string();
            
            violations.push(Violation {
                policy: policy.clone(),
                description: message,
                explanation: self.get_policy_explanation(&policy),
                severity: Severity::from_number(severity),
                range: Range {
                    start: Position { byte: 0, line: line_num - 1, column: column - 1 },
                    end: Position { byte: 0, line: line_num - 1, column: column },
                },
                file: file_path.to_string(),
            });
        }
        
        Ok(violations)
    }
    
    /// Get explanation for a policy
    fn get_policy_explanation(&self, policy: &str) -> String {
        // In a real implementation, this would look up detailed explanations
        // For now, return a generic message
        format!("See perldoc Perl::Critic::Policy::{}", policy)
    }
    
    /// Clear cache for a file
    pub fn invalidate_cache(&mut self, file_path: &str) {
        self.cache.remove(file_path);
    }
    
    /// Convert violations to diagnostics
    pub fn to_diagnostics(&self, violations: &[Violation]) -> Vec<crate::diagnostics::Diagnostic> {
        violations.iter().map(|v| crate::diagnostics::Diagnostic {
            range: (v.range.start.byte, v.range.end.byte),
            severity: v.severity.to_diagnostic_severity(),
            code: Some(v.policy.clone()),
            message: v.description.clone(),
            related_information: vec![],
            tags: vec![],
        }).collect()
    }
    
    /// Get quick fix for a violation
    pub fn get_quick_fix(&self, violation: &Violation, _content: &str) -> Option<QuickFix> {
        match violation.policy.as_str() {
            "Variables::ProhibitUnusedVariables" => {
                Some(QuickFix {
                    title: "Remove unused variable".to_string(),
                    edit: TextEdit {
                        range: violation.range.clone(),
                        new_text: String::new(),
                    },
                })
            }
            "Subroutines::ProhibitUnusedPrivateSubroutines" => {
                Some(QuickFix {
                    title: "Remove unused subroutine".to_string(),
                    edit: TextEdit {
                        range: violation.range.clone(),
                        new_text: String::new(),
                    },
                })
            }
            "TestingAndDebugging::RequireUseStrict" => {
                Some(QuickFix {
                    title: "Add 'use strict'".to_string(),
                    edit: TextEdit {
                        range: Range {
                            start: Position { byte: 0, line: 0, column: 0 },
                            end: Position { byte: 0, line: 0, column: 0 },
                        },
                        new_text: "use strict;\n".to_string(),
                    },
                })
            }
            "TestingAndDebugging::RequireUseWarnings" => {
                Some(QuickFix {
                    title: "Add 'use warnings'".to_string(),
                    edit: TextEdit {
                        range: Range {
                            start: Position { byte: 0, line: 0, column: 0 },
                            end: Position { byte: 0, line: 0, column: 0 },
                        },
                        new_text: "use warnings;\n".to_string(),
                    },
                })
            }
            _ => None,
        }
    }
}

/// A quick fix for a violation
#[derive(Debug, Clone)]
pub struct QuickFix {
    pub title: String,
    pub edit: TextEdit,
}

/// A text edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

/// Built-in policy analyzer that works without external perlcritic
pub struct BuiltInAnalyzer {
    policies: Vec<Box<dyn Policy>>,
}

/// Trait for implementing policies
pub trait Policy: Send + Sync {
    fn name(&self) -> &str;
    fn severity(&self) -> Severity;
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
        Self {
            policies: vec![
                Box::new(RequireUseStrict),
                Box::new(RequireUseWarnings),
            ],
        }
    }
}

impl BuiltInAnalyzer {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_severity_levels() {
        assert_eq!(Severity::from_number(1), Severity::Brutal);
        assert_eq!(Severity::from_number(5), Severity::Gentle);
    }
    
    #[test]
    fn test_builtin_policies() {
        let analyzer = BuiltInAnalyzer::new();
        let ast = Node::new(
            crate::ast::NodeKind::Error { message: "test".to_string() },
            crate::ast::SourceLocation { start: 0, end: 10 }
        );
        
        // Test without strict/warnings
        let violations = analyzer.analyze(&ast, "print 'hello';\n");
        assert_eq!(violations.len(), 2);
        
        // Test with strict/warnings
        let violations = analyzer.analyze(&ast, "use strict;\nuse warnings;\nprint 'hello';\n");
        assert_eq!(violations.len(), 0);
    }
}