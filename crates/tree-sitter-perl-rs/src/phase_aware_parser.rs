//! Phase-aware parser for handling Perl's compilation phases
//!
//! This module tracks Perl's execution phases (BEGIN, CHECK, INIT, etc.)
//! to properly handle heredocs and other constructs that behave differently
//! depending on when they're evaluated.

use crate::anti_pattern_detector::{AntiPattern, Diagnostic, Location, Severity};
use crate::partial_parse_ast::{ExtendedAstNode, RuntimeContext};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub enum PerlPhase {
    TopLevel, // Normal parsing context
    Begin,    // BEGIN block - compile time
    Check,    // CHECK block - after compile
    Init,     // INIT block - before runtime
    Runtime,  // Normal runtime code
    End,      // END block - program termination
    Eval,     // Inside eval string/block
    Use,      // Inside 'use' statement
}

#[derive(Debug)]
pub struct PhaseContext {
    pub phase: PerlPhase,
    pub start_line: usize,
    pub variables_modified: Vec<String>,
    pub side_effects: Vec<String>,
}

#[derive(Debug)]
pub struct PhaseAwareParser {
    phase_stack: Vec<PhaseContext>,
    current_phase: PerlPhase,
    deferred_heredocs: Vec<DeferredHeredoc>,
    phase_variables: HashMap<String, Vec<PhaseAssignment>>,
}

