//! Diagnostics provider for Perl code analysis
//!
//! This module provides syntax error detection, linting, and code quality checks.

use crate::ast::{Node, NodeKind};
use crate::error::ParseError;
use crate::symbol::{SymbolTable, SymbolExtractor, SymbolKind};

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// A diagnostic message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: (usize, usize),
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub message: String,
    pub related_information: Vec<RelatedInformation>,
    pub tags: Vec<DiagnosticTag>,
}

/// Related information for a diagnostic
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInformation {
    pub location: (usize, usize),
    pub message: String,
}

/// Tags for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
}

/// Diagnostics provider
pub struct DiagnosticsProvider {
    symbol_table: SymbolTable,
    _source: String,
}

impl DiagnosticsProvider {
    /// Create a new diagnostics provider
    pub fn new(ast: &Node, source: String) -> Self {
        let extractor = SymbolExtractor::new();
        let symbol_table = extractor.extract(ast);
        
        Self {
            symbol_table,
            _source: source,
        }
    }
    
    /// Get all diagnostics for the document
    pub fn get_diagnostics(&self, ast: &Node, parse_errors: &[ParseError]) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Convert parse errors to diagnostics
        for error in parse_errors {
            diagnostics.push(self.parse_error_to_diagnostic(error));
        }
        
        // Run various linting checks
        self.check_undefined_variables(ast, &mut diagnostics);
        self.check_unused_variables(&mut diagnostics);
        self.check_deprecated_syntax(ast, &mut diagnostics);
        self.check_strict_warnings(ast, &mut diagnostics);
        self.check_common_mistakes(ast, &mut diagnostics);
        
