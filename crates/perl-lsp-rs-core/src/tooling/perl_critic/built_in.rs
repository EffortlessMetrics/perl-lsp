use super::{built_in_quick_fix, insertion_range, QuickFix, Severity, Violation};
use perl_parser_core::Node;

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
        missing_use_statement_violation(
            self,
            content,
            "strict",
            "Always use strict to catch common mistakes",
        )
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
        missing_use_statement_violation(
            self,
            content,
            "warnings",
            "Always use warnings to catch potential issues",
        )
    }
}

/// Prohibit two-argument `open`.
struct ProhibitTwoArgOpen;

impl Policy for ProhibitTwoArgOpen {
    fn name(&self) -> &str {
        "InputOutput::ProhibitTwoArgOpen"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        open_statement_violations(
            self,
            content,
            has_two_arg_open,
            "Use three-argument open with an explicit mode to avoid shell interpolation hazards",
            "Code uses two-argument open",
        )
    }
}

/// Prohibit bareword filehandles in `open`.
struct ProhibitBarewordFileHandles;

impl Policy for ProhibitBarewordFileHandles {
    fn name(&self) -> &str {
        "InputOutput::ProhibitBarewordFileHandles"
    }

    fn severity(&self) -> Severity {
        Severity::Stern
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        open_statement_violations(
            self,
            content,
            opens_bareword_filehandle,
            "Use lexical filehandles (my $fh) instead of bareword handles",
            "Code opens a bareword filehandle",
        )
    }
}

impl Default for BuiltInAnalyzer {
    fn default() -> Self {
        Self {
            policies: vec![
                Box::new(RequireUseStrict),
                Box::new(RequireUseWarnings),
                Box::new(ProhibitTwoArgOpen),
                Box::new(ProhibitBarewordFileHandles),
            ],
        }
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
        built_in_quick_fix(violation)
    }
}

fn missing_use_statement_violation(
    policy: &dyn Policy,
    content: &str,
    feature: &str,
    explanation: &str,
) -> Vec<Violation> {
    if content.contains(&format!("use {feature}")) {
        return Vec::new();
    }

    vec![Violation {
        policy: policy.name().to_string(),
        description: format!("Code does not use {feature}"),
        explanation: explanation.to_string(),
        severity: policy.severity(),
        range: insertion_range(),
        file: String::new(),
    }]
}

fn open_statement_violations(
    policy: &dyn Policy,
    content: &str,
    predicate: impl Fn(&str) -> bool,
    explanation: &str,
    description: &str,
) -> Vec<Violation> {
    extract_open_statements(content)
        .into_iter()
        .filter(|(_, statement)| predicate(statement))
        .map(|(start, _)| {
            let (line_num, col_num) = byte_offset_to_line_col(content, start);
            let line_u32 = u32::try_from(line_num).unwrap_or(u32::MAX);
            let col_u32 = u32::try_from(col_num).unwrap_or(u32::MAX);
            Violation {
                policy: policy.name().to_string(),
                description: description.to_string(),
                explanation: explanation.to_string(),
                severity: policy.severity(),
                range: perl_parser_core::position::Range {
                    start: perl_parser_core::position::Position {
                        line: line_u32,
                        column: col_u32,
                        byte: start,
                    },
                    end: perl_parser_core::position::Position {
                        line: line_u32,
                        column: col_u32.saturating_add(4),
                        byte: start.saturating_add(4),
                    },
                },
                file: String::new(),
            }
        })
        .collect()
}

/// Convert a byte offset into (line, column) — both zero-indexed.
/// Works correctly with both LF and CRLF line endings.
fn byte_offset_to_line_col(content: &str, offset: usize) -> (usize, usize) {
    let prefix = &content[..offset.min(content.len())];
    let line = prefix.bytes().filter(|&b| b == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |idx| idx + 1);
    let col = content[line_start..offset.min(content.len())].chars().count();
    (line, col)
}

