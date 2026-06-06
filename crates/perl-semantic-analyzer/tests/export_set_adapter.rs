use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::export_analyzer::ExportSymbolExtractor;
use perl_semantic_facts::{Confidence, Provenance};
use perl_tdd_support::must;
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

#[test]
fn regression_merges_export_assignments_across_statements() -> Result<(), Box<dyn Error>> {
    // Real CPAN modules often build @EXPORT_OK and %EXPORT_TAGS incrementally
    // across multiple assignment statements. The adapter must merge them correctly.
    let set = extract_export_set(
        r#"
package MyLib;
use Exporter 'import';
our @EXPORT = qw(foo);
our @EXPORT_OK = qw(bar);
our @EXPORT_OK = qw(bar baz);
our %EXPORT_TAGS = (core => [qw(foo bar)]);
our %EXPORT_TAGS = (all => [qw(foo bar baz)]);
1;
"#,
    )?;

    assert_eq!(set.default_exports, vec!["foo".to_string()]);
    assert_eq!(set.optional_exports, vec!["bar".to_string(), "baz".to_string()]);

    let core = set
        .tags
        .iter()
        .find(|t| t.name == "core")
        .ok_or_else(|| "core tag must exist".to_string())?;
    assert_eq!(core.members, vec!["bar".to_string(), "foo".to_string()]);

    let all = set
        .tags
        .iter()
        .find(|t| t.name == "all")
        .ok_or_else(|| "all tag must exist".to_string())?;
    assert_eq!(all.members, vec!["bar".to_string(), "baz".to_string(), "foo".to_string()]);

    Ok(())
}

#[test]
fn custom_import_sub_produces_unknown_exports() {
    let result = extract_export_set(
        r#"
package CustomExporter;
sub import { my ($pkg, @args) = @_; my $caller = caller(0);
  foreach my $name (@args) { no strict 'refs'; *{"${caller}::${name}"} = \&$name; } }
sub func1 { "exported" }
1;
"#,
    );
    assert!(result.is_ok(), "custom import fixture should produce an export set");
    let set = must(result);
    assert!(set.default_exports.is_empty());
    assert!(set.optional_exports.is_empty());
    assert!(set.tags.is_empty());
    assert_eq!(set.confidence, Confidence::Low);
    assert_eq!(set.provenance, Provenance::ImportExportInference);
}

#[test]
fn regression_exporter_not_confused_with_custom_import() {
    let result = extract_export_set(
        r#"
package HybridModule;
use Exporter;
our @EXPORT = qw(exported);
sub import { my ($pkg, @args) = @_; $pkg->SUPER::import(@args); }
1;
"#,
    );
    assert!(result.is_ok(), "Exporter fixture with custom import should produce an export set");
    let set = must(result);
    assert_eq!(set.default_exports, vec!["exported".to_string()]);
    assert_eq!(set.confidence, Confidence::High);
}
