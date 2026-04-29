use super::{QuickFix, Severity, Violation, built_in_quick_fix, insertion_range};
use perl_parser_core::Node;
use perl_parser_core::position::{Position, Range};

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

/// Prohibit bareword filehandles in `open`.
struct ProhibitBarewordFileHandles;

/// Prohibit two-argument `open`.
struct ProhibitTwoArgOpen;

/// Prohibit string-based eval
struct ProhibitStringyEval;

impl Policy for ProhibitBarewordFileHandles {
    fn name(&self) -> &str {
        "InputOutput::ProhibitBarewordFileHandles"
    }

    fn severity(&self) -> Severity {
        Severity::Stern
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        find_bareword_open_filehandles(content)
            .into_iter()
            .map(|range| Violation {
                policy: self.name().to_string(),
                description: "Code uses a bareword filehandle".to_string(),
                explanation: "Use lexical filehandles (e.g. my $fh) for safer IO".to_string(),
                severity: self.severity(),
                range,
                file: String::new(),
            })
            .collect()
    }
}

impl Policy for ProhibitTwoArgOpen {
    fn name(&self) -> &str {
        "InputOutput::ProhibitTwoArgOpen"
    }

    fn severity(&self) -> Severity {
        Severity::Harsh
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        extract_open_statements(content)
            .into_iter()
            .filter(|(_, statement)| has_two_arg_open(statement))
            .map(|(start, _)| {
                let r = range_for_match(content, start, start + 4);
                Violation {
                    policy: self.name().to_string(),
                    description: "Code uses two-argument open".to_string(),
                    explanation: "Use three-argument open with an explicit mode to avoid shell interpolation hazards".to_string(),
                    severity: self.severity(),
                    range: r,
                    file: String::new(),
                }
            })
            .collect()
    }
}

impl Policy for ProhibitStringyEval {
    fn name(&self) -> &str {
        "BuiltinFunctions::ProhibitStringyEval"
    }

    fn severity(&self) -> Severity {
        Severity::Cruel
    }

    fn analyze(&self, _ast: &Node, content: &str) -> Vec<Violation> {
        if !has_stringy_eval(content) {
            return Vec::new();
        }

        vec![Violation {
            policy: self.name().to_string(),
            description: "Code uses string eval".to_string(),
            explanation:
                "String eval executes dynamically generated code and is difficult to analyze safely"
                    .to_string(),
            severity: self.severity(),
            range: insertion_range(),
            file: String::new(),
        }]
    }
}

