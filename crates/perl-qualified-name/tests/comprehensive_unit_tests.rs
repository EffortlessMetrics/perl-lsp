//! Comprehensive unit tests for `perl-qualified-name` crate.
//!
//! Focuses on edge cases, error variant matching, Display/Error trait
//! coverage, and thorough validation of all public API surfaces.

use perl_qualified_name::{
    QualifiedNameError, is_valid_identifier_part, split_qualified_name,
    validate_perl_qualified_name,
};

// ---------------------------------------------------------------------------
// split_qualified_name — edge cases
// ---------------------------------------------------------------------------

#[test]
fn split_empty_string_returns_none_and_empty_bare() {
    let (pkg, bare) = split_qualified_name("");
    assert_eq!(pkg, None);
    assert_eq!(bare, "");
}

#[test]
fn split_single_identifier_returns_none_package() {
    let (pkg, bare) = split_qualified_name("foo");
    assert_eq!(pkg, None);
    assert_eq!(bare, "foo");
}

#[test]
fn split_two_segments_extracts_package_and_bare() {
    let (pkg, bare) = split_qualified_name("Foo::bar");
    assert_eq!(pkg, Some("Foo"));
    assert_eq!(bare, "bar");
}

#[test]
fn split_three_segments_uses_last_separator() {
    let (pkg, bare) = split_qualified_name("A::B::C");
    assert_eq!(pkg, Some("A::B"));
    assert_eq!(bare, "C");
}

#[test]
fn split_four_segments_preserves_nested_package() {
    let (pkg, bare) = split_qualified_name("W::X::Y::Z");
    assert_eq!(pkg, Some("W::X::Y"));
    assert_eq!(bare, "Z");
}

#[test]
fn split_trailing_separator_leaves_empty_bare() {
    let (pkg, bare) = split_qualified_name("Foo::");
    assert_eq!(pkg, Some("Foo"));
    assert_eq!(bare, "");
}

#[test]
fn split_leading_separator_leaves_empty_package_prefix() {
    let (pkg, bare) = split_qualified_name("::Bar");
    assert_eq!(pkg, Some(""));
    assert_eq!(bare, "Bar");
}

#[test]
fn split_only_separator_returns_both_empty() {
    let (pkg, bare) = split_qualified_name("::");
    assert_eq!(pkg, Some(""));
    assert_eq!(bare, "");
}

#[test]
fn split_double_separator_keeps_empty_between() {
    let (pkg, bare) = split_qualified_name("Foo::::Bar");
    assert_eq!(pkg, Some("Foo::"));
    assert_eq!(bare, "Bar");
}

#[test]
fn split_name_containing_single_colon_is_unqualified() {
    let (pkg, bare) = split_qualified_name("Foo:Bar");
    assert_eq!(pkg, None);
    assert_eq!(bare, "Foo:Bar");
}

#[test]
fn split_underscore_segments() {
    let (pkg, bare) = split_qualified_name("_Priv::_inner");
    assert_eq!(pkg, Some("_Priv"));
    assert_eq!(bare, "_inner");
}

// ---------------------------------------------------------------------------
// validate_perl_qualified_name — happy paths
// ---------------------------------------------------------------------------

#[test]
fn validate_simple_bare_name() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("foo")?;
    validate_perl_qualified_name("_bar")?;
    validate_perl_qualified_name("Baz123")?;
    Ok(())
}

#[test]
fn validate_two_segment_qualified_name() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("Foo::Bar")?;
    Ok(())
}

#[test]
fn validate_deeply_nested_qualified_name() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("A::B::C::D::E")?;
    Ok(())
}

#[test]
fn validate_unicode_identifiers() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("café")?;
    validate_perl_qualified_name("Ünit::Tëst")?;
    validate_perl_qualified_name("日本語")?;
    validate_perl_qualified_name("α::β::γ")?;
    Ok(())
}

#[test]
fn validate_underscored_names() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("_private")?;
    validate_perl_qualified_name("__double")?;
    validate_perl_qualified_name("_A::_B")?;
    Ok(())
}

#[test]
fn validate_mixed_case_and_digits() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("HTTP2::Client")?;
    validate_perl_qualified_name("v5_34")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_perl_qualified_name — error variant matching
// ---------------------------------------------------------------------------

#[test]
fn validate_empty_name_returns_empty_name_error() {
    let err = validate_perl_qualified_name("").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptyName);
}

#[test]
fn validate_dollar_sigil_returns_leading_sigil_error() {
    let err = validate_perl_qualified_name("$foo").unwrap_err();
    assert_eq!(err, QualifiedNameError::LeadingSigil('$'));
}

#[test]
fn validate_at_sigil_returns_leading_sigil_error() {
    let err = validate_perl_qualified_name("@array").unwrap_err();
    assert_eq!(err, QualifiedNameError::LeadingSigil('@'));
}

