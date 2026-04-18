//! Snapshot tests for PullDiagnosticsProvider with @INC path inclusion (PL701).
//!
//! These tests complement assertion-based diagnostics tests by snapshotting the
//! response payload shape for PL701 diagnostics with and without include_paths.
//! This makes protocol-visible changes obvious during review.

use insta::assert_yaml_snapshot;
use lsp_types::{DocumentDiagnosticReport, NumberOrString, Uri};
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;

/// Extract items from a full diagnostic report, returning an error if it is Unchanged.
fn items_from_report(
    report: DocumentDiagnosticReport,
) -> Result<Vec<lsp_types::Diagnostic>, Box<dyn std::error::Error>> {
    match report {
        DocumentDiagnosticReport::Full(full) => Ok(full.full_document_diagnostic_report.items),
        DocumentDiagnosticReport::Unchanged(_) => {
            Err("expected Full diagnostic report, got Unchanged".into())
        }
    }
}

/// Returns true when a diagnostic has the given code string (e.g. "PL701").
fn has_code(diag: &lsp_types::Diagnostic, code: &str) -> bool {
    matches!(&diag.code, Some(NumberOrString::String(s)) if s == code)
}

/// Scrub the result_id from a DocumentDiagnosticReport for stable snapshots.
fn scrubbed_report(report: &DocumentDiagnosticReport) -> serde_json::Value {
    match report {
        DocumentDiagnosticReport::Full(full) => {
            let result_id = full.full_document_diagnostic_report.result_id.clone();
            let items = &full.full_document_diagnostic_report.items;
            let mut map = serde_json::Map::new();
            map.insert("kind".to_string(), serde_json::json!("full"));
            if result_id.is_some() {
                // Replace with stable placeholder
                map.insert("resultId".to_string(), serde_json::json!("<stable-result-id>"));
            }
            map.insert("items".to_string(), serde_json::json!(items));
            serde_json::Value::Object(map)
        }
        DocumentDiagnosticReport::Unchanged(_) => {
            let mut map = serde_json::Map::new();
            map.insert("kind".to_string(), serde_json::json!("unchanged"));
            map.insert("resultId".to_string(), serde_json::json!("<stable-result-id>"));
            serde_json::Value::Object(map)
        }
    }
}

/// Scrub non-deterministic fields from a diagnostic for stable snapshots.
fn scrub_diagnostic(diag: &lsp_types::Diagnostic) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("code".to_string(), serde_json::json!(diag.code));
    map.insert("message".to_string(), serde_json::json!(diag.message));
    map.insert("severity".to_string(), serde_json::json!(diag.severity));
    map.insert("source".to_string(), serde_json::json!(diag.source));
    map.insert("range".to_string(), serde_json::json!(diag.range));
    if let Some(ref tags) = diag.tags {
        map.insert("tags".to_string(), serde_json::json!(tags));
    }
    if let Some(ref data) = diag.data {
        map.insert("data".to_string(), data.clone());
    }
    serde_json::Value::Object(map)
}

// ============================================================================
// PL701 @INC Path Inclusion Snapshots
//
// These tests verify that the PL701 (ModuleNotFound) diagnostic message
// includes the searched @INC paths when include_paths is provided.
// ============================================================================

#[test]
fn snapshot_pl701_with_include_paths() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_missing_module.pl".parse()?;
    // Use a non-core module that will definitely not be found
    let content = "use Missing::Module::That::Does::Not::Exist;\n";

    // Provide specific include paths that should appear in the PL701 message
    let include_paths = Some(vec!["/test/path1".to_string(), "/test/path2".to_string()]);

    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = items_from_report(report)?;

    // Find the PL701 diagnostic
    let pl701 = items
        .iter()
        .find(|d| has_code(d, "PL701"))
        .ok_or("Expected at least one PL701 (missing module) diagnostic")?;

    let snapshot_value = scrub_diagnostic(pl701);
    assert_yaml_snapshot!("pl701_with_include_paths", snapshot_value);
    Ok(())
}

#[test]
fn snapshot_pl701_without_include_paths() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_missing_module.pl".parse()?;
    let content = "use Missing::Module::That::Does::Not::Exist;\n";

    // Pass None for include_paths - should use fallback message
    let report = provider.get_document_diagnostics(&uri, content, None, None);
    let items = items_from_report(report)?;

    // Find the PL701 diagnostic
    let pl701 = items
        .iter()
        .find(|d| has_code(d, "PL701"))
        .ok_or("Expected at least one PL701 (missing module) diagnostic")?;

    let snapshot_value = scrub_diagnostic(pl701);
    assert_yaml_snapshot!("pl701_without_include_paths", snapshot_value);
    Ok(())
}

