//! Regression bank for import/export visibility fixtures.
//!
//! These cases lock down current Exporter extraction behavior so the upcoming
//! ImportSpec/ExportSet/VisibleSymbols layer can build deterministic semantics
//! on top of known exporter patterns.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::export_analyzer::{ExportInfo, ExportSymbolExtractor};
use std::error::Error;

fn extract_export_info(code: &str) -> Result<ExportInfo, Box<dyn Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    ExportSymbolExtractor::extract(&ast)
        .ok_or_else(|| "expected Exporter-based module, got None".into())
}

#[test]
fn exporter_import_with_export_ok_and_tags_is_stable_fixture() -> Result<(), Box<dyn Error>> {
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

    let info = extract_export_info(code)?;
    assert!(info.default_export.contains("foo"));
    assert!(info.optional_export.contains("bar"));
    assert!(info.optional_export.contains("baz"));

    let all_tag = info.export_tags.get("all").ok_or("missing expected :all export tag")?;
    assert!(all_tag.iter().any(|symbol| symbol == "foo"));
    assert!(all_tag.iter().any(|symbol| symbol == "bar"));
    assert!(all_tag.iter().any(|symbol| symbol == "baz"));
    Ok(())
}

#[test]
fn parent_exporter_fixture_keeps_default_and_optional_sets() -> Result<(), Box<dyn Error>> {
    let code = r#"
package ParentStyle;
use parent 'Exporter';
our @EXPORT = qw(alpha);
our @EXPORT_OK = qw(beta gamma);
1;
"#;

    let info = extract_export_info(code)?;
    assert_eq!(info.default_export.len(), 1);
    assert!(info.default_export.contains("alpha"));
    assert_eq!(info.optional_export.len(), 2);
    assert!(info.optional_export.contains("beta"));
    assert!(info.optional_export.contains("gamma"));
    Ok(())
}

#[test]
fn non_exporter_module_with_export_arrays_is_not_treated_as_export_source()
-> Result<(), Box<dyn Error>> {
    let code = r#"
package NotExporter;
our @EXPORT = qw(fake_default);
our @EXPORT_OK = qw(fake_optional);
1;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let info = ExportSymbolExtractor::extract(&ast);
    assert!(info.is_none(), "module without Exporter inheritance must not produce export info");
    Ok(())
}
