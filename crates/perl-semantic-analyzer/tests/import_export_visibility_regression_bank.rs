//! Regression bank for import/export visibility substrate fixtures.
//! These tests lock down expected export-set extraction semantics for
//! upcoming ImportSpec/ExportSet/VisibleSymbols integration work.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::export_analyzer::ExportSymbolExtractor;
use std::collections::HashSet;

fn hs(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn extract(code: &str) -> Result<perl_semantic_analyzer::analysis::export_analyzer::ExportInfo, String> {
    let mut parser = Parser::new(code);
    let ast = parser.parse().map_err(|e| format!("parse failed: {e:?}"))?;
    ExportSymbolExtractor::extract(&ast).ok_or_else(|| "not detected as exporter-based module".to_string())
}

#[test]
fn exporter_fixture_baseline_default_optional_and_tags() -> Result<(), String> {
    let code = r#"
package MyLib;
use Exporter 'import';
our @EXPORT = qw(foo);
our @EXPORT_OK = qw(bar baz);
our %EXPORT_TAGS = (
  all => [qw(foo bar baz)],
);
1;
"#;

    let info = extract(code)?;
    assert_eq!(info.default_export, hs(&["foo"]));
    assert_eq!(info.optional_export, hs(&["bar", "baz"]));

    let all = info.export_tags.get("all").ok_or_else(|| "missing :all tag".to_string())?;
    assert_eq!(all, &vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]);
    Ok(())
}

#[test]
fn exporter_fixture_parent_inheritance_and_assignment_forms() -> Result<(), String> {
    let code = r#"
package MyParented;
use parent qw(Exporter);
@EXPORT = qw(alpha);
@EXPORT_OK = qw(beta gamma);
%EXPORT_TAGS = ( core => [qw(alpha beta)] );
1;
"#;

    let info = extract(code)?;
    assert_eq!(info.default_export, hs(&["alpha"]));
    assert_eq!(info.optional_export, hs(&["beta", "gamma"]));

    let core = info.export_tags.get("core").ok_or_else(|| "missing :core tag".to_string())?;
    assert_eq!(core, &vec!["alpha".to_string(), "beta".to_string()]);
    Ok(())
}

#[test]
fn exporter_fixture_detects_without_export_lists() -> Result<(), String> {
    let code = r#"
package NoLists;
use Exporter;
1;
"#;

    let info = extract(code)?;
    assert!(info.default_export.is_empty());
    assert!(info.optional_export.is_empty());
    assert!(info.export_tags.is_empty());
    Ok(())
}
