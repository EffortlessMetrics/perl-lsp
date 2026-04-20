#![allow(clippy::panic)]

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::NodeKind;

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
fn test_use_vstring_preserves_full_version_segment() {
    let ast = parse("use v5.38;");
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::Use { module, .. } = &statements[0].kind {
            assert_eq!(module, "v5.38");
        } else {
            panic!("expected top-level Use node");
        }
    } else {
        panic!("expected Program node");
    }
}

#[test]
fn test_use_vstring_three_part_preserves_patch_segment() {
    let ast = parse("use v5.12.0;");
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::Use { module, .. } = &statements[0].kind {
            assert_eq!(module, "v5.12.0");
        } else {
            panic!("expected top-level Use node");
        }
    } else {
        panic!("expected Program node");
    }
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
