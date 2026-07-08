mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::NodeKind;

fn assert_use_module_preserved(source: &str, expected: &str) -> Result<(), String> {
    let ast = parse(source);
    let NodeKind::Program { statements } = &ast.kind else {
        return Err(format!("expected Program node, got {:?}", ast.kind));
    };
    assert_eq!(statements.len(), 1);

    let Some(statement) = statements.first() else {
        return Err("expected one top-level statement".to_string());
    };
    let NodeKind::Use { module, .. } = &statement.kind else {
        return Err(format!("expected top-level Use node, got {:?}", statement.kind));
    };

    assert_eq!(module, expected);
    Ok(())
}

// --- VString version directives in `use` ---

#[test]
fn test_use_vstring_v5_14() {
    assert_clean_parse("use v5.14;");
}

#[test]
fn test_use_vstring_v5_12_0() {
    assert_clean_parse("use v5.12.0;");
}

#[test]
fn test_use_vstring_v5_6_1() {
    assert_clean_parse("use v5.6.1;");
}

#[test]
fn test_use_vstring_v5_26_0() {
    assert_clean_parse("use v5.26.0;");
}

#[test]
fn test_use_vstring_v5_38() {
    assert_clean_parse("use v5.38;");
}

// --- Numeric version directives in `use` ---

#[test]
fn test_use_numeric_version() {
    assert_clean_parse("use 5.036;");
}

#[test]
fn test_use_numeric_version_old_style() {
    assert_clean_parse("use 5.008;");
}

// --- Standard module imports ---

#[test]
fn test_use_module_simple() {
    assert_clean_parse("use strict;");
}

#[test]
fn test_use_module_with_colons() {
    assert_clean_parse("use File::Basename;");
}

#[test]
fn test_use_module_with_empty_import() {
    assert_clean_parse("use File::Basename ();");
}

#[test]
fn test_use_module_inside_block_without_semicolon() {
    assert_clean_parse("{ use test_use }");
}

#[test]
fn test_use_module_brace_import_after_semicolonless_block_use() {
    assert_clean_parse(
        r#"
{ use test_use }
use test_use { () };
is ref $test_use::got[0], 'HASH', 'use parses arguments in term lexing cx';
"#,
    );
}

#[test]
fn test_use_overload() {
    assert_clean_parse("use overload;");
}

#[test]
fn test_use_no_warnings() {
    assert_clean_parse("no warnings 'recursion';");
}

// --- VString in full program context (CPAN patterns) ---

#[test]
fn test_use_vstring_with_other_statements() {
    assert_clean_parse(
        r#"
use v5.14;
use warnings;
use strict;
my $x = 1;
"#,
    );
}

#[test]
fn test_use_vstring_followed_by_module_import() {
    assert_clean_parse(
        r#"
use v5.14;
use Scalar::Util qw( blessed );
"#,
    );
}

#[test]
fn test_use_vstring_three_part_in_program() {
    assert_clean_parse(
        r#"
use v5.12.0;
use warnings;
1;
"#,
    );
}

#[test]
fn test_use_vstring_preserves_full_version_segment() -> Result<(), String> {
    assert_use_module_preserved("use v5.38;", "v5.38")
}

#[test]
fn test_use_vstring_three_part_preserves_patch_segment() -> Result<(), String> {
    assert_use_module_preserved("use v5.12.0;", "v5.12.0")
}

#[test]
fn test_use_eval_require() {
    assert_clean_parse(
        r#"
if( !$ENV{PERL_FUTURE_NO_XS} and eval { require Future::XS } ) {
    1;
}
"#,
    );
}
