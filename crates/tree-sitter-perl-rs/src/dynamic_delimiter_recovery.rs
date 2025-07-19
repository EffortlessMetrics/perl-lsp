//! Dynamic heredoc delimiter recovery system
//!
//! This module attempts to resolve heredoc delimiters that are computed
//! at runtime, using various heuristics and recovery strategies.

use std::collections::HashMap;
use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct DynamicDelimiterRecovery {
    /// Known variable assignments in scope
    variable_values: HashMap<String, Vec<PossibleValue>>,
    /// Common delimiter patterns seen in real code
    common_delimiters: Vec<&'static str>,
    /// Recovery mode configuration
    recovery_mode: RecoveryMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryMode {
    /// Just mark as unparseable
    Conservative,
    /// Try common patterns and heuristics
    BestGuess,
    /// Prompt user for hint
    Interactive,
    /// Execute in sandbox (requires opt-in)
    Sandbox,
}

#[derive(Debug, Clone)]
pub struct PossibleValue {
    pub value: String,
    pub confidence: f32,
    pub source: ValueSource,
}

#[derive(Debug, Clone)]
pub enum ValueSource {
    Literal,           // Direct assignment
    Concatenation,     // String concatenation
    FunctionReturn,    // Return value of known function
    UserHint,          // User-provided hint
    Heuristic,         // Guessed from context
}

#[derive(Debug)]
pub struct DelimiterAnalysis {
    pub delimiter: Option<String>,
    pub confidence: f32,
    pub alternatives: Vec<String>,
    pub recovery_strategy: String,
    pub warnings: Vec<String>,
}

// Common patterns for delimiter variables
static DELIMITER_ASSIGNMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?m)^\s*(?:my\s+)?\$(\w+)\s*=\s*["']([^"']+)["']"#).unwrap()
});

static COMMON_DELIMITER_NAMES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec!["delimiter", "delim", "end", "eof", "marker", "tag", "term", "terminator"]
});

impl DynamicDelimiterRecovery {
    pub fn new(mode: RecoveryMode) -> Self {
        Self {
            variable_values: HashMap::new(),
            common_delimiters: vec![
                "EOF", "END", "EOT", "EOD", "DONE", "STOP",
                "HERE", "DATA", "TEXT", "SQL", "HTML", "XML",
                "PERL", "CODE", "SCRIPT", "TEMPLATE",
            ],
            recovery_mode: mode,
        }
    }
    
    /// Scan code for variable assignments that might be delimiters
    pub fn scan_for_assignments(&mut self, code: &str) {
        for cap in DELIMITER_ASSIGNMENT.captures_iter(code) {
            if let (Some(var), Some(val)) = (cap.get(1), cap.get(2)) {
                let var_name = var.as_str();
                let value = val.as_str();
                
                // Higher confidence if variable name suggests delimiter
                let confidence = if COMMON_DELIMITER_NAMES.iter()
                    .any(|&n| var_name.to_lowercase().contains(n)) {
                    0.8
                } else {
                    0.5
                };
                
                self.variable_values
                    .entry(var_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(PossibleValue {
                        value: value.to_string(),
                        confidence,
                        source: ValueSource::Literal,
                    });
            }
        }
    }
    
    /// Analyze a dynamic delimiter expression
    pub fn analyze_dynamic_delimiter(
        &self,
        expression: &str,
        context: &ParseContext,
    ) -> DelimiterAnalysis {
        let mut analysis = DelimiterAnalysis {
            delimiter: None,
            confidence: 0.0,
            alternatives: Vec::new(),
            recovery_strategy: String::new(),
            warnings: Vec::new(),
        };
        
        match self.recovery_mode {
            RecoveryMode::Conservative => {
                analysis.warnings.push(
                    "Dynamic delimiter cannot be resolved without code execution".to_string()
                );
                analysis.recovery_strategy = "Marked as unparseable".to_string();
            }
            
            RecoveryMode::BestGuess => {
                // Try various heuristics
                if let Some(delimiter) = self.try_resolve_variable(expression, context) {
                    analysis.delimiter = Some(delimiter.value.clone());
                    analysis.confidence = delimiter.confidence;
                    analysis.recovery_strategy = format!("Resolved via {:?}", delimiter.source);
                } else {
                    // Fall back to common patterns
                    analysis.alternatives = self.guess_common_delimiters(expression);
                    analysis.recovery_strategy = "Guessing from common patterns".to_string();
                    
                    if !analysis.alternatives.is_empty() {
                        analysis.delimiter = Some(analysis.alternatives[0].clone());
                        analysis.confidence = 0.3;
                    }
                }
            }
            
            RecoveryMode::Interactive => {
                analysis.warnings.push(
                    "User input required to resolve dynamic delimiter".to_string()
                );
                analysis.recovery_strategy = "Awaiting user hint".to_string();
                // In real implementation, would trigger UI prompt
            }
            
            RecoveryMode::Sandbox => {
                analysis.warnings.push(
                    "Sandbox execution required to resolve dynamic delimiter".to_string()
                );
                analysis.recovery_strategy = "Requires --enable-sandbox flag".to_string();
            }
        }
        
        // Add general warnings
        if expression.contains("$") {
            analysis.warnings.push(
                "Variable interpolation in delimiter makes static analysis unreliable".to_string()
            );
        }
        
        if expression.contains("(") || expression.contains("{") {
            analysis.warnings.push(
                "Complex expression in delimiter requires runtime evaluation".to_string()
            );
        }
        
        analysis
    }
    
    /// Try to resolve a variable to its value
    fn try_resolve_variable(&self, expr: &str, context: &ParseContext) -> Option<&PossibleValue> {
        // Simple case: just a variable like $delimiter
        if let Some(var_name) = expr.strip_prefix("$") {
            if let Some(values) = self.variable_values.get(var_name) {
                // Return highest confidence value
                return values.iter()
                    .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());
            }
        }
        
        // TODO: Handle more complex expressions like ${var} or $var . "END"
        
        None
    }
    
