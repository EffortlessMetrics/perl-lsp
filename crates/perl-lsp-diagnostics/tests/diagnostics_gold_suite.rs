use perl_corpus::{load_gold_fixtures, GoldAssertion};
use perl_lsp_diagnostics::DiagnosticsProvider;
use perl_parser::Parser;
use std::fs;
use std::path::Path;
use std::sync::Arc;

fn validate_fixture(
    _fixture_name: &str,
    perl_code: &str,
    expected: &perl_corpus::GoldExpected,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(perl_code);
    let parse_output = parser.parse_with_recovery();

    let ast = Arc::new(parse_output.ast.clone());
    let provider = DiagnosticsProvider::new(&ast, perl_code.to_string());

    let diagnostics = provider.get_diagnostics(&ast, &parse_output.diagnostics, perl_code, None);

    let mut all_passed = true;
    let mut report = String::new();

    for assertion in &expected.diagnostics {
        match assertion {
            GoldAssertion::NoDiagnostics => {
                if !diagnostics.is_empty() {
                    all_passed = false;
                    report.push_str(&format!(
                        "  FAIL: Expected no diagnostics, but got {}:\n",
                        diagnostics.len()
                    ));
                    for diag in &diagnostics {
                        let code_str = diag.code.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
                        report.push_str(&format!("    - [{}] {}\n", code_str, diag.message));
                    }
                } else {
                    report.push_str("  PASS: No diagnostics emitted\n");
                }
            }

            GoldAssertion::NoDiagnostic { code } => {
                let has_code = diagnostics.iter().any(|d| d.code.as_ref() == Some(code));
                if has_code {
                    all_passed = false;
                    report.push_str(&format!(
                        "  FAIL: Diagnostic {} should not be present\n",
                        code
                    ));
                } else {
                    report.push_str(&format!("  PASS: Diagnostic {} not present\n", code));
                }
            }

            GoldAssertion::DiagnosticPresent {
                code,
                byte_offset,
                message_contains,
            } => {
                let found = diagnostics.iter().find(|d| {
                    d.code.as_ref() == Some(code)
                        && byte_offset.map(|off| d.range.0 == off).unwrap_or(true)
                        && message_contains
                            .as_ref()
                            .map(|msg| d.message.contains(msg))
                            .unwrap_or(true)
                });

                if let Some(diag) = found {
                    report.push_str(&format!(
                        "  PASS: Diagnostic {} present at byte {}\n",
                        code, diag.range.0
                    ));
                } else {
                    all_passed = false;
                    let found_codes: Vec<_> =
                        diagnostics.iter().filter_map(|d| d.code.as_ref()).collect();
                    report.push_str(&format!(
                        "  FAIL: Expected diagnostic {} not found. Found: {:?}\n",
                        code, found_codes
                    ));
                }
            }

            GoldAssertion::DiagnosticCount { code, count } => {
                let actual_count =
                    diagnostics.iter().filter(|d| d.code.as_ref() == Some(code)).count();
                if actual_count == *count {
                    report.push_str(&format!("  PASS: {} diagnostics with code {}\n", count, code));
                } else {
                    all_passed = false;
                    report.push_str(&format!(
                        "  FAIL: Expected {} diagnostics with code {}, got {}\n",
                        count, code, actual_count
                    ));
                }
            }
        }
    }

    Ok((all_passed, report))
}

#[test]
fn gold_fixtures_diagnostics_suite() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root");

    let gold_dir = workspace_root.join("test_corpus").join("gold");

    let fixtures = match load_gold_fixtures(&gold_dir) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to load gold fixtures: {}", e);
            eprintln!("Looked in: {}", gold_dir.display());
            return;
        }
    };

    if fixtures.is_empty() {
        eprintln!("Warning: No gold fixtures found in {}", gold_dir.display());
        return;
    }

    eprintln!("\n=== Gold Corpus Diagnostics Test Suite ===");
    eprintln!("Testing {} fixtures\n", fixtures.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut detail_reports = Vec::new();

    for fixture in fixtures {
        let perl_code = match fs::read_to_string(&fixture.fixture_path) {
            Ok(code) => code,
            Err(e) => {
                failed += 1;
                detail_reports.push((
                    fixture.name.clone(),
                    false,
                    format!("ERROR: Could not read fixture file: {}\n", e),
                ));
                continue;
            }
        };

        match validate_fixture(&fixture.name, &perl_code, &fixture.expected) {
            Ok((success, report)) => {
                if success {
                    passed += 1;
                    detail_reports.push((fixture.name.clone(), true, report));
                } else {
                    failed += 1;
                    detail_reports.push((fixture.name.clone(), false, report));
                }
            }
            Err(e) => {
                failed += 1;
                detail_reports.push((fixture.name.clone(), false, format!("ERROR: {}\n", e)));
            }
        }
    }

    for (name, success, report) in detail_reports {
        let status = if success { "PASS" } else { "FAIL" };
        eprintln!("{}: {}", status, name);
        eprintln!("{}", report);
    }

    eprintln!("\n=== Summary ===");
    eprintln!("Passed: {}", passed);
    eprintln!("Failed: {}", failed);
    eprintln!("Total:  {}", passed + failed);

    if failed > 0 {
        eprintln!(
            "\nPrecision: {:.1}%",
            (passed as f64 / (passed + failed) as f64) * 100.0
        );
    }

    assert_eq!(
        failed, 0,
        "Gold corpus diagnostics suite failed: {} failures",
        failed
    );
}
