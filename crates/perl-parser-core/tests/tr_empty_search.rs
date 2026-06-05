//! Regression tests for empty `tr///` search list (#752, finding 7).
//!
//! `tr///` with an empty search list is valid Perl — it counts characters.
//! `$count = ($str =~ tr///)` and `$str =~ tr///d` are well-defined idioms.
//! The parser must not reject them with "missing search list in transliteration".

mod cpan_test_helpers;
use cpan_test_helpers::*;

/// `$s =~ tr///;` is the character-count idiom; must parse without error.
#[test]
fn tr_empty_search_bare_is_valid() {
    assert_clean_parse(r#"$s =~ tr///;"#);
}

/// `$count = ($s =~ tr///)` — count variant with assignment.
#[test]
fn tr_empty_search_count_assignment_is_valid() {
    assert_clean_parse(r#"$count = ($s =~ tr///);"#);
}

/// Empty search with `d` modifier is valid.
#[test]
fn tr_empty_search_with_d_modifier_is_valid() {
    assert_clean_parse(r#"$s =~ tr///d;"#);
}

/// Empty search with `s` modifier is valid.
#[test]
fn tr_empty_search_with_s_modifier_is_valid() {
    assert_clean_parse(r#"$s =~ tr///s;"#);
}

/// `y///` (alias for tr) with empty search is also valid.
#[test]
fn y_empty_search_is_valid() {
    assert_clean_parse(r#"$s =~ y///;"#);
}

// --- Regression: existing correct parses must still work ---

/// `tr/abc/xyz/` must still parse.
#[test]
fn tr_non_empty_search_still_parses() {
    assert_clean_parse(r#"$s =~ tr/abc/xyz/;"#);
}

/// `tr/a-z/A-Z/` must still parse.
#[test]
fn tr_range_still_parses() {
    assert_clean_parse(r#"$s =~ tr/a-z/A-Z/;"#);
}

/// `tr/x//` (count specific char) must still parse.
#[test]
fn tr_count_specific_char_still_parses() {
    assert_clean_parse(r#"$s =~ tr/x//;"#);
}

/// `y/a/b/` must still parse.
#[test]
fn y_normal_still_parses() {
    assert_clean_parse(r#"$s =~ y/a/b/;"#);
}
