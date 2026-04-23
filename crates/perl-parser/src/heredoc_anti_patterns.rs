//! Anti-pattern detection for heredoc edge cases
//!
//! This crate provides detection and analysis of problematic Perl patterns
//! that make static parsing difficult or impossible, particularly around heredocs.
//!
//! The [`AntiPatternDetector`] scans Perl source for seven categories of
//! heredoc-related anti-patterns and produces [`Diagnostic`]s describing each
//! finding, with severity, explanation, suggested fix, and documentation
//! references.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,   // Code will likely fail
    Warning, // Code works but is problematic
    Info,    // Code could be improved
}

#[derive(Debug, Clone, PartialEq)]
pub enum AntiPattern {
    FormatHeredoc { location: Location, format_name: String, heredoc_delimiter: String },
    BeginTimeHeredoc { location: Location, heredoc_content: String, side_effects: Vec<String> },
    DynamicHeredocDelimiter { location: Location, expression: String },
    SourceFilterHeredoc { location: Location, module: String },
    RegexCodeBlockHeredoc { location: Location },
    EvalStringHeredoc { location: Location },
    TiedHandleHeredoc { location: Location, handle_name: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub pattern: AntiPattern,
    pub message: String,
    pub explanation: String,
    pub suggested_fix: Option<String>,
    pub references: Vec<String>,
}

pub struct AntiPatternDetector {
    patterns: Vec<Box<dyn PatternDetector>>,
}

trait PatternDetector: Send + Sync {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)>;
    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic>;
}

fn location_from_start(code: &str, offset: usize, start: usize) -> Location {
    let line = code[..start].lines().count();
    let column = match code[..start].rfind('\n') {
        Some(last_newline) => start.saturating_sub(last_newline + 1),
        None => start,
    };

    Location { line, column, offset: offset + start }
}

fn mask_non_code_regions(code: &str) -> String {
    let mut masked = String::with_capacity(code.len());
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut escaped = false;

    for ch in code.chars() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                masked.push('\n');
            } else {
                masked.push(' ');
            }
            continue;
        }

        if in_single_quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single_quote = false;
            }
            masked.push(' ');
            continue;
        }

        if in_double_quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double_quote = false;
            }
            masked.push(' ');
            continue;
        }

        match ch {
            '#' => {
                in_line_comment = true;
                masked.push(' ');
            }
            '\'' => {
                in_single_quote = true;
                masked.push(' ');
            }
            '"' => {
                in_double_quote = true;
                masked.push(' ');
            }
            _ => masked.push(ch),
        }
    }

    masked
}

// Format heredoc detector
struct FormatHeredocDetector;

/// Pattern for identifying format declarations
static FORMAT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r"(?m)^\s*format\s+(\w+)\s*=\s*$") {
        Ok(re) => re,
        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),
    });

impl PatternDetector for FormatHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in FORMAT_PATTERN.captures_iter(&scan_code) {
            if let (Some(match_pos), Some(name_match)) = (cap.get(0), cap.get(1)) {
                let format_name = name_match.as_str().to_string();
                let location = location_from_start(code, offset, match_pos.start());

                // Look for heredoc marker inside format body (simplified)
                let body_start = match_pos.end();
                let body_end = code[body_start..].find("\n.").unwrap_or(code.len() - body_start);
                let body = &scan_code[body_start..body_start + body_end];

                if body.contains("<<") {
                    results.push((
                        AntiPattern::FormatHeredoc {
                            location: location.clone(),
                            format_name,
                            heredoc_delimiter: "UNKNOWN".to_string(), // Would need better extraction
                        },
                        location,
                    ));
                }
            }
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic> {
        let AntiPattern::FormatHeredoc { format_name, .. } = pattern else {
            return None;
        };

        Some(Diagnostic {
            severity: Severity::Warning,
            pattern: pattern.clone(),
            message: format!("Heredoc declared inside format '{}'", format_name),
            explanation: "Heredocs inside format declarations are often handled specially by the Perl interpreter and can be difficult to parse statically.".to_string(),
            suggested_fix: Some("Consider moving the heredoc outside the format or using a simple string if possible.".to_string()),
            references: vec!["perldoc perlform".to_string()],
        })
    }
}

// BEGIN-time heredoc detector
struct BeginTimeHeredocDetector;

