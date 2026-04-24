//! Snapshot tests for pull diagnostics output surface.
//!
//! These tests capture the baseline output of key functions to detect any
//! unintended changes. Snapshot tests are particularly important for the
//! @INC path inclusion feature (PL701) because the diagnostic message format
//! is user-visible.
//!
//! ## What's Snapshot Tested
//!
//! - PL701 diagnostic messages (with and without include_paths)
//! - Unchanged document diagnostic report structure
//! - Full document diagnostic report structure
//! - Parse error diagnostic messages
//! - Type mapper outputs (severity, tags, related information)

use insta::assert_snapshot;
use lsp_types::{DocumentDiagnosticReport, NumberOrString, Uri};
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;

/// Helper to extract items from a full diagnostic report.
fn get_full_items(report: DocumentDiagnosticReport) -> Vec<lsp_types::Diagnostic> {
    match report {
        DocumentDiagnosticReport::Full(full) => full.full_document_diagnostic_report.items,
        _ => vec![],
    }
}

/// Returns true when a diagnostic has the given code string (e.g. "PL701").
fn has_code(diag: &lsp_types::Diagnostic, code: &str) -> bool {
    matches!(&diag.code, Some(NumberOrString::String(s)) if s == code)
}

/// Formats a diagnostic for snapshot comparison.
fn format_diagnostic(diag: &lsp_types::Diagnostic) -> String {
    let code_str = diag
        .code
        .as_ref()
        .map(|c| match c {
            NumberOrString::String(s) => s.clone(),
            NumberOrString::Number(n) => n.to_string(),
        })
        .unwrap_or_default();

    let severity_str = diag.severity.map(|s| format!("{:?}", s)).unwrap_or_default();

    let data_str = diag.data.as_ref().map(|d| format!("{:#}", d)).unwrap_or_default();

    let related = diag
        .related_information
        .as_ref()
        .map(|v| {
            v.iter()
                .map(|ri| {
                    format!(
                        "  - location: {:?} {}:{}-{}:{}, message: {:?}",
                        ri.location.uri,
                        ri.location.range.start.line,
                        ri.location.range.start.character,
                        ri.location.range.end.line,
                        ri.location.range.end.character,
                        ri.message,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!(
        "code: {}\n\
         severity: {}\n\
         message: {}\n\
         source: {:?}\n\
         range: {}:{}-{}:{}\n\
         tags: {:?}\n\
         data: {}\n\
         related_information:\n{}",
        code_str,
        severity_str,
        diag.message,
        diag.source,
        diag.range.start.line,
        diag.range.start.character,
        diag.range.end.line,
        diag.range.end.character,
        diag.tags,
        data_str,
        related
    )
}

/// Formats a document diagnostic report for snapshot comparison.
fn format_report(report: &DocumentDiagnosticReport) -> String {
    match report {
        DocumentDiagnosticReport::Full(full) => {
            let items = &full.full_document_diagnostic_report.items;
            let result_id = full
                .full_document_diagnostic_report
                .result_id
                .clone()
                .unwrap_or_else(|| "<none>".to_string());
            let formatted_items: Vec<String> = items.iter().map(format_diagnostic).collect();
            format!(
                "kind: Full\n\
                 result_id: {}\n\
                 items_count: {}\n\
                 items:\n{}",
                result_id,
                items.len(),
                formatted_items.join("\n---\n")
            )
        }
        DocumentDiagnosticReport::Unchanged(unchanged) => {
            format!(
                "kind: Unchanged\n\
                 result_id: {}",
                unchanged.unchanged_document_diagnostic_report.result_id
            )
        }
    }
}

// ============================================================================
// PL701 @INC Path Inclusion Snapshots
//
// These snapshots verify the PL701 diagnostic message format when include_paths
// are provided vs when they are not provided.
// ============================================================================

/// Snapshot PL701 diagnostic message WITH include_paths.
/// This verifies the message format includes the searched paths.
#[test]
fn snapshot_pl701_diagnostic_with_include_paths() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_missing_module.pl".parse().unwrap();
    // Use a non-core module that will definitely not be found
    let content = "use Missing::Module::That::Does::Not::Exist;\n";

    // Provide specific include paths that should appear in the PL701 message
    let include_paths = Some(vec!["/test/path1".to_string(), "/test/path2".to_string()]);

    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = get_full_items(report);

    // Find the PL701 diagnostic
    let pl701 = items.iter().find(|d| has_code(d, "PL701")).unwrap();

    // Snapshot the diagnostic
    assert_snapshot!("pl701_with_include_paths", format_diagnostic(pl701));
}

/// Snapshot PL701 diagnostic message WITHOUT include_paths (None).
/// This verifies the fallback message format.
#[test]
fn snapshot_pl701_diagnostic_without_include_paths() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_missing_module.pl".parse().unwrap();
    let content = "use Missing::Module::That::Does::Not::Exist;\n";

    // Pass None for include_paths - should use fallback message
    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    // Find the PL701 diagnostic
    let pl701 = items.iter().find(|d| has_code(d, "PL701")).unwrap();

    // Snapshot the diagnostic
    assert_snapshot!("pl701_without_include_paths", format_diagnostic(pl701));
}

/// Snapshot PL701 diagnostic with empty include_paths vec.
/// This verifies that Some(vec![]) behaves the same as None.
#[test]
fn snapshot_pl701_diagnostic_with_empty_include_paths() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_missing_module.pl".parse().unwrap();
    let content = "use Missing::Module::That::Does::Not::Exist;\n";

    // Pass Some(vec![]) for include_paths - should use fallback message (same as None)
    let include_paths = Some(vec![]);
    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = get_full_items(report);

    // Find the PL701 diagnostic
    let pl701 = items.iter().find(|d| has_code(d, "PL701")).unwrap();

    // Snapshot the diagnostic
    assert_snapshot!("pl701_with_empty_include_paths", format_diagnostic(pl701));
}

/// Snapshot PL701 with single include path.
#[test]
fn snapshot_pl701_diagnostic_single_include_path() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_missing_module.pl".parse().unwrap();
    let content = "use Missing::Module;\n";

    let include_paths = Some(vec!["/custom/lib".to_string()]);
    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = get_full_items(report);

    let pl701 = items.iter().find(|d| has_code(d, "PL701")).unwrap();
    assert_snapshot!("pl701_single_include_path", format_diagnostic(pl701));
}

/// Snapshot PL701 with path containing spaces.
#[test]
fn snapshot_pl701_diagnostic_include_path_with_spaces() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_missing_module.pl".parse().unwrap();
    let content = "use Missing::Module;\n";

    let include_paths = Some(vec!["/path/with spaces".to_string(), "/another/path".to_string()]);
    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = get_full_items(report);

    let pl701 = items.iter().find(|d| has_code(d, "PL701")).unwrap();
    assert_snapshot!("pl701_include_path_with_spaces", format_diagnostic(pl701));
}