#[derive(Debug, Clone)]
pub struct DeferredHeredoc {
    pub location: Location,
    pub delimiter: String,
    pub phase: PerlPhase,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct PhaseAssignment {
    pub variable: String,
    pub value: Option<String>,
    pub phase: PerlPhase,
    pub line: usize,
}

#[derive(Debug)]
pub enum PhaseAction {
    Parse,                                        // Normal parsing
    Defer { reason: String, severity: Severity }, // Defer to runtime
    PartialParse { warning: String },             // Parse with warnings
}

#[allow(clippy::unwrap_used)]
static PHASE_BLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*(BEGIN|CHECK|INIT|END)\s*\{").unwrap());

#[allow(clippy::unwrap_used)]
static EVAL_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\beval\s*["'{]"#).unwrap());

#[allow(clippy::unwrap_used)]
static USE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*use\s+").unwrap());

impl Default for PhaseAwareParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseAwareParser {
    pub fn new() -> Self {
        Self {
            phase_stack: Vec::new(),
            current_phase: PerlPhase::TopLevel,
            deferred_heredocs: Vec::new(),
            phase_variables: HashMap::new(),
        }
    }

    /// Analyze code and identify phase transitions
    pub fn analyze_phases(&mut self, code: &str) -> Vec<PhaseTransition> {
        let mut transitions = Vec::new();

        for (line_idx, line) in code.lines().enumerate() {
            let line_num = line_idx + 1;

            // Check for phase blocks
            if let Some(cap) = PHASE_BLOCK_PATTERN.captures(line)
                && let Some(phase_name) = cap.get(1)
            {
                let phase = match phase_name.as_str() {
                    "BEGIN" => PerlPhase::Begin,
                    "CHECK" => PerlPhase::Check,
                    "INIT" => PerlPhase::Init,
                    "END" => PerlPhase::End,
                    _ => continue,
                };

                transitions.push(PhaseTransition {
                    from: self.current_phase.clone(),
                    to: phase.clone(),
                    line: line_num,
                    reason: format!("{} block", phase_name.as_str()),
                });
            }

            // Check for eval
            if EVAL_PATTERN.is_match(line) {
                transitions.push(PhaseTransition {
                    from: self.current_phase.clone(),
                    to: PerlPhase::Eval,
                    line: line_num,
                    reason: "eval expression".to_string(),
                });
            }

            // Check for use statements
            if USE_PATTERN.is_match(line) {
                transitions.push(PhaseTransition {
                    from: self.current_phase.clone(),
                    to: PerlPhase::Use,
                    line: line_num,
                    reason: "use statement".to_string(),
                });
            }
        }

        transitions
    }

    /// Enter a new phase
    pub fn enter_phase(&mut self, phase: PerlPhase, line: usize) {
        let context = PhaseContext {
            phase: self.current_phase.clone(),
            start_line: line,
            variables_modified: Vec::new(),
            side_effects: Vec::new(),
        };

        self.phase_stack.push(context);
        self.current_phase = phase;
    }

    /// Exit current phase
    pub fn exit_phase(&mut self) {
        if let Some(context) = self.phase_stack.pop() {
            self.current_phase = context.phase;

            // Track any variables modified in this phase
            for var in context.variables_modified {
                self.phase_variables.entry(var.clone()).or_default().push(PhaseAssignment {
                    variable: var,
                    value: None, // Would need data flow analysis
                    phase: self.current_phase.clone(),
                    line: context.start_line,
                });
            }
        }
    }

    /// Determine how to handle a heredoc in current phase
    pub fn handle_phase_heredoc(&mut self, delimiter: &str, location: Location) -> PhaseAction {
        match self.current_phase {
            PerlPhase::Begin => {
                // BEGIN blocks are most problematic
                self.deferred_heredocs.push(DeferredHeredoc {
                    location: location.clone(),
                    delimiter: delimiter.to_string(),
                    phase: PerlPhase::Begin,
                    reason: "BEGIN-time heredoc may modify parsing state".to_string(),
                });

                PhaseAction::Defer {
                    reason: "Heredoc in BEGIN block - compile-time side effects possible"
                        .to_string(),
                    severity: Severity::Warning,
                }
            }

            PerlPhase::Check => {
                // CHECK phase - executes after compilation, before main program
                PhaseAction::PartialParse {
                    warning:
                        "Heredoc in CHECK block - executes after compilation, before main program"
                            .to_string(),
                }
            }

            PerlPhase::Init => {
                // INIT phase - executes before main program starts
                PhaseAction::PartialParse {
                    warning: "Heredoc in INIT block - executes before main program starts"
                        .to_string(),
                }
            }

            PerlPhase::Eval => {
                // Eval heredocs require runtime evaluation
                self.deferred_heredocs.push(DeferredHeredoc {
                    location: location.clone(),
                    delimiter: delimiter.to_string(),
                    phase: PerlPhase::Eval,
                    reason: "Heredoc in eval requires runtime evaluation".to_string(),
                });

                PhaseAction::Defer {
                    reason: "Heredoc in eval - dynamic evaluation required".to_string(),
                    severity: Severity::Warning,
                }
            }

            PerlPhase::Use => {
                // use statements with heredocs are rare but possible
                PhaseAction::PartialParse {
                    warning: "Heredoc in use statement - module loading may be affected"
                        .to_string(),
                }
            }

            PerlPhase::End => {
                // END phase - executes at program termination
                PhaseAction::PartialParse {
                    warning: "Heredoc in END block - content executes at program termination"
                        .to_string(),
                }
            }

            PerlPhase::TopLevel | PerlPhase::Runtime => {
                // Normal parsing is fine
                PhaseAction::Parse
            }
        }
    }

    /// Generate diagnostics for phase-related issues
    pub fn generate_phase_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for heredoc in &self.deferred_heredocs {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                pattern: AntiPattern::BeginTimeHeredoc {
                    location: heredoc.location.clone(),
                    side_effects: vec!["Phase-dependent parsing".to_string()],
                    heredoc_content: heredoc.delimiter.clone(),
                },
                message: format!("Heredoc in {} block", self.phase_name()),
                explanation: heredoc.reason.clone(),
                suggested_fix: match heredoc.phase {
                    PerlPhase::Begin => Some("Move heredoc to INIT block or runtime".to_string()),
                    PerlPhase::Eval => Some("Consider using a regular string instead".to_string()),
                    PerlPhase::Check => {
                        Some("Move to INIT block or runtime if possible".to_string())
                    }
                    PerlPhase::Init => Some(
                        "Consider moving to runtime if initialization order allows".to_string(),
                    ),
                    PerlPhase::End => Some(
                        "Ensure cleanup logic doesn't depend on parse-time evaluation".to_string(),
                    ),
                    PerlPhase::Use => {
                        Some("Avoid heredocs in use statements; use string literals".to_string())
                    }
                    PerlPhase::TopLevel | PerlPhase::Runtime => None,
                },
                references: vec!["perldoc perlmod".to_string()],
            });
        }

        diagnostics
    }

    /// Check if a variable might have phase-dependent values
    pub fn is_phase_dependent(&self, var_name: &str) -> Option<PhaseWarning> {
        if let Some(assignments) = self.phase_variables.get(var_name) {
            let phases: Vec<_> =
                assignments.iter().map(|a| self.phase_name_for(&a.phase)).collect();

            if phases.len() > 1 || assignments.iter().any(|a| matches!(a.phase, PerlPhase::Begin)) {
                return Some(PhaseWarning {
                    variable: var_name.to_string(),
                    phases,
                    message: format!("Variable '{}' modified in multiple phases", var_name),
                });
            }
        }

        None
    }