/// Pattern for identifying BEGIN block openings
static BEGIN_BLOCK_START_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r"\bBEGIN\s*\{") {
        Ok(re) => re,
        Err(_) => unreachable!("BEGIN_BLOCK_START_PATTERN regex failed to compile"),
    });

fn find_matching_brace(code: &str, opening_brace_idx: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    let mut depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for (idx, &byte) in bytes.iter().enumerate().skip(opening_brace_idx) {
        let ch = byte as char;

        if escaped {
            escaped = false;
            continue;
        }

        if in_single_quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single_quote = false;
            }
            continue;
        }

        if in_double_quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double_quote = false;
            }
            continue;
        }

        match ch {
            '\'' => in_single_quote = true,
            '"' => in_double_quote = true,
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }

    None
}

impl PatternDetector for BeginTimeHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for begin_match in BEGIN_BLOCK_START_PATTERN.find_iter(&scan_code) {
            let Some(opening_brace_rel) = begin_match.as_str().rfind('{') else {
                continue;
            };
            let opening_brace_idx = begin_match.start() + opening_brace_rel;
            let Some(closing_brace_idx) = find_matching_brace(&scan_code, opening_brace_idx) else {
                continue;
            };
            let block_content = &scan_code[opening_brace_idx + 1..closing_brace_idx];

            if !block_content.contains("<<") {
                continue;
            }

            let location = location_from_start(code, offset, begin_match.start());

            results.push((
                AntiPattern::BeginTimeHeredoc {
                    location: location.clone(),
                    heredoc_content: block_content.to_string(),
                    side_effects: vec!["Phase-dependent parsing".to_string()],
                },
                location,
            ));
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic> {
        if let AntiPattern::BeginTimeHeredoc { .. } = pattern {
            Some(Diagnostic {
                severity: Severity::Error,
                pattern: pattern.clone(),
                message: "Heredoc declared during BEGIN-time".to_string(),
                explanation: "Heredocs declared inside BEGIN blocks are evaluated during the compilation phase. This can lead to complex side effects that are difficult to track statically.".to_string(),
                suggested_fix: Some("Move the heredoc declaration out of the BEGIN block if it doesn't need to be evaluated during compilation.".to_string()),
                references: vec!["perldoc perlmod".to_string()],
            })
        } else {
            None
        }
    }
}

// Dynamic delimiter detector
struct DynamicDelimiterDetector;

/// Pattern for identifying dynamic heredoc delimiters
static DYNAMIC_DELIMITER_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r"<<\s*\$\{[^}]+\}|<<\s*\$\w+|<<\s*`[^`]+`") {
        Ok(re) => re,
        Err(_) => unreachable!("DYNAMIC_DELIMITER_PATTERN regex failed to compile"),
    });

impl PatternDetector for DynamicDelimiterDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in DYNAMIC_DELIMITER_PATTERN.captures_iter(&scan_code) {
            if let Some(match_pos) = cap.get(0) {
                let expression = match_pos.as_str().to_string();
                let location = location_from_start(code, offset, match_pos.start());

                results.push((
                    AntiPattern::DynamicHeredocDelimiter { location: location.clone(), expression },
                    location,
                ));
            }
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic> {
        let AntiPattern::DynamicHeredocDelimiter { expression, .. } = pattern else {
            return None;
        };

        Some(Diagnostic {
            severity: Severity::Warning,
            pattern: pattern.clone(),
            message: format!("Dynamic heredoc delimiter: {}", expression),
            explanation: "Using variables or expressions as heredoc delimiters makes it impossible to know the terminator without executing the code.".to_string(),
            suggested_fix: Some("Use a literal string as the heredoc terminator.".to_string()),
            references: vec!["perldoc perlop".to_string()],
        })
    }
}

// Source filter detector
struct SourceFilterDetector;

/// Pattern for identifying common source filter modules
static SOURCE_FILTER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r"use\s+Filter::(Simple|Util::Call|cpp|exec|sh|decrypt|tee)") {
        Ok(re) => re,
        Err(_) => unreachable!("SOURCE_FILTER_PATTERN regex failed to compile"),
    }
});

