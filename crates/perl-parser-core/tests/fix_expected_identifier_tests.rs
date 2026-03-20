mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for issue #2397: expected_identifier — version literals, sigil edge cases

#[test]
fn test_use_version_numeric() {
    // use 5.010; — numeric version literal
    let source = r#"use 5.010;"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_version_vstring() {
    // use v5.10; — v-string version
    let source = r#"use v5.10;"#;
    assert_clean_parse(source);
}

#[test]
fn test_our_version_vstring() {
    // our $VERSION = v1.2.3;
    let source = r#"our $VERSION = v1.2.3;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_version_vstring() {
    // package Foo v1.0; — package with v-string version
    let source = r#"package Foo v1.0;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_version_dotted() {
    // package Foo 1.0; — package with dotted numeric version
    let source = r#"package Foo 1.0;"#;
    assert_clean_parse(source);
}

#[test]
fn test_local_dollar_underscore() {
    // local $_; — bare special variable
    let source = r#"local $_;"#;
    assert_clean_parse(source);
}

#[test]
fn test_local_input_record_separator() {
    // local $/; — input record separator
    let source = r#"local $/;"#;
    assert_clean_parse(source);
}

#[test]
fn test_local_dollar_bang() {
    // local $!; — errno variable
    let source = r#"local $!;"#;
    assert_clean_parse(source);
}

// Actual corpus failures: no if CONDITION, MODULE
#[test]
fn test_no_if_condition_module() {
    // no if $] >= 5.014, warnings => 'Imager::channelmask';
    let source = r#"no if $] >= 5.014, warnings => 'Imager::channelmask';"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_if_feature_variant() {
    // no if $^V ge v5.10.0, 'feature', 'switch';
    let source = r#"no if $^V ge v5.10.0, 'feature', 'switch';"#;
    assert_clean_parse(source);
}

// Actual corpus failures: method keyword used as function call
#[test]
fn test_method_as_function_call() {
    // sub request_method { method(@_) }
    let source = r#"sub request_method { method(@_) }"#;
    assert_clean_parse(source);
}

#[test]
fn test_method_call_bare() {
    // method(@_) — method as a bareword function call
    let source = r#"method(@_);"#;
    assert_clean_parse(source);
}
