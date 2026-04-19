//! Comprehensive unit tests for `perl-module-boundary`.
//!
//! Covers the public API: [`find_standalone_module_token_ranges`],
//! [`contains_standalone_module_token`], and [`ModuleTokenRange`].

use perl_module::boundary::{
    ModuleTokenRange, contains_standalone_module_token, find_standalone_module_token_ranges,
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn ranges(line: &str, module_name: &str) -> Vec<ModuleTokenRange> {
    find_standalone_module_token_ranges(line, module_name).collect()
}

// ===========================================================================
// 1. Empty / degenerate inputs
// ===========================================================================

#[test]
fn empty_line_returns_no_matches() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("", "Foo");
    assert!(r.is_empty(), "expected no matches for empty line");
    Ok(())
}

#[test]
fn empty_module_name_returns_no_matches() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Bar;", "");
    assert!(r.is_empty(), "expected no matches for empty module_name");
    Ok(())
}

#[test]
fn both_empty_returns_no_matches() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("", "");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn contains_returns_false_for_empty_line() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_standalone_module_token("", "Foo"));
    Ok(())
}

#[test]
fn contains_returns_false_for_empty_module() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!contains_standalone_module_token("use Foo;", ""));
    Ok(())
}

// ===========================================================================
// 2. Simple standalone matches (canonical :: separator)
// ===========================================================================

#[test]
fn single_canonical_module_in_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Bar;", "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 4, end: 12 });
    Ok(())
}

#[test]
fn contains_detects_canonical_module() -> Result<(), Box<dyn std::error::Error>> {
    assert!(contains_standalone_module_token(
        "use Foo::Bar;",
        "Foo::Bar"
    ));
    Ok(())
}

#[test]
fn single_segment_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Moose;", "Moose");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 4, end: 9 });
    Ok(())
}

#[test]
fn deeply_nested_canonical_module() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use A::B::C::D;", "A::B::C::D");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 4, end: 14 });
    Ok(())
}

// ===========================================================================
// 3. Boundary rejection — partial matches
// ===========================================================================

#[test]
fn rejects_prefix_match_extending_right_with_colons() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Bar::Baz;", "Foo::Bar");
    assert!(
        r.is_empty(),
        "should reject when module continues right with ::"
    );
    Ok(())
}

#[test]
fn rejects_suffix_match_extending_left_with_colons() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Outer::Foo::Bar;", "Foo::Bar");
    assert!(r.is_empty(), "should reject when module is preceded by ::");
    Ok(())
}

#[test]
fn rejects_when_followed_by_identifier_char() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Barista;", "Foo::Bar");
    assert!(r.is_empty());
    assert!(!contains_standalone_module_token(
        "use Foo::Barista;",
        "Foo::Bar"
    ));
    Ok(())
}

#[test]
fn rejects_when_preceded_by_identifier_char() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use XFoo::Bar;", "Foo::Bar");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn rejects_when_preceded_by_underscore() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use _Foo::Bar;", "Foo::Bar");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn rejects_when_followed_by_underscore() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Bar_Extra;", "Foo::Bar");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn rejects_when_followed_by_digit() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Bar2;", "Foo::Bar");
    assert!(r.is_empty());
    Ok(())
}

// ===========================================================================
// 4. Legacy separator (single-quote) boundary tests
// ===========================================================================

#[test]
fn rejects_when_followed_by_legacy_separator_and_ident() -> Result<(), Box<dyn std::error::Error>> {
    // Foo::Bar'Baz — the right context is 'B which is a legacy separator
    let r = ranges("use Foo::Bar'Baz;", "Foo::Bar");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn rejects_when_preceded_by_legacy_separator_and_ident() -> Result<(), Box<dyn std::error::Error>> {
    // Outer'Foo::Bar — the left context is r' which is a legacy separator
    let r = ranges("use Outer'Foo::Bar;", "Foo::Bar");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn legacy_separator_module_not_followed_by_ident_is_standalone()
-> Result<(), Box<dyn std::error::Error>> {
    // Foo'Bar followed by ; — standalone
    let r = ranges("use Foo'Bar;", "Foo'Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 4, end: 11 });
    Ok(())
}

// ===========================================================================
// 5. Multiple occurrences on a single line
// ===========================================================================

#[test]
fn finds_multiple_standalone_matches() -> Result<(), Box<dyn std::error::Error>> {
    let line = "Foo Foo Foo";
    let r = ranges(line, "Foo");
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], ModuleTokenRange { start: 0, end: 3 });
    assert_eq!(r[1], ModuleTokenRange { start: 4, end: 7 });
    assert_eq!(r[2], ModuleTokenRange { start: 8, end: 11 });
    Ok(())
}

#[test]
fn finds_multiple_qualified_matches() -> Result<(), Box<dyn std::error::Error>> {
    let line = "use Foo::Bar; require Foo::Bar;";
    let r = ranges(line, "Foo::Bar");
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].start, 4);
    assert_eq!(r[1].start, 22);
    Ok(())
}

