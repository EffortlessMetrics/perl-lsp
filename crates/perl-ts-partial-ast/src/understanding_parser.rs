//! Code understanding parser that handles anti-patterns
//!
//! This parser extends the pure Rust parser to detect, analyze, and provide
//! insights about problematic Perl constructs, particularly heredoc edge cases.

use perl_ts_heredoc_analysis::anti_pattern_detector::{AntiPattern, AntiPatternDetector, Diagnostic};
use crate::partial_parse_ast::{ExtendedAstBuilder, ExtendedAstNode, RecoveryState};
use perl_parser_pest::pure_rust_parser::{AstNode, PerlParser, PureRustPerlParser, Rule};
use pest::Parser;
use std::sync::Arc;

pub struct UnderstandingParser {
    base_parser: PureRustPerlParser,
    anti_pattern_detector: AntiPatternDetector,
    recovery_enabled: bool,
}

impl Default for UnderstandingParser {
    fn default() -> Self {
        Self::new()
    }
}

impl UnderstandingParser {
    pub fn new() -> Self {
        Self {
            base_parser: PureRustPerlParser::new(),
            anti_pattern_detector: AntiPatternDetector::new(),
            recovery_enabled: true,
        }
    }

    /// Parse code with full anti-pattern detection and recovery
    pub fn parse_with_understanding(&mut self, code: &str) -> Result<ParseResult, String> {
        // First, detect all anti-patterns
        let diagnostics = self.anti_pattern_detector.detect_all(code);

        // Try normal parsing
        match PerlParser::parse(Rule::program, code) {
            Ok(pairs) => {
                // Normal parse succeeded, but we might still have warnings
                let ast = self.build_extended_ast(pairs, &diagnostics, code);
                Ok(ParseResult { ast, diagnostics, parse_coverage: 100.0, recovery_points: vec![] })
            }
            Err(parse_error) => {
                // Parse failed, attempt recovery
                if self.recovery_enabled {
                    self.parse_with_recovery(code, parse_error, diagnostics)
                } else {
                    Err(format!("Parse error: {}", parse_error))
                }
            }
        }
    }

    /// Build extended AST with anti-pattern annotations
    fn build_extended_ast(
        &mut self,
        pairs: pest::iterators::Pairs<Rule>,
        diagnostics: &[Diagnostic],
        code: &str,
    ) -> ExtendedAstNode {
        let mut builder = ExtendedAstBuilder::new();

        // Add relevant diagnostics to builder
        for diag in diagnostics {
            builder.add_diagnostic(diag.clone());
        }

        // Build normal AST
        if let Ok(ast) = self.base_parser.build_ast(pairs) {
            builder.build_normal(ast)
        } else {
            ExtendedAstNode::Unparseable {
                pattern: AntiPattern::DynamicHeredocDelimiter {
                    location: perl_ts_heredoc_analysis::anti_pattern_detector::Location {
                        line: 0,
                        column: 0,
                        offset: 0,
                    },
                    expression: "unknown".to_string(),
                },
                raw_text: Arc::from(code),
                reason: "Failed to build AST".to_string(),
                diagnostics: diagnostics.to_vec(),
                recovery_point: 0,
            }
        }
    }

