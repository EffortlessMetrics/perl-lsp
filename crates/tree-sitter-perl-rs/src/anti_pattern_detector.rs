//! Anti-pattern detection for heredoc edge cases
//!
//! This module provides detection and analysis of problematic Perl patterns
//! that make static parsing difficult or impossible, particularly around heredocs.

use once_cell::sync::Lazy;
use regex::Regex;

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

#[derive(Debug, Clone)]
pub enum AntiPattern {
    FormatHeredoc {
        location: Location,
        format_name: String,
        heredoc_delimiter: String,
    },
    BeginTimeHeredoc {
        location: Location,
        side_effects: Vec<String>,
        heredoc_content: String,
    },
    SourceFilterHeredoc {
        location: Location,
        filter_module: String,
        affected_lines: Vec<usize>,
    },
    DynamicHeredocDelimiter {
        location: Location,
        expression: String,
    },
    RegexCodeBlockHeredoc {
        location: Location,
        pattern: String,
        flags: String,
    },
    EvalStringHeredoc {
        location: Location,
        eval_type: String, // "string" or "block"
        contains_heredoc: bool,
    },
    TiedHandleHeredoc {
        location: Location,
        handle_name: String,
    },
}

#[derive(Debug, Clone)]
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
    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic;
}

// Format heredoc detector
struct FormatHeredocDetector;

static FORMAT_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*format\s+(\w+)\s*=\s*$").unwrap());

impl PatternDetector for FormatHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for cap in FORMAT_PATTERN.captures_iter(code) {
            let match_pos = cap.get(0).unwrap();
            let format_name = cap.get(1).unwrap().as_str().to_string();

            // Check if next non-empty line is a heredoc
            let after_format = &code[match_pos.end()..];
            for line in after_format.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if trimmed.starts_with("<<") {
                    let location = Location {
                        line: code[..match_pos.start()].lines().count(),
                        column: match_pos.start()
                            - code[..match_pos.start()].rfind('\n').unwrap_or(0),
                        offset: offset + match_pos.start(),
                    };

                    let delimiter = trimmed
                        .trim_start_matches("<<")
                        .trim_start_matches(['\'', '"', '`'])
                        .split([' ', '\t', ';', '\n'])
                        .next()
                        .unwrap_or("")
                        .to_string();

                    results.push((
                        AntiPattern::FormatHeredoc {
                            location: location.clone(),
                            format_name,
                            heredoc_delimiter: delimiter,
                        },
                        location,
                    ));
                }
                break; // Only check the first non-empty line
            }
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
        let AntiPattern::FormatHeredoc { format_name, .. } = pattern else {
            // Defensive programming
            panic!("FormatHeredocDetector received incompatible pattern type.");
        };

        Diagnostic {
            severity: Severity::Warning,
            pattern: pattern.clone(),
            message: format!("Format '{}' uses heredoc syntax", format_name),
            explanation: "Perl formats are deprecated since Perl 5.8. Their interaction with heredocs can cause parsing ambiguities and maintenance issues.".to_string(),
            suggested_fix: Some("Consider using sprintf, printf, or a templating module like Template::Toolkit instead:\n\nmy $report = sprintf(\"%-20s %10s\\n\", $name, $value);".to_string()),
            references: vec![
                "perldoc perlform".to_string(),
                "https://perldoc.perl.org/perldiag#Use-of-uninitialized-value-in-format".to_string(),
            ],
        }
    }
}

// BEGIN-time heredoc detector
struct BeginTimeHeredocDetector;

static BEGIN_BLOCK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)\bBEGIN\s*\{([^}]*<<[^}]*)\}").unwrap());

impl PatternDetector for BeginTimeHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for cap in BEGIN_BLOCK_PATTERN.captures_iter(code) {
            let match_pos = cap.get(0).unwrap();
            let block_content = cap.get(1).unwrap().as_str();

            // Detect side effects in BEGIN block
            let mut side_effects = Vec::new();