// ============================================================================
// Unchanged Report Snapshot
//
// Verifies the unchanged document diagnostic report structure.
// ============================================================================

/// Snapshot the unchanged document diagnostic report structure.
#[test]
fn snapshot_unchanged_report_structure() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = "my $x = 1;\n";

    // First request - full report
    let first = provider.get_document_diagnostics(&uri, content, None, None);
    let result_id = match &first {
        DocumentDiagnosticReport::Full(full) => {
            full.full_document_diagnostic_report.result_id.clone().unwrap()
        }
        DocumentDiagnosticReport::Unchanged(_) => panic!("expected full report first"),
    };

    // Second request with same content - should be unchanged
    let second = provider.get_document_diagnostics(&uri, content, Some(result_id), None);

    assert_snapshot!("unchanged_report", format_report(&second));
}

// ============================================================================
// Full Report Snapshot
//
// Verifies the full document diagnostic report structure.
// ============================================================================

/// Snapshot the full document diagnostic report structure.
#[test]
fn snapshot_full_report_structure() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    // Content that produces a parse error
    let content = "my $x = ;\n";

    let report = provider.get_document_diagnostics(&uri, content, None, None);

    assert_snapshot!("full_report", format_report(&report));
}

/// Snapshot full report with multiple diagnostic types.
#[test]
fn snapshot_full_report_multiple_diagnostics() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = r#"use strict;
use warnings;
print $y;
my $unused = 1;
"#;

    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    // Snapshot the diagnostics sorted by code for stable ordering
    let mut sorted_items: Vec<_> = items.iter().collect();
    sorted_items.sort_by_key(|d| {
        d.code
            .as_ref()
            .and_then(|c| match c {
                NumberOrString::String(s) => Some(s.clone()),
                NumberOrString::Number(n) => Some(n.to_string()),
            })
            .unwrap_or_default()
    });

    let formatted: Vec<String> = sorted_items.iter().map(|d| format_diagnostic(d)).collect();
    assert_snapshot!("full_report_multiple_diagnostics", formatted.join("\n---\n"));
}

