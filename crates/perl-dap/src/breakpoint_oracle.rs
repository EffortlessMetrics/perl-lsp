//! A reusable breakpoint *truth layer* over the existing AST validator.
//!
//! Both the native DAP path and the external-peer / session-packet path need to
//! answer the same questions: *is this line breakable? where are the breakable
//! lines? what subroutines exist?* Rather than re-derive that logic, this module
//! exposes a small [`BreakpointOracle`] trait backed by the existing
//! [`crate::breakpoint::AstBreakpointValidator`] (decision D5).

use perl_parser::Parser;
use perl_parser::ast::{Node, NodeKind};
use ropey::Rope;

use crate::breakpoint::{AstBreakpointValidator, BreakpointError, BreakpointValidator};
use crate::model::{DebugBreakpoint, DebugFunctionSymbol, DebugSource};

/// The oracle's verdict on a single requested breakpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakpointValidationOutcome {
    /// Whether the breakpoint can bind.
    pub verified: bool,
    /// The line it would actually bind to (may be adjusted from the request).
    pub actual_line: u32,
    /// Human-readable note when rejected or adjusted.
    pub message: Option<String>,
}

/// The breakpoint truth layer.
pub trait BreakpointOracle {
    /// Validate a single requested source breakpoint.
    fn validate_source_breakpoint(
        &self,
        breakpoint: &DebugBreakpoint,
    ) -> BreakpointValidationOutcome;

    /// Enumerate every breakable (executable) line in the source.
    fn breakable_line_candidates(&self) -> Vec<u32>;

    /// Enumerate the subroutines defined in the source.
    fn function_candidates(&self) -> Vec<DebugFunctionSymbol>;
}

/// An [`BreakpointOracle`] backed by the AST validator for one source.
pub struct AstBreakpointOracle {
    source: DebugSource,
    validator: AstBreakpointValidator,
    rope: Rope,
    line_count: u32,
    ast: Node,
}

impl AstBreakpointOracle {
    /// Build an oracle for `source` from its `text`.
    ///
    /// # Errors
    /// Returns [`BreakpointError`] if the source cannot be parsed.
    pub fn new(source: DebugSource, text: &str) -> Result<Self, BreakpointError> {
        let validator = AstBreakpointValidator::new(text)?;
        let rope = Rope::from_str(text);
        let line_count = u32::try_from(rope.len_lines()).unwrap_or(u32::MAX);
        let mut parser = Parser::new(text);
        let ast = parser.parse().map_err(|e| BreakpointError::ParseError(format!("{e:?}")))?;
        Ok(Self { source, validator, rope, line_count, ast })
    }

    /// 1-based line for a byte offset, clamped into range.
    fn line_of(&self, byte: usize) -> u32 {
        let byte = byte.min(self.rope.len_bytes());
        let line0 = self.rope.byte_to_line(byte);
        u32::try_from(line0 + 1).unwrap_or(u32::MAX)
    }

    fn collect_subs(&self, node: &Node, out: &mut Vec<DebugFunctionSymbol>) {
        if let NodeKind::Subroutine { name, .. } = &node.kind {
            if let Some(name) = name {
                out.push(DebugFunctionSymbol {
                    name: name.clone(),
                    source: self.source.clone(),
                    start_line: self.line_of(node.location.start),
                    end_line: self.line_of(node.location.end),
                });
            }
        }
        node.for_each_child(|child| self.collect_subs(child, out));
    }
}

impl BreakpointOracle for AstBreakpointOracle {
    fn validate_source_breakpoint(
        &self,
        breakpoint: &DebugBreakpoint,
    ) -> BreakpointValidationOutcome {
        let line = i64::from(breakpoint.line);
        let column = breakpoint.column.map(i64::from);
        let v = self.validator.validate_with_column(line, column);
        BreakpointValidationOutcome {
            verified: v.verified,
            actual_line: u32::try_from(v.line.max(0)).unwrap_or(breakpoint.line),
            message: v.message,
        }
    }

    fn breakable_line_candidates(&self) -> Vec<u32> {
        (1..=self.line_count)
            .filter(|&line| self.validator.is_executable_line(i64::from(line)))
            .collect()
    }

    fn function_candidates(&self) -> Vec<DebugFunctionSymbol> {
        let mut out = Vec::new();
        self.collect_subs(&self.ast, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oracle(text: &str) -> AstBreakpointOracle {
        AstBreakpointOracle::new(DebugSource::from_path("/work/script.pl"), text).expect("parse")
    }

    #[test]
    fn comment_line_is_not_verified() {
        let o = oracle("# a comment\nmy $x = 1;\nprint $x;\n");
        let bp = DebugBreakpoint {
            id: None,
            source: DebugSource::from_path("/work/script.pl"),
            line: 1,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        };
        let outcome = o.validate_source_breakpoint(&bp);
        assert!(!outcome.verified, "a comment line cannot hold a breakpoint");
        assert!(outcome.message.is_some());
    }

    #[test]
    fn executable_line_is_verified() {
        let o = oracle("# a comment\nmy $x = 1;\nprint $x;\n");
        let bp = DebugBreakpoint {
            id: None,
            source: DebugSource::from_path("/work/script.pl"),
            line: 2,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        };
        assert!(o.validate_source_breakpoint(&bp).verified);
    }

    #[test]
    fn breakable_lines_exclude_comments_and_blanks() {
        let o = oracle("# comment\n\nmy $x = 1;\nprint $x;\n");
        let lines = o.breakable_line_candidates();
        assert!(!lines.contains(&1), "comment excluded");
        assert!(!lines.contains(&2), "blank excluded");
        assert!(lines.contains(&3), "assignment included");
    }

    #[test]
    fn function_candidates_reports_named_subs() {
        let src = "sub run {\n    my $x = 1;\n    return $x;\n}\n\nsub helper {\n    1;\n}\n";
        let subs = oracle(src).function_candidates();
        let names: Vec<&str> = subs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"run"), "found run: {names:?}");
        assert!(names.contains(&"helper"), "found helper: {names:?}");
        let run = subs.iter().find(|s| s.name == "run").expect("run");
        assert_eq!(run.start_line, 1);
        assert!(run.end_line >= run.start_line);
    }
}