impl Default for BuiltInAnalyzer {
    fn default() -> Self {
        Self {
            policies: vec![
                Box::new(RequireUseStrict),
                Box::new(RequireUseWarnings),
                Box::new(ProhibitBarewordFileHandles),
                Box::new(ProhibitTwoArgOpen),
                Box::new(ProhibitStringyEval),
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
    if has_use_statement(content, feature) {
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


fn has_use_statement(content: &str, feature: &str) -> bool {
    content.lines().any(|line| line_has_use_feature(line, feature))
}

fn line_has_use_feature(line: &str, feature: &str) -> bool {
    let code_portion = line.split('#').next().unwrap_or_default();
    let use_stmt = format!("use {feature}");
    let mut search = code_portion;

    while let Some(pos) = search.find(&use_stmt) {
        let before_ok = pos == 0
            || !search[..pos]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');

        let after = &search[pos + use_stmt.len()..];
        let after_trimmed = after.trim_start();
        let after_ok = after_trimmed.is_empty()
            || after_trimmed.starts_with(';')
            || after_trimmed.starts_with('(')
            || after_trimmed.starts_with('v')
            || after_trimmed.starts_with(char::is_whitespace);

        if before_ok && after_ok {
            return true;
        }

        search = &search[pos + use_stmt.len()..];
    }

    false
}

fn find_bareword_open_filehandles(content: &str) -> Vec<Range> {
    let mut ranges = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0usize;

    while i + 4 <= bytes.len() {
        if &bytes[i..i + 4] != b"open" || !is_word_boundary(bytes, i, i + 4) {
            i += 1;
            continue;
        }

        let mut cursor = i + 4;
        cursor = skip_ascii_whitespace(bytes, cursor);
        if bytes.get(cursor) != Some(&b'(') {
            i += 1;
            continue;
        }

        cursor = skip_ascii_whitespace(bytes, cursor + 1);
        let Some(handle_start) = bytes.get(cursor).copied() else {
            break;
        };
        if !handle_start.is_ascii_uppercase() {
            i += 1;
            continue;
        }

        let mut handle_end = cursor + 1;
        while bytes
            .get(handle_end)
            .is_some_and(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b'_')
        {
            handle_end += 1;
        }

        let after_handle = skip_ascii_whitespace(bytes, handle_end);
        if bytes.get(after_handle) == Some(&b',') {
            ranges.push(range_for_match(content, cursor, handle_end));
        }

        i = handle_end;
    }

    ranges
}

fn range_for_match(content: &str, start: usize, end: usize) -> Range {
    let prefix = &content[..start];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |idx| idx + 1);
    let column = content[line_start..start].chars().count();
    let width = content[start..end].chars().count();
    let line_u32 = usize_to_u32(line);
    let column_u32 = usize_to_u32(column);
    let width_u32 = usize_to_u32(width);

    Range {
        start: Position { byte: start, line: line_u32, column: column_u32 },
        end: Position { byte: end, line: line_u32, column: column_u32.saturating_add(width_u32) },
    }
}

fn usize_to_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

fn is_word_boundary(bytes: &[u8], start: usize, end: usize) -> bool {
    let left = start.checked_sub(1).and_then(|idx| bytes.get(idx)).copied();
    let right = bytes.get(end).copied();
    !left.is_some_and(is_word_byte) && !right.is_some_and(is_word_byte)
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn skip_ascii_whitespace(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    cursor
}

/// Extract `open` statements from `content`, returning (byte_offset, &str) pairs.
/// The byte offset points to the start of the `open` keyword on the line.
/// Works correctly with both LF and CRLF line endings.
fn extract_open_statements(content: &str) -> Vec<(usize, &str)> {
    let mut statements = Vec::new();
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
        // Advance offset past the line bytes plus any line-ending byte(s).
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

/// Return `true` when `open_stmt` (starting with `open`) uses the two-argument form.
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

fn has_stringy_eval(content: &str) -> bool {
    content.lines().any(is_stringy_eval_line)
}

fn is_stringy_eval_line(line: &str) -> bool {
    let code_portion = line.split('#').next().unwrap_or_default();
    let mut search = code_portion;
    while let Some(eval_pos) = search.find("eval") {
        // Word boundary: char before must not be alphanumeric or '_'
        let before_ok = eval_pos == 0
            || !search[..eval_pos]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        let rest = &search[eval_pos + 4..];
        // Word boundary: char after must not be alphanumeric or '_'
        let after_ok = rest.chars().next().is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        if before_ok && after_ok {
            let after_eval = rest.trim_start();
            // String eval: literal strings (eval "..." / eval '...')
            // or variable expressions (eval $code / eval @args / eval \$ref)
            let is_literal_string = after_eval.starts_with('"') || after_eval.starts_with('\'');
            let is_variable = after_eval.starts_with('$')
                || after_eval.starts_with('@')
                || after_eval.starts_with('%')
                || after_eval.starts_with('\\');
            if is_literal_string || is_variable {
                return true;
            }
        }
        // Advance past this non-matching occurrence
        search = &search[eval_pos + 4..];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::BuiltInAnalyzer;
    use perl_parser::Parser;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn builtin_analyzer_flags_bareword_open_filehandle() -> TestResult {
        let source = "use strict;\nuse warnings;\nopen(FILE, '<', 'foo.txt');\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        let has_bareword_violation = violations
            .iter()
            .any(|violation| violation.policy == "InputOutput::ProhibitBarewordFileHandles");
        assert!(has_bareword_violation, "expected bareword filehandle violation");
        Ok(())
    }

    #[test]
    fn builtin_analyzer_accepts_lexical_open_filehandle() -> TestResult {
        let source = "use strict;\nuse warnings;\nopen(my $fh, '<', 'foo.txt');\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        let has_bareword_violation = violations
            .iter()
            .any(|violation| violation.policy == "InputOutput::ProhibitBarewordFileHandles");
        assert!(!has_bareword_violation, "lexical filehandles should not be flagged");
        Ok(())
    }

    #[test]
    fn reports_stringy_eval_violation() -> TestResult {
        let source = r#"
use strict;
use warnings;
my $src = '$x = 1;';
eval "$src";
"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(
            violations.iter().any(|v| v.policy == "BuiltinFunctions::ProhibitStringyEval"),
            "expected ProhibitStringyEval violation for eval \"...\""
        );
        Ok(())
    }

    #[test]
    fn ignores_block_eval() -> TestResult {
        let source = r#"
use strict;
use warnings;
eval { my $x = 1; };
"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(
            !violations.iter().any(|v| v.policy == "BuiltinFunctions::ProhibitStringyEval"),
            "block eval should not be flagged as stringy eval"
        );
        Ok(())
    }

    #[test]
    fn reports_stringy_eval_variable() -> TestResult {
        // eval $var is the most common real-world stringy eval pattern and must be caught.
        let source = r#"
use strict;
use warnings;
my $code = 'print "hello\n"';
eval $code;
"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(
            violations.iter().any(|v| v.policy == "BuiltinFunctions::ProhibitStringyEval"),
            "expected ProhibitStringyEval violation for eval $var pattern"
        );
        Ok(())
    }

    #[test]
    fn reports_stringy_eval_single_quote() -> TestResult {
        let source = "use strict;\nuse warnings;\neval 'print 1';\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(
            violations.iter().any(|v| v.policy == "BuiltinFunctions::ProhibitStringyEval"),
            "expected ProhibitStringyEval violation for eval '...'"
        );
        Ok(())
    }

    #[test]
    fn builtin_analyzer_flags_two_arg_open() -> TestResult {
        // Two-argument open: `open FH, $path;` — one comma, no explicit mode.
        let source = "use strict;\nuse warnings;\nopen FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        let has_two_arg_violation =
            violations.iter().any(|v| v.policy == "InputOutput::ProhibitTwoArgOpen");
        assert!(has_two_arg_violation, "expected InputOutput::ProhibitTwoArgOpen violation");
        Ok(())
    }

    #[test]
    fn builtin_analyzer_accepts_three_arg_open() -> TestResult {
        // Three-argument open: `open(my $fh, '<', $path);` — two commas, explicit mode.
        let source = "use strict;\nuse warnings;\nopen(my $fh, '<', $path);\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        let has_two_arg_violation =
            violations.iter().any(|v| v.policy == "InputOutput::ProhibitTwoArgOpen");
        assert!(
            !has_two_arg_violation,
            "three-argument open should not be flagged as two-argument open"
        );
        Ok(())
    }

    #[test]
    fn builtin_analyzer_ignores_commented_use_strict() -> TestResult {
        let source = "# use strict;
use warnings;
my $x = 1;
";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(
            violations.iter().any(|v| v.policy == "TestingAndDebugging::RequireUseStrict"),
            "commented use strict should not satisfy the policy"
        );
        Ok(())
    }

    #[test]
    fn builtin_analyzer_accepts_versioned_use_strict() -> TestResult {
        let source = "use strict v1.00;
use warnings;
my $x = 1;
";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        assert!(
            !violations.iter().any(|v| v.policy == "TestingAndDebugging::RequireUseStrict"),
            "versioned use strict should satisfy the policy"
        );
        Ok(())
    }

    #[test]
    fn two_arg_open_violation_has_correct_line() -> TestResult {
        // The violation must point to line 2 (zero-indexed), not line 0.
        let source = "use strict;\nuse warnings;\nopen FH, $path;\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let analyzer = BuiltInAnalyzer::new();
        let violations = analyzer.analyze(&ast, source);

        let v = violations
            .iter()
            .find(|v| v.policy == "InputOutput::ProhibitTwoArgOpen")
            .ok_or("expected InputOutput::ProhibitTwoArgOpen violation")?;

        assert_eq!(v.range.start.line, 2, "violation should be on line 2 (zero-indexed)");
        Ok(())
    }
}