    fn phase_name(&self) -> &str {
        self.phase_name_for(&self.current_phase)
    }

    fn phase_name_for(&self, phase: &PerlPhase) -> &'static str {
        match phase {
            PerlPhase::TopLevel => "top-level",
            PerlPhase::Begin => "BEGIN",
            PerlPhase::Check => "CHECK",
            PerlPhase::Init => "INIT",
            PerlPhase::Runtime => "runtime",
            PerlPhase::End => "END",
            PerlPhase::Eval => "eval",
            PerlPhase::Use => "use",
        }
    }
}

#[derive(Debug)]
pub struct PhaseTransition {
    pub from: PerlPhase,
    pub to: PerlPhase,
    pub line: usize,
    pub reason: String,
}

#[derive(Debug)]
pub struct PhaseWarning {
    pub variable: String,
    pub phases: Vec<&'static str>,
    pub message: String,
}

/// Integration with ExtendedAstNode
impl PhaseAwareParser {
    pub fn create_phase_node(&self, heredoc: &DeferredHeredoc) -> ExtendedAstNode {
        ExtendedAstNode::RuntimeDependentParse {
            construct_type: format!("{}_heredoc", self.phase_name()),
            static_parts: vec![],
            dynamic_parts: vec![crate::partial_parse_ast::DynamicPart {
                expression: heredoc.delimiter.clone(),
                context: match self.current_phase {
                    PerlPhase::Begin => RuntimeContext::BeginBlock,
                    PerlPhase::Eval => RuntimeContext::EvalString,
                    _ => RuntimeContext::BeginBlock, // Default
                },
                fallback_parse: None,
            }],
            diagnostics: self.generate_phase_diagnostics(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_detection() {
        let mut parser = PhaseAwareParser::new();
        let code = r#"
BEGIN {
    my $config = <<'END';
    setting = value
END
}

my $runtime = <<'EOF';
Normal heredoc
EOF

END {
    print <<'DONE';
    Cleanup
DONE
}
"#;

        let transitions = parser.analyze_phases(code);
        assert_eq!(transitions.len(), 2); // BEGIN and END
        assert!(matches!(transitions[0].to, PerlPhase::Begin));
        assert!(matches!(transitions[1].to, PerlPhase::End));
    }

    #[test]
    fn test_phase_heredoc_handling() {
        let mut parser = PhaseAwareParser::new();
        parser.enter_phase(PerlPhase::Begin, 1);

        let action =
            parser.handle_phase_heredoc("END", Location { line: 2, column: 5, offset: 20 });

        match action {
            PhaseAction::Defer { reason, severity } => {
                assert!(reason.contains("BEGIN"));
                assert_eq!(severity, Severity::Warning);
            }
            _ => panic!("Expected Defer action"),
        }
    }

    #[test]
    fn test_end_phase_heredoc_warning() {
        let mut parser = PhaseAwareParser::new();
        parser.enter_phase(PerlPhase::End, 1);

        let action =
            parser.handle_phase_heredoc("CLEANUP", Location { line: 2, column: 5, offset: 20 });

        match action {
            PhaseAction::PartialParse { warning } => {
                assert!(warning.contains("END block"));
                assert!(warning.contains("program termination"));
            }
            _ => panic!("Expected PartialParse action for END phase"),
        }
    }

    #[test]
    fn test_eval_phase_heredoc_deferred() {
        let mut parser = PhaseAwareParser::new();
        parser.enter_phase(PerlPhase::Eval, 1);

        let action =
            parser.handle_phase_heredoc("DYNAMIC", Location { line: 2, column: 5, offset: 20 });

        match action {
            PhaseAction::Defer { reason, severity } => {
                assert!(reason.contains("eval"));
                assert_eq!(severity, Severity::Warning);
            }
            _ => panic!("Expected Defer action for eval phase"),
        }

        // Verify eval heredoc was added to deferred list
        assert_eq!(parser.deferred_heredocs.len(), 1);
        assert_eq!(parser.deferred_heredocs[0].phase, PerlPhase::Eval);
        assert_eq!(parser.deferred_heredocs[0].delimiter, "DYNAMIC");

        // Generate diagnostics and verify eval-specific suggestion
        let diagnostics = parser.generate_phase_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suggested_fix.as_ref().unwrap().contains("regular string"));
    }
}
