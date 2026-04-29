//! Regression bank for import/export visibility fixtures.
//!
//! These cases lock down current Exporter extraction behavior so the upcoming
//! ImportSpec/ExportSet/VisibleSymbols layer can build deterministic semantics
//! on top of known exporter patterns.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::export_analyzer::{ExportInfo, ExportSymbolExtractor};
use perl_semantic_facts::Provenance;
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

    let all_tag = info
        .export_tags
        .get("all")
        .ok_or("missing expected :all export tag")?;
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
fn non_exporter_module_with_export_arrays_is_not_treated_as_export_source() -> Result<(), Box<dyn Error>> {
    let code = r#"
package NotExporter;
our @EXPORT = qw(fake_default);
our @EXPORT_OK = qw(fake_optional);
1;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let info = ExportSymbolExtractor::extract(&ast);
    assert!(
        info.is_none(),
        "module without Exporter inheritance must not produce export info"
    );
    Ok(())
}

#[test]
fn export_set_adapter_captures_default_optional_and_tags() -> Result<(), Box<dyn Error>> {
    let code = r#"
package AdapterFixture;
use Exporter 'import';
our @EXPORT = qw(default_one);
our @EXPORT_OK = qw(optional_one optional_two);
our %EXPORT_TAGS = (
  all => [qw(default_one optional_one optional_two)],
  opt => [qw(optional_one optional_two)],
);
1;
"#;

    let info = extract_export_info(code)?;
    let export_set = info.to_export_set();

    assert!(export_set.default_exports.contains("default_one"));
    assert!(export_set.optional_exports.contains("optional_one"));
    assert!(export_set.optional_exports.contains("optional_two"));
    assert_eq!(export_set.provenance, Provenance::ImportExportInference);

    let all_tag = export_set.export_tags.get("all").ok_or("missing :all tag")?;
    assert!(all_tag.contains("default_one"));
    assert!(all_tag.contains("optional_one"));
    assert!(all_tag.contains("optional_two"));
    Ok(())
}

#[test]
fn export_set_adapter_supports_parent_exporter_inheritance_form() -> Result<(), Box<dyn Error>> {
    let code = r#"
package ParentAdapter;
use parent 'Exporter';
our @EXPORT = qw(alpha);
our @EXPORT_OK = qw(beta);
our %EXPORT_TAGS = (
  both => [qw(alpha beta)],
);
1;
"#;

    let info = extract_export_info(code)?;
    let export_set = info.to_export_set();
    assert!(export_set.default_exports.contains("alpha"));
    assert!(export_set.optional_exports.contains("beta"));
    assert!(export_set.export_tags.contains_key("both"));
    Ok(())
}