        diagnostics
    }
    
    /// Convert a parse error to a diagnostic
    fn parse_error_to_diagnostic(&self, error: &ParseError) -> Diagnostic {
        let message = error.to_string();
        let location = match error {
            ParseError::UnexpectedToken { location, .. } => *location,
            ParseError::SyntaxError { location, .. } => *location,
            _ => 0,
        };
        
        Diagnostic {
            range: (location, location + 1),
            severity: DiagnosticSeverity::Error,
            code: Some("syntax-error".to_string()),
            message,
            related_information: Vec::new(),
            tags: Vec::new(),
        }
    }
    
    /// Check for undefined variables
    fn check_undefined_variables(&self, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        self.walk_node(node, &mut |n| {
            if let NodeKind::Variable { sigil, name } = &n.kind {
                let var_name = format!("{}{}", sigil, name);
                
                // Skip special variables
                if is_special_variable(&var_name) {
                    return;
                }
                
                // Check if variable is defined in any scope
                let kind = match sigil.as_str() {
                    "$" => SymbolKind::ScalarVariable,
                    "@" => SymbolKind::ArrayVariable,
                    "%" => SymbolKind::HashVariable,
                    _ => SymbolKind::ScalarVariable,
                };
                if self.symbol_table.find_symbol(name, 0, kind).is_empty() {
                    diagnostics.push(Diagnostic {
                        range: (n.location.start, n.location.end),
                        severity: DiagnosticSeverity::Warning,
                        code: Some("undefined-variable".to_string()),
                        message: format!("Variable '{}' is not defined", var_name),
                        related_information: Vec::new(),
                        tags: Vec::new(),
                    });
                }
            }
        });
    }
    
    /// Check for unused variables
    fn check_unused_variables(&self, diagnostics: &mut Vec<Diagnostic>) {
        // Check each symbol in the symbol table
        let symbols = self.symbol_table.symbols.values().flatten().collect::<Vec<_>>();
        for symbol in symbols {
            // Skip if it's a subroutine or package
            if matches!(symbol.kind, SymbolKind::Subroutine | SymbolKind::Package) {
                continue;
            }
            
            // Check if the symbol has any references
            if self.symbol_table.find_references(symbol).is_empty() {
                // Skip parameters and special variables
                if symbol.name.starts_with('_') || is_special_variable(&symbol.name) {
                    continue;
                }
                
                diagnostics.push(Diagnostic {
                    range: (symbol.location.start, symbol.location.end),
                    severity: DiagnosticSeverity::Warning,
                    code: Some("unused-variable".to_string()),
                    message: format!("Variable '{}' is declared but never used", symbol.name),
                    related_information: Vec::new(),
                    tags: vec![DiagnosticTag::Unnecessary],
                });
            }
        }
    }
    
    /// Check for deprecated syntax
    fn check_deprecated_syntax(&self, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        self.walk_node(node, &mut |n| {
            match &n.kind {
                // Check for deprecated 'defined @array' or 'defined %hash'
                NodeKind::FunctionCall { name, args } => {
                    if name == "defined" {
                        if let Some(arg) = args.first() {
                            if let NodeKind::Variable { sigil, .. } = &arg.kind {
                                if sigil == "@" || sigil == "%" {
                                    diagnostics.push(Diagnostic {
                                        range: (n.location.start, n.location.end),
                                        severity: DiagnosticSeverity::Warning,
                                        code: Some("deprecated-defined".to_string()),
                                        message: format!("Use of 'defined {}variable' is deprecated", sigil),
                                        related_information: vec![
                                            RelatedInformation {
                                                location: (arg.location.start, arg.location.end),
                                                message: format!("Use 'if ({}array)' instead", sigil),
                                            }
                                        ],
                                        tags: vec![DiagnosticTag::Deprecated],
                                    });
                                }
                            }
                        }
                    }
                }
                
                // Check for deprecated $[ variable
                NodeKind::Variable { sigil, name } => {
                    if sigil == "$" && name == "[" {
                        diagnostics.push(Diagnostic {
                            range: (n.location.start, n.location.start + 2),
                            severity: DiagnosticSeverity::Warning,
                            code: Some("deprecated-array-base".to_string()),
                            message: "Use of '$[' is deprecated and will be removed".to_string(),
                            related_information: Vec::new(),
                            tags: vec![DiagnosticTag::Deprecated],
                        });
                    }
                }
                
                _ => {}
            }
        });
    }
    
    /// Check for common strict/warnings issues
    fn check_strict_warnings(&self, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        let mut has_strict = false;
        let mut has_warnings = false;
        
        // Check if 'use strict' and 'use warnings' are present
        self.walk_node(node, &mut |n| {
            if let NodeKind::Use { module, args: _ } = &n.kind {
                if module == "strict" {
                    has_strict = true;
                } else if module == "warnings" {
                    has_warnings = true;
                }
            }
        });
        
        // Add diagnostics if missing
        if !has_strict {
            diagnostics.push(Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Information,
                code: Some("missing-strict".to_string()),
                message: "Consider adding 'use strict;' for better error checking".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            });
        }
        
        if !has_warnings {
            diagnostics.push(Diagnostic {
                range: (0, 0),
                severity: DiagnosticSeverity::Information,
                code: Some("missing-warnings".to_string()),
                message: "Consider adding 'use warnings;' for better error detection".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            });
        }
    }
    
    /// Check for common mistakes
    fn check_common_mistakes(&self, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        self.walk_node(node, &mut |n| {
            match &n.kind {
                // Check for assignment in condition
                NodeKind::If { condition, .. } | NodeKind::While { condition, .. } => {
                    self.check_assignment_in_condition(condition, diagnostics);
                }
                
                // Check for == or != with undef
                NodeKind::Binary { op, left, right } => {
                    if op == "==" || op == "!=" {
                        if self.might_be_undef(left) || self.might_be_undef(right) {
                            diagnostics.push(Diagnostic {
                                range: (n.location.start, n.location.end),
                                severity: DiagnosticSeverity::Warning,
                                code: Some("numeric-undef".to_string()),
                                message: format!("Using '{}' with potentially undefined value", op),
                                related_information: vec![
                                    RelatedInformation {
                                        location: (n.location.start, n.location.end),
                                        message: "Consider using 'defined' check or '//' operator".to_string(),
                                    }
                                ],
                                tags: Vec::new(),
                            });
                        }
                    }
                }
                
                _ => {}
            }
        });
    }
    
    /// Check for assignment in condition (common mistake)
    fn check_assignment_in_condition(&self, condition: &Node, diagnostics: &mut Vec<Diagnostic>) {
        if let NodeKind::Binary { op, .. } = &condition.kind {
            if op == "=" {
                diagnostics.push(Diagnostic {
                    range: (condition.location.start, condition.location.end),
                    severity: DiagnosticSeverity::Warning,
                    code: Some("assignment-in-condition".to_string()),
                    message: "Assignment in condition - did you mean '=='?".to_string(),
                    related_information: Vec::new(),
                    tags: Vec::new(),
                });
            }
        }
    }
    
    /// Check if a node might evaluate to undef
    fn might_be_undef(&self, node: &Node) -> bool {
        match &node.kind {
            NodeKind::Variable { name, .. } => {
                // If variable is not defined in scope, it might be undef
                self.symbol_table.find_symbol(name, 0, SymbolKind::ScalarVariable).is_empty()
            }
            NodeKind::Undef => true,
            _ => false,
        }
    }
    
    /// Walk the AST and call a function for each node
    fn walk_node<F>(&self, node: &Node, func: &mut F)
    where
        F: FnMut(&Node),
    {
        func(node);
        
        // Visit children based on node kind
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.walk_node(stmt, func);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.walk_node(stmt, func);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.walk_node(condition, func);
                self.walk_node(then_branch, func);
                for (cond, branch) in elsif_branches {
                    self.walk_node(cond, func);
                    self.walk_node(branch, func);
                }
                if let Some(branch) = else_branch {
                    self.walk_node(branch, func);
                }
            }
            NodeKind::While { condition, body, .. } => {
                self.walk_node(condition, func);
                self.walk_node(body, func);
            }
            NodeKind::Binary { left, right, .. } => {
                self.walk_node(left, func);
                self.walk_node(right, func);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.walk_node(arg, func);
                }
            }
            _ => {} // Other nodes don't have children or are handled differently
        }
    }
}

