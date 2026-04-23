//! Regression tests for strict transliteration parsing.
//!
//! Ensures `tr///` and `y///` parsing supports optional whitespace before
//! delimiters and rejects invalid modifier characters with diagnostics.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── Basic whitespace support ─────────────────────────────────────────────────

#[test]
fn transliteration_allows_whitespace_before_delimiter() {
    assert_clean_parse(r#"$x =~ tr /a-z/A-Z/;"#);
    assert_clean_parse(r#"$x =~ y  {abc}{xyz}r;"#);
}

// ── Invalid modifiers are rejected ───────────────────────────────────────────

#[test]
fn transliteration_rejects_invalid_modifiers() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/z;"#, "invalid transliteration modifier");
    assert_has_error(r#"$x =~ y/a-z/A-Z/1;"#, "invalid transliteration modifier");
}

// ── Whitespace variations: all paired delimiters ──────────────────────────────

/// tr with space before paired brace
#[test]
fn transliteration_space_before_brace() {
    assert_clean_parse(r#"$x =~ tr {a-z} {A-Z};"#);
}

/// tr with space before paired bracket
#[test]
fn transliteration_space_before_bracket() {
    assert_clean_parse(r#"$x =~ tr [a-z] [A-Z];"#);
}

/// tr with space before paired paren
#[test]
fn transliteration_space_before_paren() {
    assert_clean_parse(r#"$x =~ tr (a-z) (A-Z);"#);
}

/// tr with space before paired angle
#[test]
fn transliteration_space_before_angle() {
    assert_clean_parse(r#"$x =~ tr <a-z> <A-Z>;"#);
}

// ── Whitespace variations: non-paired delimiters ──────────────────────────────

/// tr with space before slash delimiter
#[test]
fn transliteration_space_before_slash() {
    assert_clean_parse(r#"$x =~ tr /a-z/A-Z/;"#);
}

/// tr with space before pipe delimiter
#[test]
fn transliteration_space_before_pipe() {
    assert_clean_parse(r#"$x =~ tr |a-z|A-Z|;"#);
}

/// tr with space before hash delimiter
#[test]
fn transliteration_space_before_hash() {
    assert_clean_parse(r#"$x =~ tr #a-z#A-Z#;"#);
}

// ── Multiple whitespace: tabs, newlines, mixed ───────────────────────────────

/// tr with tab before delimiter
#[test]
fn transliteration_tab_before_delimiter() {
    assert_clean_parse("$x =~ tr\t/a-z/A-Z/;");
}

/// tr with newline before delimiter (Perl allows this)
#[test]
fn transliteration_newline_before_delimiter() {
    assert_clean_parse("$x =~ tr\n/a-z/A-Z/;");
}

/// tr with multiple spaces before delimiter
#[test]
fn transliteration_multiple_spaces_before_delimiter() {
    assert_clean_parse(r#"$x =~ tr     /a-z/A-Z/;"#);
}

// ── y operator variants (alias for tr) ───────────────────────────────────────

/// y with space before delimiter
#[test]
fn transliteration_y_space_before_delimiter() {
    assert_clean_parse(r#"$x =~ y /a-z/A-Z/;"#);
}

/// y with paired braces and space
#[test]
fn transliteration_y_space_before_brace() {
    assert_clean_parse(r#"$x =~ y {a-z} {A-Z};"#);
}

// ── Valid modifiers only ─────────────────────────────────────────────────────

/// tr with single valid modifier 'c'
#[test]
fn transliteration_modifier_c_complement() {
    assert_clean_parse(r#"$x =~ tr/0-9//c;"#);
}

/// tr with single valid modifier 'd'
#[test]
fn transliteration_modifier_d_delete() {
    assert_clean_parse(r#"$x =~ tr/a-z//d;"#);
}

/// tr with single valid modifier 's'
#[test]
fn transliteration_modifier_s_squash() {
    assert_clean_parse(r#"$x =~ tr/a-z/a-z/s;"#);
}

/// tr with single valid modifier 'r'
#[test]
fn transliteration_modifier_r_return() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/r;"#);
}

/// tr with multiple valid modifiers combined
#[test]
fn transliteration_multiple_valid_modifiers() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/cd;"#);
    assert_clean_parse(r#"$x =~ tr/a-z/a-z/sr;"#);
    assert_clean_parse(r#"$x =~ tr/0-9//cds;"#);
}

// ── Invalid modifier detection ───────────────────────────────────────────────

/// tr rejects invalid modifier 'g' (that's for substitution)
#[test]
fn transliteration_rejects_g_modifier() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/g;"#, "invalid transliteration modifier");
}

/// tr rejects invalid modifier 'i' (that's for regex)
#[test]
fn transliteration_rejects_i_modifier() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/i;"#, "invalid transliteration modifier");
}

/// tr rejects invalid modifier 'x' (that's for regex)
#[test]
fn transliteration_rejects_x_modifier() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/x;"#, "invalid transliteration modifier");
}

/// tr rejects numeric modifiers
#[test]
fn transliteration_rejects_numeric_modifiers() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/1;"#, "invalid transliteration modifier");
    assert_has_error(r#"$x =~ tr/a-z/A-Z/0;"#, "invalid transliteration modifier");
}

/// tr stops parsing modifiers at underscore (underscore is not alphanumeric for modifier purposes)
/// This leaves the underscore as a separate token, which is then parsed as a bareword identifier.
#[test]
fn transliteration_underscore_after_modifiers() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/; $_;"#);
}

// ── Empty transliteration lists ──────────────────────────────────────────────

/// tr with empty search list is invalid — strict parser rejects it
#[test]
fn transliteration_empty_search_list_rejected() {
    assert_has_error(r#"$x =~ tr//abc/;"#, "Missing search list");
}

/// tr with empty replacement list (deletes matching chars)
#[test]
fn transliteration_empty_replacement_list() {
    assert_clean_parse(r#"$x =~ tr/abc//;"#);
}

/// tr with both lists empty is invalid — strict parser rejects it
#[test]
fn transliteration_both_lists_empty_rejected() {
    assert_has_error(r#"$x =~ tr///;"#, "Missing search list");
}

// ── Unicode and special characters in transliteration lists ────────────────────

/// tr with Unicode characters in search list
#[test]
fn transliteration_unicode_search() {
    assert_clean_parse(r#"$x =~ tr/äöü/ÄÖÜ/;"#);
}

/// tr with Unicode in both lists
#[test]
fn transliteration_unicode_both_lists() {
    assert_clean_parse(r#"$x =~ tr/αβγ/ΑΒΓ/;"#);
}

/// tr with escaped characters
#[test]
fn transliteration_escaped_chars() {
    assert_clean_parse(r#"$x =~ tr/\n\t\r/XYZ/;"#);
}

/// tr with character ranges
#[test]
fn transliteration_character_ranges() {
    assert_clean_parse(r#"$x =~ tr/0-9a-zA-Z/0-9A-Za-z/;"#);
}

// ── Transliteration in different contexts ────────────────────────────────────

/// tr in scalar assignment
#[test]
fn transliteration_scalar_assignment() {
    assert_clean_parse(r#"my $x = $y =~ tr/a-z/A-Z/r;"#);
}

/// tr in list context
#[test]
fn transliteration_list_context() {
    assert_clean_parse(r#"my @x = split /,/, $y =~ tr/a-z/A-Z/r;"#);
}

/// tr as standalone statement
#[test]
fn transliteration_standalone_statement() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/;"#);
}

/// tr chained with other operators
#[test]
fn transliteration_chained() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/ =~ /[A-Z]/;"#);
}

// ── Hash context regression: tr/y must not be confused with hash keys ────────

/// $h{tr} — 'tr' as bare hash key
#[test]
fn transliteration_not_confused_with_hash_key_tr() {
    assert_clean_parse(r#"my $x = $h{tr};"#);
}

/// $h{y} — 'y' as bare hash key
#[test]
fn transliteration_not_confused_with_hash_key_y() {
    assert_clean_parse(r#"my $x = $h{y};"#);
}

/// $h->{tr} — via arrow
#[test]
fn transliteration_not_confused_with_arrow_hash_key_tr() {
    assert_clean_parse(r#"my $x = $_->{tr};"#);
}

/// $h->{y} — via arrow
#[test]
fn transliteration_not_confused_with_arrow_hash_key_y() {
    assert_clean_parse(r#"my $x = $_->{y};"#);
}

// ── Regression: after-arrow method calls must be recognized ──────────────────

/// Method named 'tr' should not trigger transliteration parsing
#[test]
fn transliteration_method_named_tr_not_operator() {
    assert_clean_parse(r#"$obj->tr("arg");"#);
}

/// Method named 'y' should not trigger transliteration parsing
#[test]
fn transliteration_method_named_y_not_operator() {
    assert_clean_parse(r#"$obj->y("arg");"#);
}

// ── Both tr and y syntax variants ────────────────────────────────────────────

/// tr and y are equivalent
#[test]
fn transliteration_y_equivalent_to_tr() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/;"#);
    assert_clean_parse(r#"$x =~ y/a-z/A-Z/;"#);
}

/// Both accept whitespace
#[test]
fn transliteration_both_accept_whitespace() {
    assert_clean_parse(r#"$x =~ tr /a-z/A-Z/;"#);
    assert_clean_parse(r#"$x =~ y /a-z/A-Z/;"#);
}

// ── Delimiter selection: verify all supported delimiters work ────────────────

/// tr with slash (most common)
#[test]
fn transliteration_slash_delimiter() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/;"#);
}

/// tr with tilde
#[test]
fn transliteration_tilde_delimiter() {
    assert_clean_parse(r#"$x =~ tr~a-z~A-Z~;"#);
}

/// tr with backtick
#[test]
fn transliteration_backtick_delimiter() {
    assert_clean_parse(r#"$x =~ tr`a-z`A-Z`;"#);
}

/// tr with exclamation
#[test]
fn transliteration_exclamation_delimiter() {
    assert_clean_parse(r#"$x =~ tr!a-z!A-Z!;"#);
}

// ── Complex real-world examples ──────────────────────────────────────────────

/// Perl's standard camelCase to snake_case pattern
#[test]
fn transliteration_camel_to_snake() {
    assert_clean_parse(r#"$name =~ tr/A-Z/a-z/;"#);
}

/// Digit transliteration with modifier
#[test]
fn transliteration_digit_mapping() {
    assert_clean_parse(r#"$code =~ tr/0123456789/ABCDEFGHIJ/;"#);
}

/// Delete all digits pattern
#[test]
fn transliteration_delete_with_modifier() {
    assert_clean_parse(r#"$text =~ tr/0-9//d;"#);
}

/// Complement modifier use case
#[test]
fn transliteration_complement_pattern() {
    assert_clean_parse(r#"$text =~ tr/a-zA-Z//cd;"#);
}

// ── Boundary: nested braces in paired delimiter context ─────────────────────

/// tr with nested braces in search list
#[test]
fn transliteration_nested_paired_delimiters() {
    assert_clean_parse(r#"$x =~ tr{a-z}{A-Z};"#);
}

/// y with nested brackets
#[test]
fn transliteration_y_nested_brackets() {
    assert_clean_parse(r#"$x =~ y[a-z][A-Z];"#);
}
