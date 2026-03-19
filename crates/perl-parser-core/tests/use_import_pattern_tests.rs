//! Test coverage for `use` statement import patterns.
//!
//! Covers the expected_import_item bucket: basic imports, version imports,
//! parent/base inheritance, pragmas, `no` statements, and complex patterns.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ===========================================================================
// Basic import patterns
// ===========================================================================

#[test]
fn use_bare_module() {
    assert_clean_parse("use Module;");
}

#[test]
fn use_module_qw_import_list() {
    assert_clean_parse("use Module qw(func1 func2);");
}

#[test]
fn use_module_tag_all() {
    assert_clean_parse("use Module ':all';");
}

#[test]
fn use_module_empty_import() {
    assert_clean_parse("use Module ();");
}

// ===========================================================================
// Version import patterns
// ===========================================================================

#[test]
fn use_module_with_version() {
    assert_clean_parse("use Module 1.23;");
}

#[test]
fn use_module_version_and_qw() {
    assert_clean_parse("use Module 1.23 qw(func);");
}

#[test]
fn use_v_version() {
    assert_clean_parse("use v5.26;");
}

// ===========================================================================
// Parent / base inheritance patterns
// ===========================================================================

#[test]
fn use_parent_single_quoted() {
    assert_clean_parse("use parent 'Module::Name';");
}

#[test]
fn use_parent_norequire_flag() {
    assert_clean_parse("use parent -norequire, 'Module::Name';");
}

#[test]
fn use_base_qw() {
    assert_clean_parse("use base qw(Module::Name);");
}

// ===========================================================================
// Pragma patterns
// ===========================================================================

#[test]
fn use_strict() {
    assert_clean_parse("use strict;");
}

#[test]
fn use_warnings_category() {
    assert_clean_parse("use warnings 'all';");
}

#[test]
fn use_feature_say() {
    assert_clean_parse("use feature 'say';");
}

#[test]
fn use_feature_version_bundle() {
    assert_clean_parse("use feature ':5.26';");
}

// ===========================================================================
// No (unimport) patterns
// ===========================================================================

#[test]
fn no_strict_refs() {
    assert_clean_parse("no strict 'refs';");
}

#[test]
fn no_warnings_experimental() {
    assert_clean_parse("no warnings 'experimental';");
}

// ===========================================================================
// Complex import patterns
// ===========================================================================

#[test]
fn use_exporter_import() {
    assert_clean_parse("use Exporter 'import';");
}

#[test]
fn use_constant_hash() {
    assert_clean_parse("use constant { FOO => 1, BAR => 2 };");
}