            if block_content.contains('$') && block_content.contains('=') {
                side_effects.push("Global variable modification".to_string());
            }
            if block_content.contains("sub ") {
                side_effects.push("Subroutine definition".to_string());
            }
            if block_content.contains("require ") || block_content.contains("use ") {
                side_effects.push("Module loading".to_string());
            }
            if block_content.contains("open ") {
                side_effects.push("File operations".to_string());
            }

            let location = Location {
                line: code[..match_pos.start()].lines().count(),
                column: match_pos.start() - code[..match_pos.start()].rfind('\n').unwrap_or(0),
                offset: offset + match_pos.start(),
            };

            results.push((
                AntiPattern::BeginTimeHeredoc {
                    location: location.clone(),
                    side_effects,
                    heredoc_content: block_content.to_string(),
                },
                location,
            ));
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
        let AntiPattern::BeginTimeHeredoc { side_effects, .. } = pattern else {
            panic!("BeginTimeHeredocDetector received incompatible pattern type.");
        };

        let effects_str = if side_effects.is_empty() {
            "No obvious side effects detected".to_string()
        } else {
            format!("Detected side effects: {}", side_effects.join(", "))
        };

        Diagnostic {
            severity: Severity::Warning,
            pattern: pattern.clone(),
            message: "Heredoc in BEGIN block detected".to_string(),
            explanation: format!("BEGIN blocks execute at compile time, making heredocs difficult to parse statically. {}", effects_str),
            suggested_fix: Some("Move heredoc initialization to INIT block or runtime:\n\nour $config;\nINIT {\n    $config = <<'END';\n    ...\nEND\n}".to_string()),
            references: vec![
                "perldoc perlmod#BEGIN,-UNITCHECK,-CHECK,-INIT-and-END".to_string(),
            ],
        }
    }
}

// Dynamic heredoc delimiter detector
struct DynamicDelimiterDetector;

