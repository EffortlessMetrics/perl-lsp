//! Tests for three-part dotted bare VERSION in `use` statements.
//!
//! `use 5.10.1;` (bare numeric three-part version, no import args)
//! was misparsing: the `.1` segment leaked into the args list as
//! `(use 5.10 (. 1))`.  Per perlsyn, `use VERSION` takes NO import
//! list.  `5.10.1` must be captured fully as the module/version field.
//!
//! See issue #751 (Bug 2).

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Parse a single-statement source, return (module, args) from the Use node.
fn use_node_parts(source: &str) -> Result<(String, Vec<String>), String> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let NodeKind::Program { statements } = &ast.kind else {
        return Err(format!("expected Program, got {:?}", ast.kind));
    };
    let stmt = statements.first().ok_or("no statements")?;
    let NodeKind::Use { module, args, .. } = &stmt.kind else {
        return Err(format!("expected Use node, got {:?}", stmt.kind));
    };
    Ok((module.clone(), args.clone()))
}

// ── Three-part dotted version (the bug) ──────────────────────────────────────

#[test]
fn test_use_three_part_dotted_5_10_1_parses_cleanly() -> Result<(), String> {
    assert_clean_parse("use 5.10.1;");
    Ok(())
}

#[test]
fn test_use_three_part_dotted_5_10_1_no_args() -> Result<(), String> {
    let (module, args) = use_node_parts("use 5.10.1;")?;
    assert_eq!(
        args,
        Vec::<String>::new(),
        "use 5.10.1 must have NO import args, but got: {:?}",
        args
    );
    // The module field must capture all three parts (no leaked segments).
    assert!(module.contains('5'), "module field '{}' must contain the major version digit", module);
    Ok(())
}

#[test]
fn test_use_three_part_dotted_5_10_1_full_version_captured() -> Result<(), String> {
    let (module, args) = use_node_parts("use 5.10.1;")?;
    assert_eq!(
        args,
        Vec::<String>::new(),
        "use 5.10.1 must have NO import args, got: {:?}; module='{}'",
        args,
        module
    );
    // Full dotted version should be captured in module, not truncated.
    assert_eq!(module, "5.10.1", "module field must be '5.10.1', got '{}'", module);
    Ok(())
}

#[test]
fn test_use_three_part_dotted_5_30_3_no_args() -> Result<(), String> {
    let (module, args) = use_node_parts("use 5.30.3;")?;
    assert_eq!(args, Vec::<String>::new(), "use 5.30.3 must have NO import args, got: {:?}", args);
    assert_eq!(module, "5.30.3");
    Ok(())
}

#[test]
fn test_use_three_part_dotted_5_8_0_no_args() -> Result<(), String> {
    let (module, args) = use_node_parts("use 5.8.0;")?;
    assert_eq!(args, Vec::<String>::new(), "use 5.8.0 must have NO import args, got: {:?}", args);
    assert_eq!(module, "5.8.0");
    Ok(())
}

// ── Regression guards: existing forms must keep working ──────────────────────

#[test]
fn test_use_two_part_dotted_5_10_unchanged() -> Result<(), String> {
    let (module, args) = use_node_parts("use 5.10;")?;
    assert_eq!(args, Vec::<String>::new(), "use 5.10 must have NO import args, got: {:?}", args);
    assert!(module.contains("5.10"), "module field '{}' must contain '5.10'", module);
    Ok(())
}

#[test]
fn test_use_vprefix_three_part_unchanged() {
    // use v5.10.1 (v-prefix) must still parse cleanly — unchanged path.
    assert_clean_parse("use v5.10.1;");
}

#[test]
fn test_use_vprefix_three_part_no_args() -> Result<(), String> {
    let (module, args) = use_node_parts("use v5.10.1;")?;
    assert_eq!(args, Vec::<String>::new(), "use v5.10.1 must have NO import args, got: {:?}", args);
    assert!(module.starts_with('v'), "v-prefix module '{}' must start with 'v'", module);
    Ok(())
}

#[test]
fn test_use_strict_bareword_unchanged() -> Result<(), String> {
    let (module, args) = use_node_parts("use strict;")?;
    assert_eq!(module, "strict");
    assert_eq!(args, Vec::<String>::new());
    Ok(())
}

#[test]
fn test_use_module_with_qw_import_unchanged() -> Result<(), String> {
    let (module, args) = use_node_parts("use Foo qw(a b);")?;
    assert_eq!(module, "Foo");
    assert!(!args.is_empty(), "Foo qw(a b) must have import args");
    Ok(())
}

#[test]
fn test_use_posix_empty_import_unchanged() -> Result<(), String> {
    let (module, _args) = use_node_parts("use POSIX ();")?;
    assert_eq!(module, "POSIX");
    // The key invariant: no error node.
    assert_clean_parse("use POSIX ();");
    Ok(())
}

#[test]
fn test_use_three_part_in_full_program() {
    // Realistic program context: version pragma plus other statements.
    assert_clean_parse(
        r#"
use 5.10.1;
use strict;
use warnings;
my $x = 1;
"#,
    );
}

#[test]
fn test_use_5_30_3_parses_cleanly() {
    assert_clean_parse("use 5.30.3;");
}

#[test]
fn test_use_5_8_0_parses_cleanly() {
    assert_clean_parse("use 5.8.0;");
}

#[test]
fn test_use_three_part_no_parser_errors() {
    let src = "use 5.10.1;";
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    let errors = parser.get_errors();
    assert!(errors.is_empty(), "use 5.10.1 must produce no parser errors, got: {:?}", errors);
}
