//! Heredoc anti-pattern detection diagnostics

use crate::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser::heredoc_anti_patterns::{AntiPattern, AntiPatternDetector, Severity};

/// Detect heredoc anti-patterns in Perl source code.
///
/// Returns diagnostics for problematic heredoc patterns (eval strings,
/// dynamic delimiters, format blocks, etc.)
pub fn detect_heredoc_antipatterns(source: &str) -> Vec<Diagnostic> {
    let detector = AntiPatternDetector::new();
    let raw = detector.detect_all(source);

    raw.into_iter()
        .map(|d| {
            let offset = extract_offset(&d.pattern);
            let end_offset = (offset + 1).min(source.len());

            let severity = match d.severity {
                Severity::Error => DiagnosticSeverity::Error,
                Severity::Warning => DiagnosticSeverity::Warning,
                Severity::Info => DiagnosticSeverity::Information,
            };

            let code = antipattern_code(&d.pattern);

            Diagnostic {
                range: (offset, end_offset),
                severity,
                code: Some(code.to_string()),
                message: d.message,
                related_information: Vec::new(),
                tags: Vec::new(),
                suggestion: d.suggested_fix,
            }
        })
        .collect()
}

fn extract_offset(pattern: &AntiPattern) -> usize {
    match pattern {
        AntiPattern::FormatHeredoc { location, .. }
        | AntiPattern::BeginTimeHeredoc { location, .. }
        | AntiPattern::DynamicHeredocDelimiter { location, .. }
        | AntiPattern::SourceFilterHeredoc { location, .. }
        | AntiPattern::RegexCodeBlockHeredoc { location, .. }
        | AntiPattern::EvalStringHeredoc { location, .. }
        | AntiPattern::TiedHandleHeredoc { location, .. } => location.offset,
    }
}

fn antipattern_code(pattern: &AntiPattern) -> &'static str {
    match pattern {
        AntiPattern::FormatHeredoc { .. } => DiagnosticCode::HeredocInFormat.as_str(),
        AntiPattern::BeginTimeHeredoc { .. } => DiagnosticCode::HeredocInBegin.as_str(),
        AntiPattern::DynamicHeredocDelimiter { .. } => {
            DiagnosticCode::HeredocDynamicDelimiter.as_str()
        }
        AntiPattern::SourceFilterHeredoc { .. } => DiagnosticCode::HeredocInSourceFilter.as_str(),
        AntiPattern::RegexCodeBlockHeredoc { .. } => DiagnosticCode::HeredocInRegexCode.as_str(),
        AntiPattern::EvalStringHeredoc { .. } => DiagnosticCode::HeredocInEval.as_str(),
        AntiPattern::TiedHandleHeredoc { .. } => DiagnosticCode::HeredocTiedHandle.as_str(),
    }
}