    /// Attempt to parse with recovery from anti-patterns
    fn parse_with_recovery(
        &self,
        code: &str,
        original_error: pest::error::Error<Rule>,
        mut diagnostics: Vec<Diagnostic>,
    ) -> Result<ParseResult, String> {
        let mut recovery_state = RecoveryState {
            last_good_position: 0,
            depth: 0,
            active_anti_patterns: vec![],
            deferred_heredocs: vec![],
        };

        let mut parsed_fragments = vec![];
        let mut recovery_points = vec![];
        let mut current_pos = 0;

        // Try to parse in chunks, recovering from errors
        while current_pos < code.len() {
            let chunk = &code[current_pos..];

            // Look for anti-patterns in the current chunk
            let chunk_patterns = self.anti_pattern_detector.detect_all(chunk);

            if let Some(first_pattern) = chunk_patterns.first() {
                // Found an anti-pattern, parse up to it
                let pattern_offset = match &first_pattern.pattern {
                    AntiPattern::FormatHeredoc { location, .. }
                    | AntiPattern::BeginTimeHeredoc { location, .. }
                    | AntiPattern::DynamicHeredocDelimiter { location, .. }
                    | AntiPattern::SourceFilterHeredoc { location, .. }
                    | AntiPattern::RegexCodeBlockHeredoc { location, .. }
                    | AntiPattern::EvalStringHeredoc { location, .. }
                    | AntiPattern::TiedHandleHeredoc { location, .. } => location.offset,
                };

                if pattern_offset > 0 {
                    // Parse the clean part before the anti-pattern
                    let clean_chunk = &chunk[..pattern_offset];
                    if let Ok(pairs) = PerlParser::parse(Rule::statement, clean_chunk)
                        && let Ok(ast) = PureRustPerlParser::new().build_ast(pairs)
                    {
                        parsed_fragments.push(ExtendedAstNode::Normal(ast));
                        recovery_state.last_good_position = current_pos + pattern_offset;
                    }
                }

                // Handle the anti-pattern
                let (handled_node, skip_length) = self.handle_anti_pattern(
                    &first_pattern.pattern,
                    &code[current_pos + pattern_offset..],
                    &mut recovery_state,
                );

                parsed_fragments.push(handled_node);
                recovery_points.push(current_pos + pattern_offset);
                current_pos += pattern_offset + skip_length;

                // Add the diagnostic
                diagnostics.push(first_pattern.clone());
            } else {
                // No more anti-patterns, try to parse the rest
                match PerlParser::parse(Rule::program, chunk) {
                    Ok(pairs) => {
                        if let Ok(ast) = PureRustPerlParser::new().build_ast(pairs) {
                            parsed_fragments.push(ExtendedAstNode::Normal(ast));
                        }
                        break;
                    }
                    Err(_) => {
                        // Can't parse the rest, mark as unparseable
                        parsed_fragments.push(ExtendedAstNode::Unparseable {
                            pattern: AntiPattern::DynamicHeredocDelimiter {
                                location: perl_ts_heredoc_analysis::anti_pattern_detector::Location {
                                    line: code[..current_pos].lines().count(),
                                    column: current_pos
                                        - code[..current_pos].rfind('\n').unwrap_or(0),
                                    offset: current_pos,
                                },
                                expression: "parse_error".to_string(),
                            },
                            raw_text: Arc::from(chunk),
                            reason: format!("Parse error: {}", original_error),
                            diagnostics: vec![],
                            recovery_point: current_pos,
                        });
                        break;
                    }
                }
            }
        }

        // Calculate parse coverage
        let total_length = code.len();
        let parsed_length = recovery_state.last_good_position;
        let parse_coverage = (parsed_length as f64 / total_length as f64) * 100.0;

        // Build final AST
        let final_ast = if parsed_fragments.len() == 1 {
            parsed_fragments
                .into_iter()
                .next()
                .unwrap_or(ExtendedAstNode::Normal(AstNode::EmptyExpression))
        } else {
            ExtendedAstNode::PartialParse {
                pattern: AntiPattern::DynamicHeredocDelimiter {
                    location: perl_ts_heredoc_analysis::anti_pattern_detector::Location {
                        line: 0,
                        column: 0,
                        offset: 0,
                    },
                    expression: "multiple_fragments".to_string(),
                },
                raw_text: Arc::from(code),
                parsed_fragments,
                diagnostics: vec![],
            }
        };

        Ok(ParseResult { ast: final_ast, diagnostics, parse_coverage, recovery_points })
    }