#[test]
fn validate_percent_sigil_returns_leading_sigil_error() {
    let err = validate_perl_qualified_name("%hash").unwrap_err();
    assert_eq!(err, QualifiedNameError::LeadingSigil('%'));
}

#[test]
fn validate_ampersand_sigil_returns_leading_sigil_error() {
    let err = validate_perl_qualified_name("&code").unwrap_err();
    assert_eq!(err, QualifiedNameError::LeadingSigil('&'));
}

#[test]
fn validate_star_sigil_returns_leading_sigil_error() {
    let err = validate_perl_qualified_name("*glob").unwrap_err();
    assert_eq!(err, QualifiedNameError::LeadingSigil('*'));
}

#[test]
fn validate_trailing_separator_returns_empty_segment() {
    let err = validate_perl_qualified_name("Foo::").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 1 });
}

#[test]
fn validate_leading_separator_returns_empty_segment_at_zero() {
    let err = validate_perl_qualified_name("::Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 0 });
}

#[test]
fn validate_double_separator_returns_empty_segment_in_middle() {
    let err = validate_perl_qualified_name("Foo::::Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 1 });
}

#[test]
fn validate_only_separator_returns_empty_segment_at_zero() {
    let err = validate_perl_qualified_name("::").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 0 });
}

#[test]
fn validate_digit_start_returns_invalid_segment() {
    let err = validate_perl_qualified_name("123abc").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

#[test]
fn validate_digit_start_in_second_segment_returns_invalid_segment() {
    let err = validate_perl_qualified_name("Foo::1bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 1 });
}

#[test]
fn validate_hyphenated_segment_returns_invalid_segment() {
    let err = validate_perl_qualified_name("Foo-Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

#[test]
fn validate_segment_with_space_returns_invalid_segment() {
    let err = validate_perl_qualified_name("Foo Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

#[test]
fn validate_segment_with_dot_returns_invalid_segment() {
    let err = validate_perl_qualified_name("Foo.pm").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

// ---------------------------------------------------------------------------
// QualifiedNameError — Display trait
// ---------------------------------------------------------------------------

#[test]
fn display_empty_name_error() {
    let msg = QualifiedNameError::EmptyName.to_string();
    assert_eq!(msg, "name is empty");
}

#[test]
fn display_leading_sigil_error() {
    let msg = QualifiedNameError::LeadingSigil('$').to_string();
    assert_eq!(msg, "qualified name cannot start with sigil '$'");
}

#[test]
fn display_leading_sigil_error_star() {
    let msg = QualifiedNameError::LeadingSigil('*').to_string();
    assert_eq!(msg, "qualified name cannot start with sigil '*'");
}

#[test]
fn display_empty_segment_error() {
    let msg = QualifiedNameError::EmptySegment { index: 2 }.to_string();
    assert_eq!(msg, "segment 2 is empty (leading/trailing/double separator)");
}

#[test]
fn display_invalid_segment_error() {
    let msg = QualifiedNameError::InvalidSegment { index: 3 }.to_string();
    assert_eq!(msg, "segment 3 is not a valid identifier");
}

// ---------------------------------------------------------------------------
// QualifiedNameError — std::error::Error trait
// ---------------------------------------------------------------------------

#[test]
fn error_trait_is_implemented() {
    let err: Box<dyn std::error::Error> = Box::new(QualifiedNameError::EmptyName);
    assert_eq!(err.to_string(), "name is empty");
}

// ---------------------------------------------------------------------------
// QualifiedNameError — derived traits (Debug, Clone, Copy, PartialEq, Eq)
// ---------------------------------------------------------------------------

#[test]
fn error_debug_format_is_readable() {
    let dbg = format!("{:?}", QualifiedNameError::EmptyName);
    assert!(dbg.contains("EmptyName"));

    let dbg = format!("{:?}", QualifiedNameError::LeadingSigil('$'));
    assert!(dbg.contains("LeadingSigil"));
    assert!(dbg.contains('$'));
}

#[test]
fn error_clone_produces_equal_value() {
    let orig = QualifiedNameError::EmptySegment { index: 5 };
    let cloned = orig;
    assert_eq!(orig, cloned);
}

#[test]
fn error_equality_distinguishes_variants() {
    assert_ne!(QualifiedNameError::EmptyName, QualifiedNameError::LeadingSigil('$'));
    assert_ne!(
        QualifiedNameError::EmptySegment { index: 0 },
        QualifiedNameError::EmptySegment { index: 1 }
    );
    assert_ne!(
        QualifiedNameError::EmptySegment { index: 0 },
        QualifiedNameError::InvalidSegment { index: 0 }
    );
}

// ---------------------------------------------------------------------------
// is_valid_identifier_part — thorough coverage
// ---------------------------------------------------------------------------

#[test]
fn identifier_part_empty_is_invalid() {
    assert!(!is_valid_identifier_part(""));
}

#[test]
fn identifier_part_starts_with_digit_is_invalid() {
    assert!(!is_valid_identifier_part("0abc"));
    assert!(!is_valid_identifier_part("9"));
}

#[test]
fn identifier_part_starts_with_letter_is_valid() {
    assert!(is_valid_identifier_part("a"));
    assert!(is_valid_identifier_part("Z"));
    assert!(is_valid_identifier_part("abc123"));
}

#[test]
fn identifier_part_starts_with_underscore_is_valid() {
    assert!(is_valid_identifier_part("_"));
    assert!(is_valid_identifier_part("_foo"));
    assert!(is_valid_identifier_part("__bar__"));
    assert!(is_valid_identifier_part("_123"));
}

#[test]
fn identifier_part_with_hyphen_is_invalid() {
    assert!(!is_valid_identifier_part("foo-bar"));
}

#[test]
fn identifier_part_with_space_is_invalid() {
    assert!(!is_valid_identifier_part("foo bar"));
}

#[test]
fn identifier_part_with_special_chars_is_invalid() {
    assert!(!is_valid_identifier_part("foo!"));
    assert!(!is_valid_identifier_part("foo@bar"));
    assert!(!is_valid_identifier_part("a.b"));
    assert!(!is_valid_identifier_part("a/b"));
}

#[test]
fn identifier_part_unicode_alphabetic_start_is_valid() {
    assert!(is_valid_identifier_part("ñ"));
    assert!(is_valid_identifier_part("Ω"));
    assert!(is_valid_identifier_part("αβγ"));
    assert!(is_valid_identifier_part("漢字"));
}

#[test]
fn identifier_part_unicode_digit_continuation_is_valid() {
    // Alphabetic start, then unicode alphanumeric continuation
    assert!(is_valid_identifier_part("test٣")); // Arabic-Indic digit 3
}

#[test]
fn identifier_part_pure_numeric_string_is_invalid() {
    assert!(!is_valid_identifier_part("123"));
    assert!(!is_valid_identifier_part("42"));
}

#[test]
fn identifier_part_single_underscore_is_valid() {
    assert!(is_valid_identifier_part("_"));
}

#[test]
fn identifier_part_mixed_underscore_and_digits() {
    assert!(is_valid_identifier_part("a_1_b_2"));
    assert!(is_valid_identifier_part("_0_0"));
}

// ---------------------------------------------------------------------------
// split + validate round-trip consistency
// ---------------------------------------------------------------------------

#[test]
fn split_then_reconstruct_equals_original() {
    let names = ["foo", "Foo::Bar", "A::B::C", "X::Y::Z::W", "_priv::_inner"];
    for name in names {
        let (pkg, bare) = split_qualified_name(name);
        let reconstructed = match pkg {
            Some(p) => format!("{p}::{bare}"),
            None => bare.to_string(),
        };
        assert_eq!(reconstructed, name, "round-trip failed for {name}");
    }
}

#[test]
fn all_valid_names_split_into_non_empty_bare() -> Result<(), QualifiedNameError> {
    let names = ["foo", "Foo::Bar", "A::B::C", "café", "Ünit::α"];
    for name in names {
        validate_perl_qualified_name(name)?;
        let (_pkg, bare) = split_qualified_name(name);
        assert!(!bare.is_empty(), "bare should be non-empty for valid name {name}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary / stress tests
// ---------------------------------------------------------------------------

#[test]
fn validate_single_char_identifiers() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("a")?;
    validate_perl_qualified_name("Z")?;
    validate_perl_qualified_name("_")?;
    Ok(())
}

#[test]
fn validate_long_qualified_name() -> Result<(), QualifiedNameError> {
    let segments: Vec<String> = (0..20).map(|i| format!("Seg{i}")).collect();
    let name = segments.join("::");
    validate_perl_qualified_name(&name)?;

    let (pkg, bare) = split_qualified_name(&name);
    assert!(pkg.is_some());
    assert_eq!(bare, "Seg19");
    Ok(())
}

#[test]
fn validate_segment_with_only_digits_after_valid_start() -> Result<(), QualifiedNameError> {
    validate_perl_qualified_name("a123")?;
    validate_perl_qualified_name("_999")?;
    Ok(())
}

#[test]
fn validate_rejects_sigils_even_before_valid_names() {
    assert!(validate_perl_qualified_name("$Foo::Bar").is_err());
    assert!(validate_perl_qualified_name("@Foo::Bar").is_err());
    assert!(validate_perl_qualified_name("%Foo::Bar").is_err());
    assert!(validate_perl_qualified_name("&Foo::Bar").is_err());
    assert!(validate_perl_qualified_name("*Foo::Bar").is_err());
}

#[test]
fn validate_triple_separator_produces_multiple_empty_segments() {
    // "A::::::B" splits into ["A", "", "", "B"] by "::" — first empty at index 1
    let err = validate_perl_qualified_name("A::::::B").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 1 });
}
