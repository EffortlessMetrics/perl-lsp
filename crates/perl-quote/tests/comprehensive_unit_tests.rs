//! Comprehensive unit tests for `perl-quote` public API.

use perl_quote::{
    SubstitutionError, extract_regex_parts, extract_substitution_parts,
    extract_substitution_parts_strict, extract_transliteration_parts,
    validate_substitution_modifiers,
};
use perl_tdd_support::{must, must_err};

/// Helper to convert SubstitutionError results (which don't impl std::error::Error)
fn strict(input: &str) -> (String, String, String) {
    must(extract_substitution_parts_strict(input))
}

// ──────────────────────────────────────────────────────────────
// extract_regex_parts
// ──────────────────────────────────────────────────────────────

#[test]
fn regex_bare_slash() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("/hello/i");
    assert_eq!(pat, "/hello/");
    assert_eq!(body, "hello");
    assert_eq!(mods, "i");
    Ok(())
}

#[test]
fn regex_m_operator() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("m/world/gm");
    assert_eq!(pat, "/world/");
    assert_eq!(body, "world");
    assert_eq!(mods, "gm");
    Ok(())
}

#[test]
fn regex_qr_operator() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("qr{foo}ix");
    assert_eq!(pat, "{foo}");
    assert_eq!(body, "foo");
    assert_eq!(mods, "ix");
    Ok(())
}

#[test]
fn regex_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("");
    assert_eq!(pat, "");
    assert_eq!(body, "");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn regex_qr_with_no_content() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("qr");
    assert_eq!(pat, "");
    assert_eq!(body, "");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn regex_m_with_no_content() -> Result<(), Box<dyn std::error::Error>> {
    // m followed by nothing after the single char stripped
    let (pat, body, mods) = extract_regex_parts("m");
    // m is not followed by non-alpha so text stays as "m"
    assert_eq!(body, "");
    let _ = (pat, mods);
    Ok(())
}

#[test]
fn regex_no_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("/abc/");
    assert_eq!(pat, "/abc/");
    assert_eq!(body, "abc");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn regex_braces_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("qr{nested{1}}x");
    assert_eq!(body, "nested{1}");
    assert_eq!(mods, "x");
    let _ = pat;
    Ok(())
}

#[test]
fn regex_brackets_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("qr[abc]");
    assert_eq!(pat, "[abc]");
    assert_eq!(body, "abc");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn regex_parens_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("qr(abc)s");
    assert_eq!(pat, "(abc)");
    assert_eq!(body, "abc");
    assert_eq!(mods, "s");
    Ok(())
}

#[test]
fn regex_angle_brackets_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("qr<abc>m");
    assert_eq!(pat, "<abc>");
    assert_eq!(body, "abc");
    assert_eq!(mods, "m");
    Ok(())
}

#[test]
fn regex_escaped_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (_, body, _) = extract_regex_parts(r"/he\/llo/");
    assert_eq!(body, r"he\/llo");
    Ok(())
}

#[test]
fn regex_m_with_exclamation_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("m!pattern!i");
    assert_eq!(pat, "!pattern!");
    assert_eq!(body, "pattern");
    assert_eq!(mods, "i");
    Ok(())
}

#[test]
fn regex_unicode_body() -> Result<(), Box<dyn std::error::Error>> {
    let (_, body, _) = extract_regex_parts("/café/");
    assert_eq!(body, "café");
    Ok(())
}

#[test]
fn regex_multiple_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_regex_parts("/test/gimsx");
    assert_eq!(mods, "gimsx");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// extract_substitution_parts (lenient)
// ──────────────────────────────────────────────────────────────

#[test]
fn subst_basic_slash() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s/foo/bar/g");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn subst_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s");
    assert_eq!(pat, "");
    assert_eq!(repl, "");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn subst_no_s_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // If text doesn't start with 's', it falls through
    let (pat, repl, mods) = extract_substitution_parts("/foo/bar/");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    let _ = mods;
    Ok(())
}

#[test]
fn subst_braces_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s{foo}{bar}g");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn subst_mixed_paired_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = extract_substitution_parts("s[foo]{bar}");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    Ok(())
}

