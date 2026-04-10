//! BDD-style behavior tests for perl-quote public APIs.

use perl_quote::{
    SubstitutionError, extract_regex_parts, extract_substitution_parts,
    extract_substitution_parts_strict, extract_transliteration_parts,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn given_qr_with_nested_delimiters_when_extracting_regex_parts_then_body_and_modifiers_are_preserved()
-> TestResult {
    let (pattern, body, modifiers) = extract_regex_parts("qr{foo{bar}}mx");

    assert_eq!(pattern, "{foo{bar}}");
    assert_eq!(body, "foo{bar}");
    assert_eq!(modifiers, "mx");
    Ok(())
}

#[test]
fn given_match_operator_with_non_paired_delimiter_when_extracting_regex_parts_then_delimited_pattern_is_reported()
-> TestResult {
    let (pattern, body, modifiers) = extract_regex_parts("m!path/to/file!i");

    assert_eq!(pattern, "!path/to/file!");
    assert_eq!(body, "path/to/file");
    assert_eq!(modifiers, "i");
    Ok(())
}

#[test]
fn given_substitution_with_whitespace_after_s_when_extracting_strict_parts_then_input_is_parsed()
-> TestResult {
    let (pattern, replacement, modifiers) =
        extract_substitution_parts_strict("s  {alpha}{beta}gr").map_err(|e| format!("{e:?}"))?;

    assert_eq!(pattern, "alpha");
    assert_eq!(replacement, "beta");
    assert_eq!(modifiers, "gr");
    Ok(())
}

#[test]
fn given_substitution_replacement_with_slash_in_string_literal_when_parsing_then_literal_is_not_split_early()
-> TestResult {
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/foo/print(\"a/b\")/e");

    assert_eq!(pattern, "foo");
    assert_eq!(replacement, "print(\"a/b\")");
    assert_eq!(modifiers, "e");
    Ok(())
}

#[test]
fn given_substitution_with_invalid_modifier_when_parsing_strict_then_error_is_returned() {
    let result = extract_substitution_parts_strict("s/foo/bar/z");

    assert_eq!(result, Err(SubstitutionError::InvalidModifier('z')));
}

#[test]
fn given_unclosed_replacement_when_parsing_strict_substitution_then_missing_closing_delimiter_is_reported()
 {
    let result = extract_substitution_parts_strict("s/foo/bar");

    assert_eq!(result, Err(SubstitutionError::MissingClosingDelimiter));
}

#[test]
fn given_tr_operator_with_mixed_paired_delimiters_when_extracting_then_search_replacement_and_modifiers_are_returned()
-> TestResult {
    let (search, replacement, modifiers) = extract_transliteration_parts("tr[abc]{xyz}cd");

    assert_eq!(search, "abc");
    assert_eq!(replacement, "xyz");
    assert_eq!(modifiers, "cd");
    Ok(())
}

#[test]
fn given_y_alias_with_escaped_delimiter_when_extracting_then_search_and_replacement_keep_escape_sequences()
-> TestResult {
    let (search, replacement, modifiers) = extract_transliteration_parts(r"y#a\#b#c\#d#s");

    assert_eq!(search, r"a\#b");
    assert_eq!(replacement, r"c\#d");
    assert_eq!(modifiers, "s");
    Ok(())
}
