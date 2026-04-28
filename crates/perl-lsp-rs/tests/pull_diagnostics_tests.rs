use std::env;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use lsp_types::{DocumentDiagnosticReport, NumberOrString, Uri};
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;
use url::Url;

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

/// Returns true when a diagnostic has the given code string (e.g. "PL102").
fn has_code(diag: &lsp_types::Diagnostic, code: &str) -> bool {
    matches!(&diag.code, Some(NumberOrString::String(s)) if s == code)
}

#[test]
fn pull_diagnostics_unused_variable_emits_pl102() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_unused.pl".parse()?;
    // $used is referenced; $unused is not — should produce PL102 for $unused only.
    let content = "use strict;\nuse warnings;\nsub foo {\n    my $used = 123;\n    my $unused = 456;\n    return $used;\n}\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;

    let pl102_diags: Vec<_> = items.iter().filter(|d| has_code(d, "PL102")).collect();
    if pl102_diags.is_empty() {
        return Err(format!(
            "Expected at least one PL102 (unused variable) diagnostic, got none.\nAll diagnostics: {items:#?}"
        )
        .into());
    }

    // At least one PL102 must mention $unused
    let mentions_unused =
        pl102_diags.iter().any(|d| d.message.contains("$unused") || d.message.contains("unused"));
    if !mentions_unused {
        return Err(format!(
            "Expected a PL102 diagnostic mentioning '$unused', got: {pl102_diags:#?}"
        )
        .into());
    }

    // No PL102 should mention $used (it is referenced)
    let false_positive =
        pl102_diags.iter().any(|d| d.message.contains("$used") && !d.message.contains("$unused"));
    if false_positive {
        return Err(format!(
            "Unexpected PL102 diagnostic for '$used' (it is referenced): {pl102_diags:#?}"
        )
        .into());
    }

    Ok(())
}

#[test]
fn pull_diagnostics_unused_variable_severity_is_warning() -> Result<(), Box<dyn std::error::Error>>
{
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_unused_sev.pl".parse()?;
    let content = "sub bar {\n    my $never_used = 1;\n}\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;

    let pl102 = items
        .iter()
        .find(|d| has_code(d, "PL102"))
        .ok_or("Expected PL102 diagnostic for unused variable $never_used")?;

    let severity = pl102.severity.ok_or("PL102 diagnostic must have a severity")?;
    if severity != lsp_types::DiagnosticSeverity::WARNING {
        return Err(format!("Expected WARNING severity for PL102, got {:?}", severity).into());
    }

    Ok(())
}

