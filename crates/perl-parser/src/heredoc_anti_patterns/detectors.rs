use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::heredoc_anti_patterns::model::{AntiPattern, Diagnostic, Location, Severity};
use crate::heredoc_anti_patterns::utils::{
    build_line_starts, location_from_start, mask_non_code_regions,
};

/// Scans Perl source for heredoc-related anti-patterns and produces [`Diagnostic`]s.
///
/// Construct with [`AntiPatternDetector::new`], then call [`detect_all`] with the
/// source text. The detector runs all seven built-in pattern checkers and returns
/// the results sorted by byte offset so callers receive problems in source order.
///
/// [`detect_all`]: AntiPatternDetector::detect_all
pub struct AntiPatternDetector {
    patterns: Vec<Box<dyn PatternDetector>>,
}

trait PatternDetector: Send + Sync {
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)>;
    fn diagnose(&self, pattern: &AntiPattern) -> Option<Diagnostic>;
}

// Format heredoc detector
struct FormatHeredocDetector;

/// Pattern for identifying format declarations
static FORMAT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r"(?m)^\s*format\s+(\w+)\s*=\s*$") {
        Ok(re) => re,
        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),
    });

/// Pattern for extracting heredoc delimiter declarations.
static HEREDOC_DELIMITER_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r#"<<\s*['"`]?([A-Za-z_][A-Za-z0-9_]*)['"`]?"#) {
        Ok(re) => re,
        Err(_) => unreachable!("HEREDOC_DELIMITER_PATTERN regex failed to compile"),
    });

impl PatternDetector for FormatHeredocDetector {
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in FORMAT_PATTERN.captures_iter(&scan_code) {
            if let (Some(match_pos), Some(name_match)) = (cap.get(0), cap.get(1)) {
                let format_name = name_match.as_str().to_string();
                let location = location_from_start(line_starts, offset, match_pos.start());

                // Look for heredoc marker inside format body (simplified)
                let body_start = match_pos.end();
                let body_end = code[body_start..].find("\n.").unwrap_or(code.len() - body_start);
                let body = &scan_code[body_start..body_start + body_end];
                let source_body = &code[body_start..body_start + body_end];

                if body.contains("<<") {
                    results.push((
                        AntiPattern::FormatHeredoc {
                            location: location.clone(),
                            format_name,
                            heredoc_delimiter: extract_heredoc_delimiter(source_body),
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

fn extract_heredoc_delimiter(body: &str) -> String {
    HEREDOC_DELIMITER_PATTERN
        .captures(body)
        .and_then(|captures| captures.get(1).map(|delimiter| delimiter.as_str().to_string()))
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

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
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)> {
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

            let location = location_from_start(line_starts, offset, begin_match.start());

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
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in DYNAMIC_DELIMITER_PATTERN.captures_iter(&scan_code) {
            if let Some(match_pos) = cap.get(0) {
                let expression = match_pos.as_str().to_string();
                let location = location_from_start(line_starts, offset, match_pos.start());

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
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in SOURCE_FILTER_PATTERN.captures_iter(&scan_code) {
            if let (Some(match_pos), Some(module_match)) = (cap.get(0), cap.get(1)) {
                let filter_module = module_match.as_str().to_string();
                let location = location_from_start(line_starts, offset, match_pos.start());

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
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        for cap in REGEX_HEREDOC_PATTERN.captures_iter(&scan_code) {
            if let Some(match_pos) = cap.get(0) {
                let location = location_from_start(line_starts, offset, match_pos.start());

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
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();

        for cap in EVAL_HEREDOC_PATTERN.captures_iter(code) {
            if let Some(match_pos) = cap.get(0) {
                let location = location_from_start(line_starts, offset, match_pos.start());

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

/// Pattern for identifying print statements that write heredocs to a handle.
static PRINT_HEREDOC_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r"print\s+([*$]?\w+)\s+<<") {
        Ok(re) => re,
        Err(_) => unreachable!("PRINT_HEREDOC_PATTERN regex failed to compile"),
    });

impl PatternDetector for TiedHandleDetector {
    fn detect(
        &self,
        code: &str,
        offset: usize,
        line_starts: &[usize],
    ) -> Vec<(AntiPattern, Location)> {
        let mut results = Vec::new();
        let scan_code = mask_non_code_regions(code);

        // First collect tied handles in normalized form:
        // *FH -> FH, $fh -> $fh.
        let mut tied_handles = HashSet::new();
        for cap in TIE_PATTERN.captures_iter(&scan_code) {
            if let Some(handle_match) = cap.get(1) {
                let raw_handle = handle_match.as_str();
                let normalized = raw_handle.strip_prefix('*').unwrap_or(raw_handle);
                tied_handles.insert(normalized.to_string());
            }
        }

        // Use a single static regex for all print-heredoc matches, then filter
        // by whether the handle is in the tied set. This avoids O(n) Regex
        // compilations (one per tied handle) and is faster for large files.
        for cap in PRINT_HEREDOC_PATTERN.captures_iter(&scan_code) {
            let (Some(match_pos), Some(handle_match)) = (cap.get(0), cap.get(1)) else {
                continue;
            };

            let raw_print_handle = handle_match.as_str();
            let normalized_print_handle =
                raw_print_handle.strip_prefix('*').unwrap_or(raw_print_handle);

            if tied_handles.contains(normalized_print_handle) {
                let location = location_from_start(line_starts, offset, match_pos.start());
                results.push((
                    AntiPattern::TiedHandleHeredoc {
                        location: location.clone(),
                        handle_name: normalized_print_handle.to_string(),
                    },
                    location,
                ));
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
    /// Create a detector pre-loaded with all seven built-in pattern checkers.
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

    /// Run all pattern checkers against `code` and return diagnostics sorted by offset.
    pub fn detect_all(&self, code: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let line_starts = build_line_starts(code);

        for detector in &self.patterns {
            let patterns = detector.detect(code, 0, &line_starts);
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

    /// Format a list of diagnostics as a human-readable plain-text report.
    ///
    /// Prints a header, a count, and one entry per diagnostic including its
    /// severity, location, explanation, optional suggested fix, and references.
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
mod tests;
