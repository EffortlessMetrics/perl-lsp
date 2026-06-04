//! Parser end-to-end coverage for gold LSP fixtures.
//!
//! These tests keep the parser wired to the same fixture corpus consumed by
//! higher-level editor-feature tests, so gold scenarios cannot drift into
//! unparsed or malformed Perl without a parser-facing signal.

use perl_parser::{NodeKind, Parser};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct GoldFixture {
    name: String,
    dir: PathBuf,
    expected: Option<Value>,
}

fn gold_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_corpus/gold")
}

fn discover_fixture_dirs() -> Result<Vec<GoldFixture>, Box<dyn std::error::Error>> {
    let root = gold_root();
    let entries = fs::read_dir(&root)?;
    let mut fixtures = Vec::new();

    for entry in entries {
        let entry = entry?;
        let dir = entry.path();
        if !dir.is_dir() || !dir.join("fixture.pl").exists() {
            continue;
        }

        let expected_path = dir.join("expected.json");
        let name = path_file_name(&dir)?;
        let expected = if expected_path.exists() {
            let expected_source = fs::read_to_string(&expected_path)?;
            Some(serde_json::from_str(&expected_source)?)
        } else {
            None
        };
        fixtures.push(GoldFixture { name, dir, expected });
    }

    fixtures.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(fixtures)
}

fn path_file_name(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let Some(name) = path.file_name().and_then(|part| part.to_str()) else {
        return Err(format!("path has no UTF-8 file name: {}", path.display()).into());
    };
    Ok(name.to_string())
}

fn collect_perl_sources(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_perl_sources_rec(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_perl_sources_rec(
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_perl_sources_rec(&path, files)?;
            continue;
        }

        let Some(ext) = path.extension().and_then(|part| part.to_str()) else {
            continue;
        };
        if matches!(ext, "pl" | "pm") {
            files.push(path);
        }
    }
    Ok(())
}

fn expected_syntax_diagnostic(expected: &Value) -> bool {
    let Some(diagnostics) = expected.get("diagnostics").and_then(Value::as_array) else {
        return false;
    };

    diagnostics.iter().any(|diagnostic| {
        diagnostic.get("assertion").and_then(Value::as_str) == Some("diagnostic_present")
            && diagnostic.get("code").and_then(Value::as_str) == Some("PL001")
    })
}

fn assert_clean_parse(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let mut parser = Parser::new(&source);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();

    assert!(
        !sexp.contains("ERROR"),
        "expected clean parser e2e fixture without ERROR nodes: {}\n{sexp}",
        path.display()
    );
    Ok(())
}

fn assert_recovery_parse(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let mut parser = Parser::new(&source);
    let output = parser.parse_with_recovery();

    assert!(
        matches!(output.ast.kind, NodeKind::Program { .. }),
        "expected recovery parse to still return a Program AST for {}",
        path.display()
    );
    assert!(
        !output.diagnostics.is_empty() || output.ast.to_sexp().contains("ERROR"),
        "expected syntax-error fixture to expose diagnostics or ERROR nodes: {}",
        path.display()
    );
    Ok(())
}

#[test]
fn gold_fixture_manifests_are_parser_e2e_ready() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = discover_fixture_dirs()?;
    assert!(!fixtures.is_empty(), "expected at least one gold parser e2e fixture");

    for fixture in &fixtures {
        let Some(expected) = &fixture.expected else {
            continue;
        };
        let diagnostics = expected
            .get("diagnostics")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{} expected.json must contain diagnostics[]", fixture.name))?;
        assert!(
            !diagnostics.is_empty(),
            "{} expected.json must include at least one diagnostic assertion",
            fixture.name
        );

        for diagnostic in diagnostics {
            let assertion = diagnostic
                .get("assertion")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{} diagnostic assertion must be a string", fixture.name))?;
            assert!(
                matches!(assertion, "no_diagnostics" | "no_diagnostic" | "diagnostic_present"),
                "{} has unsupported diagnostic assertion kind: {}",
                fixture.name,
                assertion
            );
        }
    }

    Ok(())
}

#[test]
fn gold_fixture_sources_parse_to_expected_syntax_status() -> Result<(), Box<dyn std::error::Error>>
{
    let fixtures = discover_fixture_dirs()?;
    assert!(!fixtures.is_empty(), "expected at least one gold parser e2e fixture");

    for fixture in &fixtures {
        let syntax_error_expected =
            fixture.expected.as_ref().is_some_and(expected_syntax_diagnostic);
        let fixture_path = fixture.dir.join("fixture.pl");

        if syntax_error_expected {
            assert_recovery_parse(&fixture_path)?;
        } else {
            assert_clean_parse(&fixture_path)?;
        }

        for source_path in collect_perl_sources(&fixture.dir)? {
            if source_path == fixture_path {
                continue;
            }
            assert_clean_parse(&source_path)?;
        }
    }

    Ok(())
}