    /// Guess common delimiters based on context
    fn guess_common_delimiters(&self, expression: &str) -> Vec<String> {
        let mut guesses = Vec::new();
        
        // If variable name contains hints
        let lower = expression.to_lowercase();
        if lower.contains("sql") {
            guesses.push("SQL".to_string());
        }
        if lower.contains("end") || lower.contains("eof") {
            guesses.push("EOF".to_string());
            guesses.push("END".to_string());
        }
        
        // Add general common delimiters
        guesses.extend(
            self.common_delimiters[..5]
                .iter()
                .map(|&s| s.to_string())
        );
        
        guesses
    }
    
    /// Add a user-provided hint
    pub fn add_user_hint(&mut self, var_name: &str, value: &str) {
        self.variable_values
            .entry(var_name.to_string())
            .or_insert_with(Vec::new)
            .push(PossibleValue {
                value: value.to_string(),
                confidence: 0.9, // High confidence for user hints
                source: ValueSource::UserHint,
            });
    }
}

/// Context information for delimiter resolution
#[derive(Debug)]
pub struct ParseContext {
    pub current_package: Option<String>,
    pub imported_modules: Vec<String>,
    pub in_subroutine: Option<String>,
    pub file_type_hint: Option<FileType>,
}

#[derive(Debug)]
pub enum FileType {
    Script,
    Module,
    Test,
    Documentation,
}

/// Integration with the main parser
pub struct DynamicHeredocNode {
    pub expression: String,
    pub analysis: DelimiterAnalysis,
    pub recovery_attempted: bool,
}

impl DynamicHeredocNode {
    pub fn to_diagnostic_message(&self) -> String {
        format!(
            "Dynamic heredoc delimiter '{}' {}. {}",
            self.expression,
            if let Some(ref delim) = self.analysis.delimiter {
                format!("resolved to '{}' (confidence: {:.0}%)", delim, self.analysis.confidence * 100.0)
            } else {
                "could not be resolved".to_string()
            },
            self.analysis.recovery_strategy
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_literal_assignment_detection() {
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
Content here
EOF
"#;
        
        recovery.scan_for_assignments(code);
        assert!(recovery.variable_values.contains_key("delimiter"));
        
        let values = &recovery.variable_values["delimiter"];
        assert_eq!(values[0].value, "EOF");
        assert!(values[0].confidence > 0.7);
    }
    
    #[test]
    fn test_common_delimiter_guessing() {
        let recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let guesses = recovery.guess_common_delimiters("$end_marker");
        
        assert!(guesses.contains(&"EOF".to_string()));
        assert!(guesses.contains(&"END".to_string()));
    }
    
    #[test]
    fn test_analysis_modes() {
        let recovery = DynamicDelimiterRecovery::new(RecoveryMode::Conservative);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };
        
        let analysis = recovery.analyze_dynamic_delimiter("$foo", &context);
        assert!(analysis.delimiter.is_none());
        assert!(!analysis.warnings.is_empty());
    }
}