    /// Handle a specific anti-pattern and return a node representing it
    fn handle_anti_pattern(
        &self,
        pattern: &AntiPattern,
        code: &str,
        _recovery_state: &mut RecoveryState,
    ) -> (ExtendedAstNode, usize) {
        match pattern {
            AntiPattern::FormatHeredoc { format_name, heredoc_delimiter, .. } => {
                // Find the end of the format
                let end_pos = code.find("\n.").unwrap_or(code.len());
                let format_text = &code[..end_pos];

                let node = ExtendedAstNode::PartialParse {
                    pattern: pattern.clone(),
                    raw_text: Arc::from(format_text),
                    parsed_fragments: vec![
                        ExtendedAstNode::Normal(AstNode::Identifier(Arc::from(
                            format_name.clone(),
                        ))),
                        ExtendedAstNode::Normal(AstNode::String(Arc::from(
                            heredoc_delimiter.clone(),
                        ))),
                    ],
                    diagnostics: vec![],
                };

                (node, end_pos + 2) // +2 for "\n."
            }

            AntiPattern::BeginTimeHeredoc { heredoc_content, .. } => {
                // Create a runtime-dependent node
                let node = ExtendedAstNode::RuntimeDependentParse {
                    construct_type: "BEGIN_heredoc".to_string(),
                    static_parts: vec![],
                    dynamic_parts: vec![crate::partial_parse_ast::DynamicPart {
                        expression: heredoc_content.clone(),
                        context: crate::partial_parse_ast::RuntimeContext::BeginBlock,
                        fallback_parse: None,
                    }],
                    diagnostics: vec![],
                };

                // Find the end of the BEGIN block
                let end_pos = code.find('}').unwrap_or(code.len()) + 1;
                (node, end_pos)
            }

            AntiPattern::DynamicHeredocDelimiter { expression, .. } => {
                // Mark as unparseable but try to find where it ends
                let delimiter_guess = expression
                    .trim_start_matches("<<")
                    .trim_start_matches(['$', '`'])
                    .split(['{', '}', '(', ')'])
                    .next()
                    .unwrap_or("EOF");

                let end_pattern = format!("\n{}\n", delimiter_guess);
                let end_pos = code.find(&end_pattern).unwrap_or(code.len());

                let node = ExtendedAstNode::Unparseable {
                    pattern: pattern.clone(),
                    raw_text: Arc::from(&code[..end_pos]),
                    reason: "Dynamic delimiter cannot be statically determined".to_string(),
                    diagnostics: vec![],
                    recovery_point: end_pos,
                };

                (node, end_pos)
            }

            _ => {
                // Default handling for other patterns
                let node = ExtendedAstNode::Unparseable {
                    pattern: pattern.clone(),
                    raw_text: Arc::from(""),
                    reason: "Unhandled anti-pattern".to_string(),
                    diagnostics: vec![],
                    recovery_point: 0,
                };

                (node, 0)
            }
        }
    }
}

#[derive(Debug)]
pub struct ParseResult {
    pub ast: ExtendedAstNode,
    pub diagnostics: Vec<Diagnostic>,
    pub parse_coverage: f64,
    pub recovery_points: Vec<usize>,
}

impl ParseResult {
    /// Generate a comprehensive report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        // Summary
        report.push_str(&format!("Parse Coverage: {:.1}%\n", self.parse_coverage));

        if !self.diagnostics.is_empty() {
            report.push_str(&format!("Issues Found: {}\n\n", self.diagnostics.len()));

            // Anti-pattern report
            let detector = AntiPatternDetector::new();
            report.push_str(&detector.format_report(&self.diagnostics));
        }

        if !self.recovery_points.is_empty() {
            report.push_str(&format!("\nRecovery Points: {:?}\n", self.recovery_points));
        }

        // AST summary
        report.push_str("\nAST Structure:\n");
        report.push_str(&self.ast.to_sexp());
        report.push('\n');

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_understanding_parser_clean_code() {
        let mut parser = UnderstandingParser::new();
        let code = r#"
my $x = 42;
print "Hello, world!\n";
"#;

        use perl_tdd_support::must;
        let result = must(parser.parse_with_understanding(code));
        assert_eq!(result.parse_coverage, 100.0);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn test_understanding_parser_with_format() {
        let mut parser = UnderstandingParser::new();
        let code = r#"
format REPORT =
<<'END'
Name: @<<<<<<<<<<<<
$name
END
.
"#;

        use perl_tdd_support::must;
        let result = must(parser.parse_with_understanding(code));
        assert!(!result.diagnostics.is_empty());
        assert!(result.ast.has_anti_patterns());
    }

    #[test]
    fn test_understanding_parser_with_begin_heredoc() {
        let mut parser = UnderstandingParser::new();
        let code = r#"
BEGIN {
    $config = <<'END';
    server = localhost
END
}
"#;

        use perl_tdd_support::must;
        let result = must(parser.parse_with_understanding(code));
        assert!(!result.diagnostics.is_empty());
        let report = result.generate_report();
        assert!(report.contains("BEGIN"));
        assert!(report.contains("side effects"));
    }
}