#[test]
fn subst_empty_pattern_and_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s///g");
    assert_eq!(pat, "");
    assert_eq!(repl, "");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn subst_empty_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s/foo//g");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn subst_escaped_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = extract_substitution_parts(r"s/foo\/bar/baz/");
    assert_eq!(pat, r"foo\/bar");
    assert_eq!(repl, "baz");
    Ok(())
}

#[test]
fn subst_multiple_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_substitution_parts("s/a/b/gimse");
    assert_eq!(mods, "gimse");
    Ok(())
}

#[test]
fn subst_invalid_modifiers_filtered() -> Result<(), Box<dyn std::error::Error>> {
    // Lenient version filters invalid modifiers silently
    let (_, _, mods) = extract_substitution_parts("s/a/b/giz");
    // 'z' should be filtered out
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn subst_unicode_content() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = extract_substitution_parts("s/café/naïve/");
    assert_eq!(pat, "café");
    assert_eq!(repl, "naïve");
    Ok(())
}

#[test]
fn subst_nested_braces() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = extract_substitution_parts("s{a{b}c}{d}");
    assert_eq!(pat, "a{b}c");
    assert_eq!(repl, "d");
    Ok(())
}

#[test]
fn subst_exclamation_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s!foo!bar!g");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn subst_hash_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s#foo#bar#g");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn subst_charset_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_substitution_parts("s/a/b/gad");
    assert_eq!(mods, "gad");
    Ok(())
}

#[test]
fn subst_perl_522_n_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_substitution_parts("s/a/b/gn");
    assert_eq!(mods, "gn");
    Ok(())
}

#[test]
fn subst_e_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_substitution_parts("s/a/b/ge");
    assert_eq!(mods, "ge");
    Ok(())
}

#[test]
fn subst_r_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_substitution_parts("s/a/b/r");
    assert_eq!(mods, "r");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// extract_substitution_parts_strict
// ──────────────────────────────────────────────────────────────

#[test]
fn strict_subst_basic() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = strict("s/foo/bar/gi");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn strict_subst_empty_pattern_and_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = strict("s///g");
    assert_eq!(pat, "");
    assert_eq!(repl, "");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn strict_subst_braces() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = strict("s{pattern}{replacement}gi");
    assert_eq!(pat, "pattern");
    assert_eq!(repl, "replacement");
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn strict_subst_missing_delimiter() {
    let err = must_err(extract_substitution_parts_strict("s"));
    assert_eq!(err, SubstitutionError::MissingDelimiter);
}

#[test]
fn strict_subst_missing_replacement() {
    let err = must_err(extract_substitution_parts_strict("s{}"));
    assert_eq!(err, SubstitutionError::MissingReplacement);
}

#[test]
fn strict_subst_missing_closing_delimiter_nonpaired() {
    let err = must_err(extract_substitution_parts_strict("s/foo/bar"));
    assert_eq!(err, SubstitutionError::MissingClosingDelimiter);
}

#[test]
fn strict_subst_invalid_modifier() {
    let err = must_err(extract_substitution_parts_strict("s/foo/bar/giz"));
    assert_eq!(err, SubstitutionError::InvalidModifier('z'));
}

#[test]
fn strict_subst_all_valid_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = strict("s/a/b/gimsxoeradlunpc");
    assert_eq!(mods, "gimsxoeradlunpc");
    Ok(())
}

#[test]
fn strict_subst_no_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = strict("s/a/b/");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn strict_subst_escaped_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = strict(r"s/foo\/bar/baz/");
    assert_eq!(pat, r"foo\/bar");
    assert_eq!(repl, "baz");
    Ok(())
}

#[test]
fn strict_subst_paired_brackets_to_braces() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = strict("s[pattern]{replacement}");
    assert_eq!(pat, "pattern");
    assert_eq!(repl, "replacement");
    Ok(())
}

#[test]
fn strict_subst_nested_braces() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = strict("s{a{b}}{c}");
    assert_eq!(pat, "a{b}");
    assert_eq!(repl, "c");
    Ok(())
}

#[test]
fn strict_subst_unicode_content() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = strict("s/über/unter/");
    assert_eq!(pat, "über");
    assert_eq!(repl, "unter");
    Ok(())
}