#[test]
fn snapshot_pl701_with_single_include_path() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_single_path.pl".parse()?;
    let content = "use Missing::Module;\n";

    let include_paths = Some(vec!["/custom/perl/lib".to_string()]);
    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = items_from_report(report)?;

    let pl701 = items
        .iter()
        .find(|d| has_code(d, "PL701"))
        .ok_or("Expected at least one PL701 (missing module) diagnostic")?;

    let snapshot_value = scrub_diagnostic(pl701);
    assert_yaml_snapshot!("pl701_with_single_include_path", snapshot_value);
    Ok(())
}

#[test]
fn snapshot_pl701_with_empty_include_paths() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_empty_paths.pl".parse()?;
    let content = "use Missing::Module;\n";

    // Empty vec is treated the same as None - shows fallback message
    let include_paths = Some(vec![]);
    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = items_from_report(report)?;

    let pl701 = items
        .iter()
        .find(|d| has_code(d, "PL701"))
        .ok_or("Expected at least one PL701 (missing module) diagnostic")?;

    let snapshot_value = scrub_diagnostic(pl701);
    assert_yaml_snapshot!("pl701_with_empty_include_paths", snapshot_value);
    Ok(())
}

#[test]
fn snapshot_pl701_unchanged_report_caching() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_caching.pl".parse()?;
    let content = "use Missing::Module;\n";
    let include_paths = Some(vec!["/test/path1".to_string()]);

    // First call - full report
    let first = provider.get_document_diagnostics(&uri, content, None, include_paths.clone());
    let result_id = match &first {
        DocumentDiagnosticReport::Full(full) => {
            full.full_document_diagnostic_report.result_id.clone()
        }
        DocumentDiagnosticReport::Unchanged(_) => {
            return Err("expected full report on first call".into());
        }
    };

    // Second call with same content and result_id - should get unchanged
    let second =
        provider.get_document_diagnostics(&uri, content, Some(result_id.unwrap()), include_paths);

    let snapshot_value = scrubbed_report(&second);
    assert_yaml_snapshot!("pl701_unchanged_report_caching", snapshot_value);
    Ok(())
}

#[test]
fn snapshot_pl701_full_report_then_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_full_then_unchanged.pl".parse()?;
    let content = "use Missing::Module;\n";
    let include_paths = Some(vec!["/test/path1".to_string()]);

    // First call - full report
    let first = provider.get_document_diagnostics(&uri, content, None, include_paths.clone());
    let first_scrubbed = scrubbed_report(&first);

    // Second call with same content - unchanged
    let result_id = match &first {
        DocumentDiagnosticReport::Full(full) => {
            full.full_document_diagnostic_report.result_id.clone()
        }
        DocumentDiagnosticReport::Unchanged(_) => {
            return Err("expected full report on first call".into());
        }
    };

    let second =
        provider.get_document_diagnostics(&uri, content, Some(result_id.unwrap()), include_paths);
    let second_scrubbed = scrubbed_report(&second);

    // Snapshot both reports together for clarity
    let mut combined = serde_json::Map::new();
    combined.insert("first_full".to_string(), first_scrubbed);
    combined.insert("second_unchanged".to_string(), second_scrubbed);

    assert_yaml_snapshot!("pl701_full_report_then_unchanged", serde_json::Value::Object(combined));
    Ok(())
}

#[test]
fn snapshot_pl701_diagnostic_data_structure() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test_data_structure.pl".parse()?;
    let content = "use Missing::Module;\n";
    let include_paths = Some(vec!["/test/path1".to_string(), "/test/path2".to_string()]);

    let report = provider.get_document_diagnostics(&uri, content, None, include_paths);
    let items = items_from_report(report)?;

    let pl701 = items
        .iter()
        .find(|d| has_code(d, "PL701"))
        .ok_or("Expected at least one PL701 (missing module) diagnostic")?;

    // Snapshot the full diagnostic including data field
    let mut map = serde_json::Map::new();
    map.insert("code".to_string(), serde_json::json!(pl701.code));
    map.insert("message".to_string(), serde_json::json!(pl701.message));
    map.insert("severity".to_string(), serde_json::json!(pl701.severity));
    map.insert("source".to_string(), serde_json::json!(pl701.source));
    map.insert("range".to_string(), serde_json::json!(pl701.range));
    map.insert("data".to_string(), pl701.data.clone().unwrap_or_default());

    assert_yaml_snapshot!("pl701_diagnostic_data_structure", serde_json::Value::Object(map));
    Ok(())
}
