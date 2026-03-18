mod cpan_test_helpers;
use cpan_test_helpers::*;

// ===== Parenthesized import list patterns that trigger expected_import_item =====
// The parenthesized import list parser (lines 702-728 of declarations.rs) only
// accepts String and Identifier tokens. These tests cover tokens that appear
// in real CPAN modules inside use Foo (...) import lists.

#[test]
fn test_use_paren_import_number() {
    // Number inside parenthesized import list (e.g., version constraint)
    // TokenKind::Number is not accepted by the current parser
    let source = r#"use Test::More (tests => 42);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_fat_arrow_pair() {
    // Fat arrow key-value pairs inside parenthesized import list
    // TokenKind::FatArrow is not handled after consuming identifier
    let source = r#"use Test::More (tests => 42);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_backslash_coderef() {
    // Backslash reference in parenthesized import list (use overload pattern)
    // TokenKind::Backslash is not accepted
    let source = r#"use overload ('""' => \&stringify, '+' => \&add);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_minus_flag() {
    // Minus flag in parenthesized import list
    // TokenKind::Minus is not accepted
    let source = r#"use Module::Load (-norequire);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_bracket_ref() {
    // Array reference inside parenthesized import list
    // TokenKind::LeftBracket is not accepted
    let source = r#"use Module (import => ['func1', 'func2']);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_hashref() {
    // Hash reference inside parenthesized import list
    // TokenKind::LeftBrace is not accepted
    let source = r#"use constant ({FOO => 1, BAR => 2});"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_colon_tag_bare() {
    // Bare :tag import (colon token + identifier, not a string)
    // TokenKind::Colon is not accepted
    let source = r#"use POSIX (:sys_wait_h);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_undef() {
    // undef in parenthesized import list
    // TokenKind::Undef keyword is not accepted
    let source = r#"use Module (undef);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_sigil_variable() {
    // Variable in parenthesized import list
    // TokenKind::ScalarSigil is not accepted
    let source = r#"use warnings ($flag);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_not_operator() {
    // Negation operator in parenthesized import list
    // TokenKind::Not is not accepted
    let source = r#"use Module (!default);"#;
    assert_clean_parse(source);
}

// ===== Patterns that already work (regression protection) =====

#[test]
fn test_use_paren_import_strings() {
    // Quoted strings in parens (already works - TokenKind::String)
    let source = r#"use POSIX (':sys_wait_h');"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_identifiers() {
    // Bare identifiers in parens (already works - TokenKind::Identifier)
    let source = r#"use Module (foo, bar, baz);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_empty() {
    // Empty import list (already works)
    let source = r#"use Module ();"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_qw_import_list() {
    // qw import list (already works)
    let source = r#"use Module qw(func1 func2 func3);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_version_import() {
    // Version-only import (already works)
    let source = r#"use Module 1.23;"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_parent_bare_string() {
    // use parent with bare string (already works)
    let source = r#"use parent 'Module::Name';"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_base_qw() {
    // use base with qw (already works)
    let source = r#"use base qw(Module::Name Other::Module);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_bare_colon_tag() {
    // Bare :tag import without parens (already works)
    let source = r#"use Exporter ':all';"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_version_plus_qw() {
    // Version number followed by qw list (already works)
    let source = r#"use Module 1.23 qw(foo bar);"#;
    assert_clean_parse(source);
}

// ===== Complex CPAN-style patterns =====

#[test]
fn test_use_overload_multiple_operators() {
    // use overload with multiple operator => coderef pairs in parens
    let source = r#"use overload ('==' => \&equal, '!=' => \&not_equal, '""' => \&stringify, fallback => 1);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_mixed_types() {
    // Mix of different token types in parens
    let source = r#"use Test::Builder (import => ['ok', 'is'], tests => 10);"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_parent_paren_list() {
    // use parent with parenthesized list of module names (strings)
    let source = r#"use parent ('Module::Name', 'Other::Module');"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_paren_import_negative_number() {
    // Negative number in parens
    let source = r#"use Module (level => -1);"#;
    assert_clean_parse(source);
}
