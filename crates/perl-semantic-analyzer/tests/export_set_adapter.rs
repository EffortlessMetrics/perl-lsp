use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::export_analyzer::ExportSymbolExtractor;
use perl_semantic_facts::Provenance;
use std::error::Error;

fn extract_export_set(code: &str) -> Result<perl_semantic_facts::ExportSet, Box<dyn Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let info = ExportSymbolExtractor::extract(&ast)
        .ok_or_else(|| "expected Exporter-based module, got None".to_string())?;
    Ok(info.to_export_set())
}

#[test]
fn export_array_maps_to_default_exports() -> Result<(), Box<dyn Error>> {
    let set = extract_export_set(
        r#"
package DefaultOnly;
use Exporter 'import';
our @EXPORT = qw(foo bar);
1;
"#,
    )?;

    assert_eq!(set.default_exports, vec!["bar".to_string(), "foo".to_string()]);
    assert!(set.optional_exports.is_empty());
    assert!(set.tags.is_empty());
    assert_eq!(set.provenance, Provenance::ImportExportInference);
    Ok(())
}

#[test]
fn export_ok_maps_to_optional_exports() -> Result<(), Box<dyn Error>> {
    let set = extract_export_set(
        r#"
package OptionalOnly;
use Exporter 'import';
our @EXPORT_OK = qw(beta alpha);
1;
"#,
    )?;

    assert!(set.default_exports.is_empty());
    assert_eq!(set.optional_exports, vec!["alpha".to_string(), "beta".to_string()]);
    Ok(())
}

#[test]
fn export_tags_map_to_tag_membership() -> Result<(), Box<dyn Error>> {
    let set = extract_export_set(
        r#"
package Tagged;
use Exporter 'import';
our %EXPORT_TAGS = (
  all => [qw(foo bar)],
  io => [qw(read write read)],
);
1;
"#,
    )?;

    assert_eq!(set.tags.len(), 2);
    assert_eq!(set.tags[0].name, "all");
    assert_eq!(set.tags[0].members, vec!["bar".to_string(), "foo".to_string()]);
    assert_eq!(set.tags[1].name, "io");
    assert_eq!(set.tags[1].members, vec!["read".to_string(), "write".to_string()]);
    Ok(())
}

#[test]
fn parent_exporter_inheritance_still_produces_export_set() -> Result<(), Box<dyn Error>> {
    let set = extract_export_set(
        r#"
package ParentBased;
use parent 'Exporter';
our @EXPORT = qw(core_symbol);
our @EXPORT_OK = qw(extra_symbol);
1;
"#,
    )?;

    assert_eq!(set.default_exports, vec!["core_symbol".to_string()]);
    assert_eq!(set.optional_exports, vec!["extra_symbol".to_string()]);
    Ok(())
}

#[test]
fn dynamic_exporter_forms_are_conservative() -> Result<(), Box<dyn Error>> {
    let set = extract_export_set(
        r#"
package DynamicStyle;
use Exporter 'import';
our @EXPORT = @{ build_exports() };
1;
"#,
    )?;

    assert!(set.default_exports.is_empty());
    assert!(set.optional_exports.is_empty());
    assert!(set.tags.is_empty());
    Ok(())
}