#[test]
fn strict_subst_empty_string_no_s_prefix() {
    // Without 's' prefix, delimiter is first char of empty string
    let err = must_err(extract_substitution_parts_strict(""));
    assert_eq!(err, SubstitutionError::MissingDelimiter);
}

#[test]
fn strict_subst_missing_replacement_paired() {
    let err = must_err(extract_substitution_parts_strict("s{pattern}"));
    assert_eq!(err, SubstitutionError::MissingReplacement);
}

#[test]
fn strict_subst_missing_closing_paired() {
    let err = must_err(extract_substitution_parts_strict("s{pattern"));
    assert_eq!(err, SubstitutionError::MissingClosingDelimiter);
}

#[test]
fn strict_subst_parens_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = strict("s(foo)(bar)");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    Ok(())
}

#[test]
fn strict_subst_angle_brackets() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = strict("s<foo><bar>");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// SubstitutionError — Debug, Clone, PartialEq
// ──────────────────────────────────────────────────────────────

#[test]
fn substitution_error_debug() -> Result<(), Box<dyn std::error::Error>> {
    let err = SubstitutionError::InvalidModifier('z');
    let debug = format!("{:?}", err);
    assert!(debug.contains("InvalidModifier"));
    Ok(())
}

#[test]
fn substitution_error_clone_eq() -> Result<(), Box<dyn std::error::Error>> {
    let err1 = SubstitutionError::MissingDelimiter;
    let err2 = err1.clone();
    assert_eq!(err1, err2);
    Ok(())
}

#[test]
fn substitution_error_variants() -> Result<(), Box<dyn std::error::Error>> {
    // Exercise all variants for coverage
    let variants = vec![
        SubstitutionError::InvalidModifier('q'),
        SubstitutionError::MissingDelimiter,
        SubstitutionError::MissingPattern,
        SubstitutionError::MissingReplacement,
        SubstitutionError::MissingClosingDelimiter,
    ];
    for v in &variants {
        let _ = format!("{:?}", v);
    }
    // Each variant should be distinct
    assert_ne!(variants[0], variants[1]);
    assert_ne!(variants[1], variants[2]);
    assert_ne!(variants[2], variants[3]);
    assert_ne!(variants[3], variants[4]);
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// extract_transliteration_parts
// ──────────────────────────────────────────────────────────────

#[test]
fn tr_basic_slash() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("tr/a-z/A-Z/");
    assert_eq!(search, "a-z");
    assert_eq!(repl, "A-Z");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn tr_y_operator() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("y/a-z/A-Z/");
    assert_eq!(search, "a-z");
    assert_eq!(repl, "A-Z");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn tr_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("tr/a-z/A-Z/cds");
    assert_eq!(search, "a-z");
    assert_eq!(repl, "A-Z");
    assert_eq!(mods, "cds");
    Ok(())
}

#[test]
fn tr_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("tr");
    assert_eq!(search, "");
    assert_eq!(repl, "");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn tr_y_empty() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("y");
    assert_eq!(search, "");
    assert_eq!(repl, "");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn tr_braces_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("tr{a-z}{A-Z}s");
    assert_eq!(search, "a-z");
    assert_eq!(repl, "A-Z");
    assert_eq!(mods, "s");
    Ok(())
}

#[test]
fn tr_mixed_paired_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("tr[a-z]{A-Z}dr");
    assert_eq!(search, "a-z");
    assert_eq!(repl, "A-Z");
    assert_eq!(mods, "dr");

    let (search, repl, mods) = extract_transliteration_parts("y(foo)<bar>r");
    assert_eq!(search, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "r");
    Ok(())
}

#[test]
fn tr_empty_search_and_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, mods) = extract_transliteration_parts("tr///");
    assert_eq!(search, "");
    assert_eq!(repl, "");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn tr_delete_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_transliteration_parts("tr/a-z//d");
    assert_eq!(mods, "d");
    Ok(())
}

#[test]
fn tr_r_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_transliteration_parts("tr/a-z/A-Z/r");
    assert_eq!(mods, "r");
    Ok(())
}

