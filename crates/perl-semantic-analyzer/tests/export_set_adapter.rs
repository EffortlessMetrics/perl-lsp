use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::export_analyzer::ExportSymbolExtractor;
use perl_semantic_facts::Provenance;
use std::error::Error;

fn export_set_from(code: &str) -> Result<perl_semantic_facts::ExportSet, Box<dyn Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    ExportSymbolExtractor::extract_export_set(&ast)
        .ok_or_else(|| "expected exporter-backed module".into())
}

#[test]
fn export_set_captures_export_and_export_ok() -> Result<(), Box<dyn Error>> {
    let code = r#"
package MyLib;
use Exporter 'import';
our @EXPORT = qw(foo);
our @EXPORT_OK = qw(bar baz);
1;
"#;

    let export_set = export_set_from(code)?;
    assert!(export_set.default_exports.contains("foo"));
    assert!(export_set.optional_exports.contains("bar"));
    assert!(export_set.optional_exports.contains("baz"));
    assert_eq!(export_set.provenance, Provenance::ImportExportInference);
    Ok(())
}

#[test]
fn export_set_captures_export_tags_membership() -> Result<(), Box<dyn Error>> {
    let code = r#"
package Color;
use Exporter 'import';
our %EXPORT_TAGS = (
  primary => [qw(red green blue)],
);
1;
"#;

    let export_set = export_set_from(code)?;
    let primary = export_set
        .tag_exports
        .get("primary")
        .ok_or("missing primary tag")?;
    assert!(primary.contains("red"));
    assert!(primary.contains("green"));
    assert!(primary.contains("blue"));
    Ok(())
}

#[test]
fn export_set_detects_parent_exporter_form() -> Result<(), Box<dyn Error>> {
    let code = r#"
package ParentStyle;
use parent 'Exporter';
our @EXPORT_OK = qw(alpha);
1;
"#;

    let export_set = export_set_from(code)?;
    assert!(export_set.optional_exports.contains("alpha"));
    Ok(())
}

#[test]
fn export_set_is_absent_for_dynamic_or_non_exporter_forms() -> Result<(), Box<dyn Error>> {
    let code = r#"
package Dynamic;
our $maybe = 'Exporter';
our @ISA = ($maybe);
our @EXPORT = qw(fake);
1;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let export_set = ExportSymbolExtractor::extract_export_set(&ast);
    assert!(
        export_set.is_none(),
        "dynamic @ISA form should remain conservatively unsupported"
    );
    Ok(())
}