/// Check if a variable is a special Perl variable
fn is_special_variable(name: &str) -> bool {
    // Common special variables
    const SPECIAL_VARS: &[&str] = &[
        "$_", "@_", "%_", "$!", "$@", "$?", "$^", "$$", "$0", "$1", "$2", "$3", "$4", "$5",
        "$6", "$7", "$8", "$9", "$.", "$,", "$/", "$\\", "$\"", "$;", "$%", "$=", "$-",
        "$~", "$|", "$&", "$`", "$'", "$+", "@+", "%+", "$[", "$]", "$^A", "$^C", "$^D",
        "$^E", "$^F", "$^H", "$^I", "$^L", "$^M", "$^N", "$^O", "$^P", "$^R", "$^S",
        "$^T", "$^V", "$^W", "$^X", "%ENV", "@INC", "%INC", "@ARGV", "%SIG", "$ARGV",
        "STDIN", "STDOUT", "STDERR", "DATA", "ARGVOUT", "$a", "$b",
    ];
    
    SPECIAL_VARS.contains(&name) || 
        name.starts_with("$^") || 
        (name.len() == 2 && name.starts_with('$') && name.chars().nth(1).map_or(false, |c| !c.is_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    
    #[test]
    fn test_undefined_variable() {
        let source = r#"
            print $undefined_var;
        "#;
        
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();
        
        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[]);
        
        assert!(diagnostics.iter().any(|d| d.code == Some("undefined-variable".to_string())));
    }
    
    #[test]
    fn test_unused_variable() {
        let source = r#"
            my $unused = 42;
            print "Hello";
        "#;
        
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();
        
        let provider = DiagnosticsProvider::new(&ast, source.to_string());
        let diagnostics = provider.get_diagnostics(&ast, &[]);
        
        assert!(diagnostics.iter().any(|d| d.code == Some("unused-variable".to_string())));
    }
}