fn extract_open_statements(content: &str) -> Vec<(usize, &str)> {
    let mut statements = Vec::new();
    // Use byte-level scanning to correctly track offsets regardless of line ending style.
    let mut offset = 0usize;

    for line in content.lines() {
        let trimmed = line.trim_start();
        let leading = line.len().saturating_sub(trimmed.len());
        if let Some(open_idx) = trimmed.find("open") {
            let absolute_open = offset + leading + open_idx;
            let before = trimmed[..open_idx].chars().last();
            let after = trimmed[open_idx + 4..].chars().next();
            let word_boundary_before =
                before.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
            let word_boundary_after =
                after.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
            if word_boundary_before && word_boundary_after {
                let statement = &trimmed[open_idx..];
                statements.push((absolute_open, statement));
            }
        }
        // `str::lines()` strips the line ending from each line, but `line.len()` does not
        // include the newline bytes. We must advance offset by the raw byte length of the
        // line *including* any line-ending character(s). To avoid CRLF over-counting we
        // find the next newline in content starting from offset + line.len().
        let after_line = offset + line.len();
        if content.as_bytes().get(after_line) == Some(&b'\r') {
            offset = after_line + 2; // CRLF
        } else if content.as_bytes().get(after_line) == Some(&b'\n') {
            offset = after_line + 1; // LF
        } else {
            offset = after_line; // EOF — no trailing newline
        }
    }

    statements
}

fn has_two_arg_open(open_stmt: &str) -> bool {
    if !open_stmt.starts_with("open") {
        return false;
    }
    let comment_free = open_stmt.split('#').next().unwrap_or(open_stmt);
    if !comment_free.contains(',') {
        return false;
    }

    let mut comma_count = 0usize;
    for ch in comment_free.chars() {
        if ch == ',' {
            comma_count += 1;
        }
        if ch == ';' || ch == ')' {
            break;
        }
    }

    comma_count == 1
}

fn opens_bareword_filehandle(open_stmt: &str) -> bool {
    if !open_stmt.starts_with("open") {
        return false;
    }
    let comment_free = open_stmt.split('#').next().unwrap_or(open_stmt);
    let mut rest = comment_free.trim_start_matches("open").trim_start();
    if let Some(stripped) = rest.strip_prefix('(') {
        rest = stripped.trim_start();
    }

    let Some(first_arg_end) = rest.find(',') else {
        return false;
    };
    let first_arg = rest[..first_arg_end].trim();

    let lexical_prefixes = ["$", "@", "%", "my ", "our ", "local "];
    if lexical_prefixes.iter().any(|prefix| first_arg.starts_with(prefix)) {
        return false;
    }

    first_arg.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn built_in_analyzer_reports_two_arg_open_and_bareword_filehandle() -> TestResult {
        let analyzer = BuiltInAnalyzer::new();
        let content = "open FH, $path;\n";
        let mut parser = perl_parser_core::Parser::new(content);
        let ast = parser.parse()?;

        let violations = analyzer.analyze(&ast, content);
        let has_two_arg = violations.iter().any(|v| v.policy == "InputOutput::ProhibitTwoArgOpen");
        let has_bareword =
            violations.iter().any(|v| v.policy == "InputOutput::ProhibitBarewordFileHandles");

        if !has_two_arg {
            return Err("expected InputOutput::ProhibitTwoArgOpen violation".into());
        }
        if !has_bareword {
            return Err("expected InputOutput::ProhibitBarewordFileHandles violation".into());
        }

        Ok(())
    }

    #[test]
    fn built_in_analyzer_accepts_three_arg_open_with_lexical_filehandle() -> TestResult {
        let analyzer = BuiltInAnalyzer::new();
        let content = "open(my $fh, '<', $path);\n";
        let mut parser = perl_parser_core::Parser::new(content);
        let ast = parser.parse()?;

        let violations = analyzer.analyze(&ast, content);
        let has_open_policies = violations.iter().any(|v| {
            v.policy == "InputOutput::ProhibitTwoArgOpen"
                || v.policy == "InputOutput::ProhibitBarewordFileHandles"
        });

        if has_open_policies {
            return Err(
                "did not expect open-related built-in policy violations for three-arg lexical open"
                    .into(),
            );
        }

        Ok(())
    }
}