#[test]
fn mix_of_standalone_and_rejected_on_same_line() -> Result<(), Box<dyn std::error::Error>> {
    // First Foo::Bar is part of Foo::Bar::Baz (rejected),
    // second Foo::Bar is standalone.
    let line = "Foo::Bar::Baz Foo::Bar";
    let r = ranges(line, "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 14, end: 22 });
    Ok(())
}

// ===========================================================================
// 6. Boundary characters — non-identifier punctuation
// ===========================================================================

#[test]
fn accepts_module_preceded_by_open_paren() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("(Foo::Bar)", "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 1, end: 9 });
    Ok(())
}

#[test]
fn accepts_module_preceded_by_dollar_sign() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("$Foo::Bar", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn accepts_module_at_start_of_line() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("Foo::Bar->new", "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 0, end: 8 });
    Ok(())
}

#[test]
fn accepts_module_at_end_of_line() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("my $x = Foo::Bar", "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].end, 16);
    Ok(())
}

#[test]
fn accepts_module_followed_by_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("Foo::Bar->new()", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn accepts_module_followed_by_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Bar;", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn accepts_module_followed_by_comma() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use Foo::Bar, 'baz';", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn accepts_module_surrounded_by_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("  Foo::Bar  ", "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 2, end: 10 });
    Ok(())
}

// ===========================================================================
// 7. ModuleTokenRange struct properties
// ===========================================================================

#[test]
fn module_token_range_debug_impl() -> Result<(), Box<dyn std::error::Error>> {
    let range = ModuleTokenRange { start: 0, end: 5 };
    let debug_str = format!("{range:?}");
    assert!(debug_str.contains("ModuleTokenRange"));
    assert!(debug_str.contains("start"));
    assert!(debug_str.contains("end"));
    Ok(())
}

#[test]
fn module_token_range_clone() -> Result<(), Box<dyn std::error::Error>> {
    let range = ModuleTokenRange { start: 4, end: 12 };
    let cloned = range;
    assert_eq!(range, cloned);
    Ok(())
}

#[test]
fn module_token_range_eq() -> Result<(), Box<dyn std::error::Error>> {
    let a = ModuleTokenRange { start: 0, end: 5 };
    let b = ModuleTokenRange { start: 0, end: 5 };
    let c = ModuleTokenRange { start: 1, end: 5 };
    assert_eq!(a, b);
    assert_ne!(a, c);
    Ok(())
}

// ===========================================================================
// 8. Iterator behavior
// ===========================================================================

#[test]
fn iterator_is_clone() -> Result<(), Box<dyn std::error::Error>> {
    let iter = find_standalone_module_token_ranges("use Foo;", "Foo");
    let mut cloned = iter.clone();
    let item = cloned.next();
    assert!(item.is_some());
    Ok(())
}

#[test]
fn iterator_is_fused_after_exhaustion() -> Result<(), Box<dyn std::error::Error>> {
    let mut iter = find_standalone_module_token_ranges("use Foo;", "Foo");
    let first = iter.next();
    assert!(first.is_some());
    let second = iter.next();
    assert!(second.is_none());
    // Calling next again should still return None (fused behavior)
    let third = iter.next();
    assert!(third.is_none());
    Ok(())
}

#[test]
fn iterator_returns_none_immediately_for_no_match() -> Result<(), Box<dyn std::error::Error>> {
    let mut iter = find_standalone_module_token_ranges("hello world", "Foo::Bar");
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
    Ok(())
}

#[test]
fn iterator_debug_impl() -> Result<(), Box<dyn std::error::Error>> {
    let iter = find_standalone_module_token_ranges("use Foo;", "Foo");
    let debug_str = format!("{iter:?}");
    assert!(debug_str.contains("ModuleTokenRangeIter"));
    Ok(())
}

// ===========================================================================
// 9. Non-overlapping guarantee
// ===========================================================================

#[test]
fn matches_are_non_overlapping_and_in_order() -> Result<(), Box<dyn std::error::Error>> {
    let line = "Foo Foo Foo";
    let r = ranges(line, "Foo");
    for window in r.windows(2) {
        assert!(
            window[0].end <= window[1].start,
            "matches must be non-overlapping: {:?} vs {:?}",
            window[0],
            window[1]
        );
    }
    Ok(())
}

// ===========================================================================
// 10. Real-world Perl patterns
// ===========================================================================

#[test]
fn use_statement_with_import_list() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use File::Basename qw(basename dirname);", "File::Basename");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 4, end: 18 });
    Ok(())
}

#[test]
fn require_statement() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("require Carp;", "Carp");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 8, end: 12 });
    Ok(())
}