static DYNAMIC_DELIMITER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<<\s*\$\{[^}]+\}|<<\s*\$\w+|<<\s*`[^`]+`").unwrap());

impl PatternDetector for DynamicDelimiterDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for match_pos in DYNAMIC_DELIMITER_PATTERN.find_iter(code) {
            let location = Location {
                line: code[..match_pos.start()].lines().count(),
                column: match_pos.start() - code[..match_pos.start()].rfind('\n').unwrap_or(0),
                offset: offset + match_pos.start(),
            };

            results.push((
                AntiPattern::DynamicHeredocDelimiter {
                    location: location.clone(),
                    expression: match_pos.as_str().to_string(),
                },
                location,
            ));
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
        let AntiPattern::DynamicHeredocDelimiter { expression, .. } = pattern else {
            panic!("DynamicDelimiterDetector received incompatible pattern type.");
        };

        Diagnostic {
            severity: Severity::Error,
            pattern: pattern.clone(),
            message: format!("Dynamic heredoc delimiter: {}", expression),
            explanation: "Heredoc delimiters computed at runtime cannot be parsed statically. This makes the code unpredictable and potentially insecure.".to_string(),
            suggested_fix: Some("Use a static delimiter with variable interpolation inside the heredoc:\n\nmy $content = <<\"END\";\nDynamic value: $variable\nEND".to_string()),
            references: vec![
                "perldoc perlop#'<<EOF'".to_string(),
            ],
        }
    }
}

// Source filter detector
struct SourceFilterDetector;

static SOURCE_FILTER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"use\s+Filter::(Simple|Util::Call|cpp|exec|sh|decrypt|tee)").unwrap());

impl PatternDetector for SourceFilterDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for cap in SOURCE_FILTER_PATTERN.captures_iter(code) {
            let match_pos = cap.get(0).unwrap();
            let filter_module = cap.get(1).unwrap().as_str().to_string();

            let location = Location {
                line: code[..match_pos.start()].lines().count(),
                column: match_pos.start() - code[..match_pos.start()].rfind('\n').unwrap_or(0),
                offset: offset + match_pos.start(),
            };

            results.push((
                AntiPattern::SourceFilterHeredoc {
                    location: location.clone(),
                    filter_module,
                    affected_lines: vec![], // Placeholder as determining exact affected lines is hard
                },
                location,
            ));
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
        let AntiPattern::SourceFilterHeredoc { filter_module, .. } = pattern else {
            panic!("SourceFilterDetector received incompatible pattern type.");
        };

        Diagnostic {
            severity: Severity::Error,
            pattern: pattern.clone(),
            message: format!("Source filter 'Filter::{}' detected", filter_module),
            explanation: "Source filters can transform code before parsing, making static analysis unreliable or impossible.".to_string(),
            suggested_fix: Some("Avoid source filters if possible, or expect limited IDE functionality.".to_string()),
            references: vec!["perldoc Filter::Simple".to_string()],
        }
    }
}

// Regex code block heredoc detector
struct RegexHeredocDetector;

// Heuristic: (?{ ... << ... })
static REGEX_HEREDOC_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\(\?\{[^}]*<<[^}]*\}").unwrap());

impl PatternDetector for RegexHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for match_pos in REGEX_HEREDOC_PATTERN.find_iter(code) {
            let location = Location {
                line: code[..match_pos.start()].lines().count(),
                column: match_pos.start() - code[..match_pos.start()].rfind('\n').unwrap_or(0),
                offset: offset + match_pos.start(),
            };

            results.push((
                AntiPattern::RegexCodeBlockHeredoc {
                    location: location.clone(),
                    pattern: match_pos.as_str().to_string(),
                    flags: "".to_string(), // Detection of flags is complex without full parse
                },
                location,
            ));
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
        let AntiPattern::RegexCodeBlockHeredoc { .. } = pattern else {
            panic!("RegexHeredocDetector received incompatible pattern type.");
        };

        Diagnostic {
            severity: Severity::Error,
            pattern: pattern.clone(),
            message: "Heredoc detected inside regex code block (?{...})".to_string(),
            explanation: "Code blocks inside regexes are executed during pattern matching. Containing heredocs within them is highly problematic for static analysis and parsing.".to_string(),
            suggested_fix: Some("Refactor the code to construct the string outside the regex.".to_string()),
            references: vec!["perldoc perlre".to_string()],
        }
    }
}

// Eval string heredoc detector
struct EvalHeredocDetector;

// Heuristic: eval '...<<...' or eval "...<<..."
// Rust regex doesn't support backreferences (\1) or lookarounds.
// We match simple cases of single or double quoted strings containing <<.
static EVAL_HEREDOC_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"eval\s+(?:'[^']*<<[^']*'|"[^"]*<<[^"]*")"#).unwrap());

impl PatternDetector for EvalHeredocDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for match_pos in EVAL_HEREDOC_PATTERN.find_iter(code) {
            let location = Location {
                line: code[..match_pos.start()].lines().count(),
                column: match_pos.start() - code[..match_pos.start()].rfind('\n').unwrap_or(0),
                offset: offset + match_pos.start(),
            };

            results.push((
                AntiPattern::EvalStringHeredoc {
                    location: location.clone(),
                    eval_type: "string".to_string(),
                    contains_heredoc: true,
                },
                location,
            ));
        }

        results
    }

    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
        let AntiPattern::EvalStringHeredoc { .. } = pattern else {
            panic!("EvalHeredocDetector received incompatible pattern type.");
        };

        Diagnostic {
            severity: Severity::Warning,
            pattern: pattern.clone(),
            message: "Heredoc detected inside eval string".to_string(),
            explanation: "Code inside string eval is parsed at runtime. Static analysis cannot reliably determine the behavior of heredocs within it.".to_string(),
            suggested_fix: Some("Use block eval or avoid heredocs inside eval strings.".to_string()),
            references: vec!["perldoc -f eval".to_string()],
        }
    }
}

// Tied handle detector
struct TiedHandleDetector;

// Heuristic: tie *FH... then print FH <<...
static TIE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"tie\s+([*\$]?\w+)").unwrap());

impl PatternDetector for TiedHandleDetector {
    fn detect(&self, code: &str, offset: usize) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        // Find all tied handles
        for cap in TIE_PATTERN.captures_iter(code) {
            let raw_handle = cap.get(1).unwrap().as_str();

            // If it's a glob (*FH), we typically print to the bare handle (FH).
            // If it's a scalar ($fh), we print to the scalar ($fh).
            let handle_to_search =
                if raw_handle.starts_with('*') { &raw_handle[1..] } else { raw_handle };

            // Look for usage of this handle with heredoc
            let usage_pattern = format!(r"print\s+{}\s+<<", regex::escape(handle_to_search));
            if let Ok(re) = Regex::new(&usage_pattern) {
                if let Some(usage_match) = re.find(code) {
                    let location = Location {
                        line: code[..usage_match.start()].lines().count(),
                        column: usage_match.start()
                            - code[..usage_match.start()].rfind('\n').unwrap_or(0),
                        offset: offset + usage_match.start(),
                    };

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

    fn diagnose(&self, pattern: &AntiPattern) -> Diagnostic {
        let AntiPattern::TiedHandleHeredoc { handle_name, .. } = pattern else {
            panic!("TiedHandleDetector received incompatible pattern type.");
        };

        Diagnostic {
            severity: Severity::Info,
            pattern: pattern.clone(),
            message: format!("Heredoc written to tied handle '{}'", handle_name),
            explanation: "Writing to a tied handle invokes custom code. The behavior of heredoc output depends on the tied class implementation.".to_string(),
            suggested_fix: None,
            references: vec!["perldoc -f tie".to_string()],
        }
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
                diagnostics.push(detector.diagnose(&pattern));
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

            report.push_str("\n");
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
        let code = r#"
BEGIN {
    $config = <<'END';
    server = localhost
END
}
"#;

        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::BeginTimeHeredoc { .. }));
    }

    #[test]
    fn test_dynamic_delimiter_detection() {
        let detector = AntiPatternDetector::new();
        let code = r#"
my $delimiter = "EOF";
my $content = <<$delimiter;
This is dynamic
EOF
"#;

        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::DynamicHeredocDelimiter { .. }));
    }

    #[test]
    fn test_source_filter_detection() {
        let detector = AntiPatternDetector::new();
        let code = r#"
use Filter::Simple;
print <<EOF;
Filtered content
EOF
"#;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::SourceFilterHeredoc { .. }));
    }

    #[test]
    fn test_regex_heredoc_detection() {
        let detector = AntiPatternDetector::new();
        let code = r#"
m/pattern(?{
    print <<'MATCH';
    Match text
MATCH
})/;
"#;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::RegexCodeBlockHeredoc { .. }));
    }

    #[test]
    fn test_eval_heredoc_detection() {
        let detector = AntiPatternDetector::new();
        let code = r#"
eval 'print <<"EVAL";
Eval content
EVAL';
"#;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::EvalStringHeredoc { .. }));
    }

    #[test]
    fn test_tied_handle_detection() {
        let detector = AntiPatternDetector::new();
        let code = r#"
tie *FH, 'Tie::Handle';
print FH <<'DATA';
Tied output
DATA
"#;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::TiedHandleHeredoc { .. }));
    }

    #[test]
    fn test_tied_scalar_handle_detection() {
        let detector = AntiPatternDetector::new();
        let code = r#"
tie $fh, 'Tie::Handle';
print $fh <<'DATA';
Tied output
DATA
"#;
        let diagnostics = detector.detect_all(code);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].pattern, AntiPattern::TiedHandleHeredoc { .. }));
    }
}