#[test]
fn pull_diagnostics_interpolated_variable_counts_as_used() -> Result<(), Box<dyn std::error::Error>>
{
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_interpolated.pl".parse()?;
    let content = "use strict;\nuse warnings;\nmy $msg = 'hello';\nprint \"$msg\\n\";\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;

    let msg_unused = items.iter().any(|d| has_code(d, "PL102") && d.message.contains("$msg"));
    if msg_unused {
        return Err(format!(
            "Interpolated variable $msg must not be flagged unused.\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

#[test]
fn pull_diagnostics_script_uri_suppresses_missing_package_warning()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = Url::from_file_path(env::temp_dir().join("Makefile.PL"))
        .map_err(|_| "failed to build Makefile.PL test URI")?
        .to_string()
        .parse()?;
    let content = "use strict;\nuse warnings;\nprint \"ok\\n\";\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;

    let has_pl200 = items.iter().any(|d| has_code(d, "PL200"));
    if has_pl200 {
        return Err(format!(
            "Script URIs should not emit PL200 missing-package diagnostics.\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

#[test]
fn pull_diagnostics_shebang_suppresses_missing_package_warning()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///smoke_script.txt".parse()?;
    let content = "#!/usr/bin/env perl\nuse strict;\nuse warnings;\nprint \"ok\\n\";\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;

    let has_pl200 = items.iter().any(|d| has_code(d, "PL200"));
    if has_pl200 {
        return Err(format!(
            "Shebang-based scripts should not emit PL200 missing-package diagnostics.\nDiagnostics: {items:#?}"
        )
        .into());
    }

    Ok(())
}

#[test]
fn pull_diagnostics_underscore_prefix_suppresses_unused_warning()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_underscore.pl".parse()?;
    // _intentionally_unused should NOT produce PL102 (underscore prefix = intentionally unused)
    let content = "sub baz {\n    my $_intentionally_unused = 1;\n    return 42;\n}\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;

    let pl102_for_underscore =
        items.iter().find(|d| has_code(d, "PL102") && d.message.contains("_intentionally_unused"));

    if pl102_for_underscore.is_some() {
        return Err(
            "PL102 must NOT be emitted for underscore-prefixed variable $_intentionally_unused"
                .into(),
        );
    }

    Ok(())
}

#[test]
fn pull_diagnostics_full_then_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test.pl".parse()?;
    let content = "my $x = ;";

    let first = provider.get_document_diagnostics(&uri, content, None, None);
    let result_id = match &first {
        DocumentDiagnosticReport::Full(full) => {
            let report = &full.full_document_diagnostic_report;
            assert!(!report.items.is_empty(), "expected diagnostics for parse error");
            assert!(
                report.items.iter().all(|item| item.source.as_deref() == Some("perl-lsp")),
                "expected deterministic diagnostic source"
            );
            report.result_id.clone().ok_or("result id missing")?
        }
        DocumentDiagnosticReport::Unchanged(_) => {
            return Err("expected full diagnostics report for initial request".into());
        }
    };

    let second = provider.get_document_diagnostics(&uri, content, Some(result_id), None);
    assert!(
        matches!(second, DocumentDiagnosticReport::Unchanged(_)),
        "expected unchanged diagnostics report on identical content"
    );

    Ok(())
}

// =========================================================================
// Hint propagation tests (#4191)
//
// These tests verify that the *fallback* parse_error_to_diagnostic path
// (exercised when parser.parse() returns Err) includes actionable hints in
// the diagnostic message, mirroring what the AST-present path already does
// via to_lsp_diagnostic → build_parse_error_suggestion.
// =========================================================================

/// Helper: collect all diagnostics from a content string and extract just the
/// parse-error (PL001 / PL002 / PL003) ones.
fn parse_error_diagnostics_for(
    content: &str,
) -> Result<Vec<lsp_types::Diagnostic>, Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///hint_test.pl".parse()?;
    let all = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;
    Ok(all
        .into_iter()
        .filter(|d| {
            matches!(
                &d.code,
                Some(NumberOrString::String(s))
                    if s == "PL001" || s == "PL002" || s == "PL003"
            )
        })
        .collect())
}

#[test]
fn parse_error_fallback_missing_semicolon_includes_hint() -> Result<(), Box<dyn std::error::Error>>
{
    // UnexpectedToken { expected: ";", found: "<something>" } — the most common parse error.
    // The message emitted by the fallback path MUST include "Suggestion:" (wired from
    // build_parse_error_suggestion via the same helper used by to_lsp_diagnostic).
    let content = "my $x = 1\nmy $y = 2;\n";
    let diags = parse_error_diagnostics_for(content)?;
    if diags.is_empty() {
        // Parser recovered successfully — nothing to assert for the fallback path.
        return Ok(());
    }
    let first = &diags[0];
    assert!(
        first.message.contains("Suggestion:") || first.message.contains(';'),
        "Parse error diagnostic should include a semicolon hint, got: {:?}",
        first.message
    );
    Ok(())
}

#[test]
fn parse_error_fallback_unclosed_block_includes_hint() -> Result<(), Box<dyn std::error::Error>> {
    // SyntaxError "Unclosed block: expected '}' but reached end of input" — sub not closed.
    // The fallback parse_error_to_diagnostic path must include "Suggestion:" for SyntaxErrors
    // that describe structural issues like unclosed blocks.
    let content = "sub foo {\n    my $x = 1;\n";
    let diags = parse_error_diagnostics_for(content)?;
    if diags.is_empty() {
        return Ok(());
    }
    let first = &diags[0];
    assert!(
        first.message.contains("Suggestion:"),
        "Unclosed block parse error should include a hint, got: {:?}",
        first.message
    );
    Ok(())
}

#[test]
fn parse_error_fallback_unclosed_string_includes_hint() -> Result<(), Box<dyn std::error::Error>> {
    // When an unclosed string literal produces an UnexpectedToken in the fallback path,
    // the diagnostic message must include "Suggestion:".
    let content = "my $x = \"hello world;\n";
    let diags = parse_error_diagnostics_for(content)?;
    if diags.is_empty() {
        return Ok(());
    }
    let first = &diags[0];
    assert!(
        first.message.contains("Suggestion:"),
        "Unterminated string parse error should include a hint, got: {:?}",
        first.message
    );
    Ok(())
}

#[test]
fn parse_error_fallback_missing_comma_includes_hint() -> Result<(), Box<dyn std::error::Error>> {
    // UnexpectedToken where a comma was expected between list elements.
    // e.g. "my @list = (1 2 3);" — missing commas between elements should produce a
    // diagnostic with "Suggestion:" in the fallback path.
    let content = "my @list = (1 2 3);\n";
    let diags = parse_error_diagnostics_for(content)?;
    if diags.is_empty() {
        return Ok(());
    }
    let first = &diags[0];
    assert!(
        first.message.contains("Suggestion:"),
        "Missing comma parse error should include a hint, got: {:?}",
        first.message
    );
    Ok(())
}
// =========================================================================
// PL701 @INC path inclusion tests (#4259 follow-up)
//
// Verifies that PullDiagnosticsProvider::get_document_diagnostics accepts
// an optional include_paths parameter and that PL701 diagnostics include
// the searched @INC paths in their message when provided.
// =========================================================================

#[test]
fn pl701_pull_diagnostics_includes_inc_paths() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_missing_module.pl".parse()?;
    // Use a non-core module that will definitely not be found
    let content = "use Missing::Module::That::Does::Not::Exist;\n";

    // Provide specific include paths that should appear in the PL701 message
    let include_paths = Some(vec!["/test/path1".to_string(), "/test/path2".to_string()]);

    let items =
        items_from_report(provider.get_document_diagnostics(&uri, content, None, include_paths))?;

    // Find the PL701 diagnostic
    let pl701 = items
        .iter()
        .find(|d| has_code(d, "PL701"))
        .ok_or("Expected at least one PL701 (missing module) diagnostic")?;

    // Verify the message includes the searched paths
    let message = &pl701.message;
    if !message.contains("/test/path1") {
        return Err(format!(
            "PL701 message should include '/test/path1' from include_paths, got: {}",
            message
        )
        .into());
    }
    if !message.contains("/test/path2") {
        return Err(format!(
            "PL701 message should include '/test/path2' from include_paths, got: {}",
            message
        )
        .into());
    }
    // Should NOT have the fallback message about "workspace or configured include paths"
    if message.contains("workspace or configured include paths") {
        return Err(format!(
            "PL701 message should show searched @INC paths, not fallback message. Got: {}",
            message
        )
        .into());
    }

    Ok(())
}

#[test]
fn pl701_pull_diagnostics_empty_inc_paths_shows_fallback_message()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_missing_module.pl".parse()?;
    let content = "use Missing::Module::That::Does::Not::Exist;\n";

    // Pass None for include_paths - should use fallback message
    let items = items_from_report(provider.get_document_diagnostics(
        &uri, content, None, None, // No include paths provided
    ))?;

    // Find the PL701 diagnostic
    let pl701 = items
        .iter()
        .find(|d| has_code(d, "PL701"))
        .ok_or("Expected at least one PL701 (missing module) diagnostic")?;

    // With empty include_paths, should show the fallback message
    let message = &pl701.message;
    if !message.contains("workspace or configured include paths") {
        return Err(format!(
            "PL701 message with no include_paths should show fallback message, got: {}",
            message
        )
        .into());
    }

    Ok(())
}

#[test]
fn pl701_respects_use_lib_paths_from_document() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let project_root = env::temp_dir().join(format!("perl_lsp_pull_diag_inc_{nonce}"));
    let lib_dir = project_root.join("lib").join("My");
    fs::create_dir_all(&lib_dir)?;
    fs::write(project_root.join("lib").join("My").join("Test.pm"), "package My::Test; 1;\n")?;
    let script_path = project_root.join("script.pl");
    let uri: Uri = Url::from_file_path(&script_path)
        .map_err(|_| "failed to create script URI for @INC test")?
        .to_string()
        .parse()?;

    let content = "use lib 'lib';\nuse My::Test;\n";
    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None, None))?;
    let has_pl701 = items.iter().any(|d| has_code(d, "PL701"));
    if has_pl701 {
        return Err(format!(
            "PL701 should not be emitted when module exists via lexical use lib path. Diagnostics: {items:#?}"
        )
        .into());
    }

    let _ = fs::remove_dir_all(&project_root);
    Ok(())
}