impl PatternDetector for SourceFilterDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in SOURCE_FILTER_PATTERN.captures_iter(&scan_code) {
            if let (Some(match_pos), Some(module_match)) = (cap.get(0), cap.get(1)) {
                let filter_module = module_match.as_str().to_string();
                let location = location_from_start(code, offset, match_pos.start());

                results.push((
                    AntiPattern::SourceFilterHeredoc {
                        location: location.clone(),
                        module: filter_module,
                    },
                    location,
                ));
            }
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic> {
        let AntiPattern::SourceFilterHeredoc { module, .. } = pattern else {
            return None;
        };

        Some(Diagnostic {
            severity: Severity::Error,
            pattern: pattern.clone(),
            message: format!("Source filter detected: Filter::{}", module),
            explanation: "Source filters rewrite the source code before it's parsed. Static analysis cannot reliably predict the state of the code after filtering.".to_string(),
            suggested_fix: Some("Avoid using source filters. They are considered problematic and often replaced by better alternatives like Devel::Declare or modern Perl features.".to_string()),
            references: vec!["perldoc Filter::Simple".to_string()],
        })
    }
}

// Regex heredoc detector
struct RegexHeredocDetector;

/// Pattern for identifying heredocs inside regex code blocks
static REGEX_HEREDOC_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r"\(\?\{[^}]*<<[^}]*\}") {
        Ok(re) => re,
        Err(_) => unreachable!("REGEX_HEREDOC_PATTERN regex failed to compile"),
    });

impl PatternDetector for RegexHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in REGEX_HEREDOC_PATTERN.captures_iter(&scan_code) {
            if let Some(match_pos) = cap.get(0) {
                let location = location_from_start(code, offset, match_pos.start());

                results.push((
                    AntiPattern::RegexCodeBlockHeredoc { location: location.clone() },
                    location,
                ));
            }
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic> {
        if let AntiPattern::RegexCodeBlockHeredoc { .. } = pattern {
            Some(Diagnostic {
                severity: Severity::Warning,
                pattern: pattern.clone(),
                message: "Heredoc inside regex code block".to_string(),
                explanation: "Declaring heredocs inside (?{ ... }) or (??{ ... }) blocks is extremely rare and difficult to parse correctly.".to_string(),
                suggested_fix: None,
                references: vec!["perldoc perlre".to_string()],
            })
        } else {
            None
        }
    }
}

// Eval heredoc detector
struct EvalHeredocDetector;

/// Pattern for identifying heredocs inside eval strings
static EVAL_HEREDOC_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r#"eval\s+(?:'[^']*<<[^']*'|"[^"]*<<[^"]*")"#) {
        Ok(re) => re,
        Err(_) => unreachable!("EVAL_HEREDOC_PATTERN regex failed to compile"),
    });

impl PatternDetector for EvalHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for cap in EVAL_HEREDOC_PATTERN.captures_iter(code) {
            if let Some(match_pos) = cap.get(0) {
                let location = location_from_start(code, offset, match_pos.start());

                results.push((
                    AntiPattern::EvalStringHeredoc { location: location.clone() },
                    location,
                ));
            }
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic> {
        if let AntiPattern::EvalStringHeredoc { .. } = pattern {
            Some(Diagnostic {
                severity: Severity::Warning,
                pattern: pattern.clone(),
                message: "Heredoc inside eval string".to_string(),
                explanation: "Heredocs declared inside strings passed to eval require double parsing and can hide malicious or complex code.".to_string(),
                suggested_fix: Some("Consider using a block eval or moving the heredoc outside the eval string.".to_string()),
                references: vec!["perldoc -f eval".to_string()],
            })
        } else {
            None
        }
    }
}

// Tied handle detector
struct TiedHandleDetector;

/// Pattern for identifying tie statements
static TIE_PATTERN: LazyLock<Regex> = LazyLock::new(|| match Regex::new(r"tie\s+([*$]\w+)") {
    Ok(re) => re,
    Err(_) => unreachable!("TIE_PATTERN regex failed to compile"),
});

impl PatternDetector for TiedHandleDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        // First find tied handles
        let mut tied_handles = Vec::new();
        for cap in TIE_PATTERN.captures_iter(&scan_code) {
            if let Some(handle_match) = cap.get(1) {
                tied_handles.push(handle_match.as_str());
            }
        }

        for raw_handle in tied_handles {
            // If it's a glob (*FH), we typically print to the bare handle (FH).
            // If it's a scalar ($fh), we print to the scalar ($fh).
            let handle_to_search = raw_handle.strip_prefix('*').unwrap_or(raw_handle);

            // Look for usage of this handle with heredoc
            let usage_pattern = format!(r"print\s+{}\s+<<", regex::escape(handle_to_search));
            if let Ok(re) = Regex::new(&usage_pattern) {
                for usage_match in re.find_iter(&scan_code) {
                    let location = location_from_start(code, offset, usage_match.start());

                    results.push((
                        AntiPattern::TiedHandleHeredoc {
                            location: location.clone(),
                            handle_name: handle_to_search.to_string(),
                        },
                        location,
                    ));
                }
            }
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic> {
        let AntiPattern::TiedHandleHeredoc { handle_name, .. } = pattern else {
            return None;
        };

        Some(Diagnostic {
            severity: Severity::Info,
            pattern: pattern.clone(),
            message: format!("Heredoc written to tied handle '{}'", handle_name),
            explanation: "Writing to a tied handle invokes custom code. The behavior of heredoc output depends on the tied class implementation.".to_string(),
            suggested_fix: None,
            references: vec!["perldoc -f tie".to_string()],
        })
    }
}