#[test]
fn tr_invalid_modifiers_filtered() -> Result<(), Box<dyn std::error::Error>> {
    // Only c, d, s, r are valid tr modifiers
    let (_, _, mods) = extract_transliteration_parts("tr/a/b/cdsg");
    assert_eq!(mods, "cds");
    Ok(())
}

#[test]
fn tr_escaped_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, _) = extract_transliteration_parts(r"tr/a\/b/c/");
    assert_eq!(search, r"a\/b");
    assert_eq!(repl, "c");
    Ok(())
}

#[test]
fn tr_unicode_content() -> Result<(), Box<dyn std::error::Error>> {
    let (search, repl, _) = extract_transliteration_parts("tr/à-ü/A-U/");
    assert_eq!(search, "à-ü");
    assert_eq!(repl, "A-U");
    Ok(())
}

#[test]
fn tr_no_prefix_fallthrough() -> Result<(), Box<dyn std::error::Error>> {
    // Without tr/y prefix, the whole input is treated as content
    let (search, _, _) = extract_transliteration_parts("/abc/def/");
    assert_eq!(search, "abc");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// validate_substitution_modifiers
// ──────────────────────────────────────────────────────────────

#[test]
fn validate_mods_valid_core() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("gimsxoer"));
    assert_eq!(mods, "gimsxoer");
    Ok(())
}

#[test]
fn validate_mods_valid_charset() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("adlu"));
    assert_eq!(mods, "adlu");
    Ok(())
}

#[test]
fn validate_mods_valid_additional() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("npc"));
    assert_eq!(mods, "npc");
    Ok(())
}

#[test]
fn validate_mods_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers(""));
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn validate_mods_invalid_char() {
    let err = must_err(validate_substitution_modifiers("giz"));
    assert_eq!(err, 'z');
}

#[test]
fn validate_mods_stops_at_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("gi "));
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn validate_mods_stops_at_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("gi;"));
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn validate_mods_stops_at_newline() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("gi\n"));
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn validate_mods_stops_at_carriage_return() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("gi\r"));
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn validate_mods_non_alpha_non_whitespace() {
    let err = must_err(validate_substitution_modifiers("gi!"));
    assert_eq!(err, '!');
}

#[test]
fn validate_mods_first_char_invalid() {
    let err = must_err(validate_substitution_modifiers("z"));
    assert_eq!(err, 'z');
}

#[test]
fn validate_mods_all_valid_combined() -> Result<(), Box<dyn std::error::Error>> {
    let mods = must(validate_substitution_modifiers("gimsxoeradlunpc"));
    assert_eq!(mods, "gimsxoeradlunpc");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Edge cases and boundary conditions
// ──────────────────────────────────────────────────────────────

#[test]
fn regex_single_char_body() -> Result<(), Box<dyn std::error::Error>> {
    let (_, body, _) = extract_regex_parts("/x/");
    assert_eq!(body, "x");
    Ok(())
}

#[test]
fn regex_only_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, body, mods) = extract_regex_parts("//");
    assert_eq!(pat, "//");
    assert_eq!(body, "");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn subst_special_chars_in_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, _, _) = extract_substitution_parts("s/\\d+\\.\\d+/x/");
    assert_eq!(pat, "\\d+\\.\\d+");
    Ok(())
}

#[test]
fn subst_backslash_at_end_of_pattern() -> Result<(), Box<dyn std::error::Error>> {
    // Backslash escapes the next char (the delimiter `/`), so pattern extends
    let (pat, _, _) = extract_substitution_parts("s/test\\/x/");
    assert_eq!(pat, "test\\/x");
    Ok(())
}

#[test]
fn tr_all_four_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, mods) = extract_transliteration_parts("tr/a/b/cdsr");
    assert_eq!(mods, "cdsr");
    Ok(())
}

#[test]
fn strict_subst_with_whitespace_between_paired() -> Result<(), Box<dyn std::error::Error>> {
    // Perl allows whitespace between paired delimiters: s{pat} {repl}
    let (pat, repl, _) = strict("s{pattern} {replacement}");
    assert_eq!(pat, "pattern");
    assert_eq!(repl, "replacement");
    Ok(())
}

#[test]
fn regex_special_regex_chars() -> Result<(), Box<dyn std::error::Error>> {
    let (_, body, _) = extract_regex_parts("/^\\w+$/");
    assert_eq!(body, "^\\w+$");
    Ok(())
}