// ============================================================================
// Parse Error Diagnostic Snapshots
//
// Verifies the fallback parse error diagnostic message format.
// ============================================================================

/// Snapshot parse error diagnostic for missing semicolon.
#[test]
fn snapshot_parse_error_missing_semicolon() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = "my $x = 1\nmy $y = 2;\n"; // Missing semicolon on first line

    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    // Find PL001 (parse error)
    let parse_errors: Vec<_> = items
        .iter()
        .filter(|d| has_code(d, "PL001") || has_code(d, "PL002") || has_code(d, "PL003"))
        .collect();

    if !parse_errors.is_empty() {
        assert_snapshot!("parse_error_missing_semicolon", format_diagnostic(parse_errors[0]));
    }
}

/// Snapshot parse error diagnostic for unclosed block.
#[test]
fn snapshot_parse_error_unclosed_block() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = "sub foo {\n    my $x = 1;\n"; // Missing closing brace

    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    let parse_errors: Vec<_> = items
        .iter()
        .filter(|d| has_code(d, "PL001") || has_code(d, "PL002") || has_code(d, "PL003"))
        .collect();

    if !parse_errors.is_empty() {
        assert_snapshot!("parse_error_unclosed_block", format_diagnostic(parse_errors[0]));
    }
}

/// Snapshot parse error diagnostic for unclosed string.
#[test]
fn snapshot_parse_error_unclosed_string() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = "my $x = \"hello world;\n"; // Unclosed string

    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    let parse_errors: Vec<_> = items
        .iter()
        .filter(|d| has_code(d, "PL001") || has_code(d, "PL002") || has_code(d, "PL003"))
        .collect();

    if !parse_errors.is_empty() {
        assert_snapshot!("parse_error_unclosed_string", format_diagnostic(parse_errors[0]));
    }
}

// ============================================================================
// Diagnostic Data JSON Structure Snapshots
//
// Verifies the structured data attached to diagnostics.
// ============================================================================

/// Snapshot the DiagnosticData JSON for a parse error.
#[test]
fn snapshot_diagnostic_data_parse_error() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = "my $x = ;\n";

    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    let pl001 = items.iter().find(|d| has_code(d, "PL001")).unwrap();
    let data = pl001.data.as_ref().unwrap();

    let snapshot = serde_json::json!({
        "code": data["code"],
        "category": data["category"],
        "fixable": data["fixable"],
        "tags": data["tags"],
    });
    assert_snapshot!("diagnostic_data_parse_error", format!("{:#}", snapshot));
}

/// Snapshot the DiagnosticData JSON for PL100 (missing strict).
#[test]
fn snapshot_diagnostic_data_missing_strict() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = "print 'hello';\n"; // Missing strict

    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    let pl100 = items.iter().find(|d| has_code(d, "PL100")).unwrap();
    let data = pl100.data.as_ref().unwrap();

    let snapshot = serde_json::json!({
        "code": data["code"],
        "category": data["category"],
        "fixable": data["fixable"],
        "tags": data["tags"],
    });
    assert_snapshot!("diagnostic_data_missing_strict", format!("{:#}", snapshot));
}

/// Snapshot the DiagnosticData JSON for PL701 (missing module).
#[test]
fn snapshot_diagnostic_data_pl701() {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse().unwrap();
    let content = "use Missing::Module;\n";

    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = get_full_items(report);

    let pl701 = items.iter().find(|d| has_code(d, "PL701")).unwrap();
    let data = pl701.data.as_ref().unwrap();

    let snapshot = serde_json::json!({
        "code": data["code"],
        "category": data["category"],
        "fixable": data["fixable"],
        "tags": data["tags"],
    });
    assert_snapshot!("diagnostic_data_pl701", format!("{:#}", snapshot));
}