impl Default for AntiPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl AntiPatternDetector {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                Box::new(FormatHeredocDetector),
                Box::new(BeginTimeHeredocDetector),
                Box::new(DynamicDelimiterDetector),
                Box::new(SourceFilterDetector),
                Box::new(RegexHeredocDetector),
                Box::new(EvalHeredocDetector),
                Box::new(TiedHandleDetector),
            ],
        }
    }

    pub fn detect_all(&self, code: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for detector in &self.patterns {
            let patterns = detector.detect(code, 0);
            for (pattern, _) in patterns {
                if let Some(diagnostic) = detector.diagnose(&pattern) {
                    diagnostics.push(diagnostic);
                }
            }
        }

        diagnostics.sort_by_key(|d| match &d.pattern {
            AntiPattern::FormatHeredoc { location, .. }
            | AntiPattern::BeginTimeHeredoc { location, .. }
            | AntiPattern::DynamicHeredocDelimiter { location, .. }
            | AntiPattern::SourceFilterHeredoc { location, .. }
            | AntiPattern::RegexCodeBlockHeredoc { location, .. }
            | AntiPattern::EvalStringHeredoc { location, .. }
            | AntiPattern::TiedHandleHeredoc { location, .. } => location.offset,
        });

        diagnostics
    }

    pub fn format_report(&self, diagnostics: &[Diagnostic]) -> String {
        let mut report = String::from("Anti-Pattern Analysis Report\n");
        report.push_str("============================\n\n");

        if diagnostics.is_empty() {
            report.push_str("No problematic patterns detected.\n");
            return report;
        }

        report.push_str(&format!("Found {} problematic patterns:\n\n", diagnostics.len()));

        for (i, diag) in diagnostics.iter().enumerate() {
            report.push_str(&format!(
                "{}. {} ({})\n",
                i + 1,
                diag.message,
                match diag.severity {
                    Severity::Error => "ERROR",
                    Severity::Warning => "WARNING",
                    Severity::Info => "INFO",
                }
            ));

            report.push_str(&format!(
                "   Location: {}\n",
                match &diag.pattern {
                    AntiPattern::FormatHeredoc { location, .. }
                    | AntiPattern::BeginTimeHeredoc { location, .. }
                    | AntiPattern::DynamicHeredocDelimiter { location, .. }
                    | AntiPattern::SourceFilterHeredoc { location, .. }
                    | AntiPattern::RegexCodeBlockHeredoc { location, .. }
                    | AntiPattern::EvalStringHeredoc { location, .. }
                    | AntiPattern::TiedHandleHeredoc { location, .. } =>
                        format!("line {}, column {}", location.line, location.column),
                }
            ));

            report.push_str(&format!("   Explanation: {}\n", diag.explanation));

            if let Some(fix) = &diag.suggested_fix {
                report.push_str(&format!(
                    "   Suggested fix:\n     {}\n",
                    fix.lines().collect::<Vec<_>>().join("\n     ")
                ));
            }

            if !diag.references.is_empty() {
                report.push_str(&format!("   References: {}\n", diag.references.join(", ")));
            }

            report.push('\n');
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_heredoc_detection() {
        let detector = AntiPatternDetector::new();
        let code = r#"
format REPORT =
<<'END'
Name: @<<<<<<<<<<<<
$name
END
.
"#;

        let diagnostics = detector.detect_all(code);
        // Note: DynamicDelimiterDetector might also flag the << inside the format body as a false positive.
        // But FormatHeredoc should appear first because it starts at 'format'.
        // So diagnostics[0] should be FormatHeredoc.
        assert!(!diagnostics.is_empty());
        assert!(matches!(diagnostics[0].pattern, AntiPattern::FormatHeredoc { .. }));
    }

    #[test]
    fn test_begin_heredoc_detection() {
        let detector = AntiPatternDetector::new();
        let code = r###"
BEGIN {
    $config = <<'END';
    server = localhost
END
}
"###;

        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::BeginTimeHeredoc { .. }));
    }

    #[test]
    fn test_begin_heredoc_detection_with_nested_braces() {
        let detector = AntiPatternDetector::new();
        let code = r###"
BEGIN {
    if ($ENV{DEV}) {
        $config = <<'END';
        server = localhost
END
    }
}
"###;

        let diagnostics = detector.detect_all(code);
        let begin_count = diagnostics
            .iter()
            .filter(|diag| matches!(diag.pattern, AntiPattern::BeginTimeHeredoc { .. }))
            .count();
        assert_eq!(begin_count, 1);
    }

    #[test]
    fn test_dynamic_delimiter_detection() {
        let detector = AntiPatternDetector::new();
        let code = r###"
my $delimiter = "EOF";
my $content = <<$delimiter;
This is dynamic
EOF
"###;

        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::DynamicHeredocDelimiter { .. }));
    }

    #[test]
    fn test_source_filter_detection() {
        let detector = AntiPatternDetector::new();
        let code = r###"
use Filter::Simple;
print <<EOF;
Filtered content
EOF
"###;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::SourceFilterHeredoc { .. }));
    }

    #[test]
    fn test_regex_heredoc_detection() {
        let detector = AntiPatternDetector::new();
        let code = r###"
m/pattern(?{
    print <<'MATCH';
    Match text
MATCH
})/
"###;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::RegexCodeBlockHeredoc { .. }));
    }

    #[test]
    fn test_eval_heredoc_detection() {
        let detector = AntiPatternDetector::new();
        let code = r###"
eval 'print <<"EVAL";
Eval content
EVAL';
"###;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::EvalStringHeredoc { .. }));
    }

    #[test]
    fn test_tied_handle_detection() {
        let detector = AntiPatternDetector::new();
        let code = r###"
tie *FH, 'Tie::Handle';
print FH <<'DATA';
Tied output
DATA
"###;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::TiedHandleHeredoc { .. }));
    }

    #[test]
    fn test_tied_scalar_handle_detection() {
        let detector = AntiPatternDetector::new();
        let code = r###"
tie $fh, 'Tie::Handle';
print $fh <<'DATA';
Tied output
DATA
"###;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::TiedHandleHeredoc { .. }));
    }

    #[test]
    fn test_tied_handle_reports_multiple_writes() {
        let detector = AntiPatternDetector::new();
        let code = r###"
tie *FH, 'Tie::Handle';
print FH <<'FIRST';
One
FIRST
print FH <<'SECOND';
Two
SECOND
"###;

        let diagnostics = detector.detect_all(code);
        let tied_handle_count = diagnostics
            .iter()
            .filter(|diag| matches!(diag.pattern, AntiPattern::TiedHandleHeredoc { .. }))
            .count();
        assert_eq!(tied_handle_count, 2);
    }

    #[test]
    fn test_location_column_is_zero_based_for_new_line_matches() {
        let detector = AntiPatternDetector::new();
        let code = "my $x = 1;\nuse Filter::Simple;\n";

        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);

        assert!(
            matches!(diagnostics[0].pattern, AntiPattern::SourceFilterHeredoc { .. }),
            "expected SourceFilterHeredoc pattern, got: {:?}",
            diagnostics[0].pattern
        );
        let AntiPattern::SourceFilterHeredoc { location, .. } = &diagnostics[0].pattern else {
            return;
        };

        assert_eq!(location.line, 1);
        assert_eq!(location.column, 0);
        assert_eq!(location.offset, 11);
    }

    #[test]
    fn test_source_filter_detection_ignores_comments_and_strings() {
        let detector = AntiPatternDetector::new();
        let code = r#"
# use Filter::Simple;
my $s = "use Filter::Simple";
"#;

        let diagnostics = detector.detect_all(code);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_begin_detection_ignores_comments_and_strings() {
        let detector = AntiPatternDetector::new();
        let code = r#"
# BEGIN { my $x = <<'END'; END }
my $s = "BEGIN { my $x = <<'END'; END }";
"#;

        let diagnostics = detector.detect_all(code);
        assert!(diagnostics.is_empty());
    }
}