#[test]
fn method_call_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("my $obj = Foo::Bar->new();", "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 10, end: 18 });
    Ok(())
}

#[test]
fn isa_check_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("if ($obj->isa('Foo::Bar')) {", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn package_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("package My::App::Controller;", "My::App::Controller");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 8, end: 27 });
    Ok(())
}

#[test]
fn extends_or_with_moose_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("extends 'My::Base::Class';", "My::Base::Class");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn static_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("Carp::croak('error');", "Carp");
    // "Carp" followed by "::" is not standalone — it continues
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn fully_qualified_function_call_module() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("Carp::croak('error');", "Carp::croak");
    assert_eq!(r.len(), 1);
    Ok(())
}

// ===========================================================================
// 11. Edge cases with special characters around module
// ===========================================================================

#[test]
fn module_in_hash_ref_access() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("$hash->{Foo::Bar}", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn module_in_array_context() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("@ISA = (Foo::Bar);", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn module_name_not_present_at_all() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("use strict;", "Foo::Bar");
    assert!(r.is_empty());
    assert!(!contains_standalone_module_token("use strict;", "Foo::Bar"));
    Ok(())
}

#[test]
fn module_name_longer_than_line() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("Foo", "Foo::Bar::Baz::Qux");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn module_name_equal_to_full_line() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("Foo::Bar", "Foo::Bar");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], ModuleTokenRange { start: 0, end: 8 });
    Ok(())
}

// ===========================================================================
// 12. contains_standalone_module_token consistency
// ===========================================================================

#[test]
fn contains_is_consistent_with_iterator() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("use Foo::Bar;", "Foo::Bar", true),
        ("use Foo::Barista;", "Foo::Bar", false),
        ("Foo::Bar::Baz", "Foo::Bar", false),
        ("Foo Foo Foo", "Foo", true),
        ("", "Foo", false),
        ("Foo", "", false),
        ("require Carp;", "Carp", true),
    ];

    for (line, module, expected) in &cases {
        let has_match = !ranges(line, module).is_empty();
        let contains = contains_standalone_module_token(line, module);
        assert_eq!(
            has_match, *expected,
            "ranges mismatch for ({line:?}, {module:?})"
        );
        assert_eq!(
            contains, *expected,
            "contains mismatch for ({line:?}, {module:?})"
        );
        assert_eq!(
            has_match, contains,
            "ranges vs contains inconsistent for ({line:?}, {module:?})"
        );
    }
    Ok(())
}

// ===========================================================================
// 13. Unicode handling
// ===========================================================================

#[test]
fn line_with_unicode_before_module() -> Result<(), Box<dyn std::error::Error>> {
    // Unicode chars are not identifier chars, so they act as boundaries
    let r = ranges("# café Foo::Bar", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

#[test]
fn line_with_unicode_after_module() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("Foo::Bar ñ", "Foo::Bar");
    assert_eq!(r.len(), 1);
    Ok(())
}

// ===========================================================================
// 14. Whitespace-only and comment-like lines
// ===========================================================================

#[test]
fn whitespace_only_line() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("   \t  ", "Foo");
    assert!(r.is_empty());
    Ok(())
}

#[test]
fn comment_line_with_module() -> Result<(), Box<dyn std::error::Error>> {
    let r = ranges("# use Foo::Bar;", "Foo::Bar");
    assert_eq!(
        r.len(),
        1,
        "boundary crate is syntax-unaware, should match in comments"
    );
    Ok(())
}