#[test]
fn subst_pipe_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = extract_substitution_parts("s|foo|bar|g");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "g");
    Ok(())
}

#[test]
fn regex_pipe_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (_, body, _) = extract_regex_parts("m|pattern|");
    assert_eq!(body, "pattern");
    Ok(())
}

#[test]
fn subst_tilde_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = extract_substitution_parts("s~foo~bar~");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    Ok(())
}

#[test]
fn strict_subst_parens_to_parens() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, mods) = strict("s(hello)(world)gi");
    assert_eq!(pat, "hello");
    assert_eq!(repl, "world");
    assert_eq!(mods, "gi");
    Ok(())
}

#[test]
fn strict_subst_invalid_replacement_delimiter() {
    // Paired pattern followed by non-paired opener for replacement
    let err = must_err(extract_substitution_parts_strict("s{pattern}replacement"));
    assert_eq!(err, SubstitutionError::MissingReplacement);
}

#[test]
fn strict_subst_missing_closing_for_replacement() {
    let err = must_err(extract_substitution_parts_strict("s{pattern}{replacement"));
    assert_eq!(err, SubstitutionError::MissingClosingDelimiter);
}

#[test]
fn subst_long_modifiers_string() -> Result<(), Box<dyn std::error::Error>> {
    // All valid modifiers in one go
    let (_, _, mods) = extract_substitution_parts("s/a/b/gimsxoeradlunpc");
    assert_eq!(mods, "gimsxoeradlunpc");
    Ok(())
}

#[test]
fn regex_qr_nested_parens() -> Result<(), Box<dyn std::error::Error>> {
    let (_, body, _) = extract_regex_parts("qr(a(b)c)");
    assert_eq!(body, "a(b)c");
    Ok(())
}

#[test]
fn regex_qr_nested_angle_brackets() -> Result<(), Box<dyn std::error::Error>> {
    let (_, body, _) = extract_regex_parts("qr<a<b>c>");
    assert_eq!(body, "a<b>c");
    Ok(())
}

#[test]
fn subst_deeply_nested_braces() -> Result<(), Box<dyn std::error::Error>> {
    let (pat, repl, _) = extract_substitution_parts("s{a{b{c}}}{d}");
    assert_eq!(pat, "a{b{c}}");
    assert_eq!(repl, "d");
    Ok(())
}

#[test]
fn test_strict_subst_slash_in_double_quoted_replacement() {
    let (pat, repl, mods) = strict(r#"s/foo/sprintf("%s/%s", $a, $b)/e"#);
    assert_eq!(pat, "foo");
    assert_eq!(repl, r#"sprintf("%s/%s", $a, $b)"#);
    assert_eq!(mods, "e");
}

// ──────────────────────────────────────────────────────────────
// Issue #2896: s/''/'/g — single-quote as literal replacement char
// ──────────────────────────────────────────────────────────────

#[test]
fn test_strict_subst_squote_as_replacement_char() {
    // s/''/'/g — replacement is a literal single-quote character
    // This is the primary failing case from TAP/Parser/YAMLish/Reader.pm
    let (pat, repl, mods) = strict("s/''/'/g");
    assert_eq!(pat, "''");
    assert_eq!(repl, "'");
    assert_eq!(mods, "g");
}

#[test]
fn test_strict_subst_squote_slash_join_regression() {
    // Regression guard: join('/', @parts) replacement must still work.
    // `'` immediately followed by `/` (closing delimiter) then `'` (quote) —
    // this is the tricky case that requires lookahead.
    let (pat, repl, mods) = strict(r#"s/([A-Za-z]+)/join('/', @parts)/ge"#);
    assert_eq!(pat, "([A-Za-z]+)");
    assert_eq!(repl, "join('/', @parts)");
    assert_eq!(mods, "ge");
}

#[test]
fn test_strict_subst_dquote_as_replacement_char() {
    // Double-quote variant of the same bug
    let (pat, repl, mods) = strict(r#"s/""/"/g"#);
    assert_eq!(pat, r#""""#);
    assert_eq!(repl, r#"""#);
    assert_eq!(mods, "g");
}
