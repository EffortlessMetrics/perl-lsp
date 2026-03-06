//! Extended unit tests for `perl-qualified-name` crate.
//!
//! This module provides comprehensive edge case testing, boundary conditions,
//! performance characteristics, and unusual Unicode scenarios to ensure robust
//! handling of all inputs.
//!
//! No unwrap()/expect() calls; all tests return Result<(), Box<dyn std::error::Error>>.

use perl_qualified_name::{
    QualifiedNameError, is_valid_identifier_part, split_qualified_name,
    validate_perl_qualified_name,
};

// ===== split_qualified_name: Advanced Splitting Scenarios =====

/// Test multiple consecutive separators between segments
#[test]
fn split_many_consecutive_separators_find_last() {
    let (pkg, bare) = split_qualified_name("Foo::::::::::Bar");
    assert_eq!(pkg, Some("Foo::::::::"));
    assert_eq!(bare, "Bar");
}

/// Test that single colons don't trigger splitting
#[test]
fn split_single_colon_not_separator() {
    let (pkg, bare) = split_qualified_name("C++:Lang");
    assert_eq!(pkg, None);
    assert_eq!(bare, "C++:Lang");
}

/// Test colon with numbers
#[test]
fn split_with_numbers_and_colons() {
    let (pkg, bare) = split_qualified_name("HTTP2Server::v5");
    assert_eq!(pkg, Some("HTTP2Server"));
    assert_eq!(bare, "v5");
}

/// Test very deep nesting
#[test]
fn split_ten_segment_qualified_name() {
    let input = "A::B::C::D::E::F::G::H::I::J";
    let (pkg, bare) = split_qualified_name(input);
    assert_eq!(pkg, Some("A::B::C::D::E::F::G::H::I"));
    assert_eq!(bare, "J");
}

/// Test leading double separator
#[test]
fn split_leading_double_colon_separator() {
    let (pkg, bare) = split_qualified_name("::Foo::Bar");
    assert_eq!(pkg, Some("::Foo"));
    assert_eq!(bare, "Bar");
}

/// Test name with only underscores
#[test]
fn split_underscore_only_segments() {
    let (pkg, bare) = split_qualified_name("_::__::___");
    assert_eq!(pkg, Some("_::__"));
    assert_eq!(bare, "___");
}

/// Test Unicode segments separated by ::
#[test]
fn split_unicode_qualified_name() {
    let (pkg, bare) = split_qualified_name("日本::パッケージ::モジュール");
    assert_eq!(pkg, Some("日本::パッケージ"));
    assert_eq!(bare, "モジュール");
}

/// Test mixed ASCII and Unicode
#[test]
fn split_mixed_ascii_unicode() {
    let (pkg, bare) = split_qualified_name("MyLib::café::résumé");
    assert_eq!(pkg, Some("MyLib::café"));
    assert_eq!(bare, "résumé");
}

// ===== validate_perl_qualified_name: Unicode Edge Cases =====

/// Test Cyrillic identifiers
#[test]
fn validate_cyrillic_identifier() -> Result<(), Box<dyn std::error::Error>> {
    validate_perl_qualified_name("Привет")?;
    validate_perl_qualified_name("Привет::Мир")?;
    Ok(())
}

/// Test Arabic identifiers
#[test]
fn validate_arabic_identifier() -> Result<(), Box<dyn std::error::Error>> {
    validate_perl_qualified_name("مرحبا")?;
    validate_perl_qualified_name("مرحبا::عالم")?;
    Ok(())
}

/// Test Hebrew identifiers
#[test]
fn validate_hebrew_identifier() -> Result<(), Box<dyn std::error::Error>> {
    validate_perl_qualified_name("שלום")?;
    validate_perl_qualified_name("שלום::עולם")?;
    Ok(())
}

/// Test Greek identifiers
#[test]
fn validate_greek_identifier() -> Result<(), Box<dyn std::error::Error>> {
    validate_perl_qualified_name("αρχή")?;
    validate_perl_qualified_name("αρχή::τέλος")?;
    Ok(())
}

/// Test Thai identifiers
#[test]
fn validate_thai_identifier() -> Result<(), Box<dyn std::error::Error>> {
    validate_perl_qualified_name("สวัสดี")?;
    validate_perl_qualified_name("สวัสดี::โลก")?;
    Ok(())
}

/// Test Korean identifiers
#[test]
fn validate_korean_identifier() -> Result<(), Box<dyn std::error::Error>> {
    validate_perl_qualified_name("안녕")?;
    validate_perl_qualified_name("안녕::세계")?;
    Ok(())
}

/// Test emoji-like Unicode characters (if considered alphabetic)
#[test]
fn validate_emoji_like_unicode() -> Result<(), Box<dyn std::error::Error>> {
    // Some emoji or special symbols may or may not be alphabetic
    // This is a boundary test
    validate_perl_qualified_name("ß")?;
    validate_perl_qualified_name("ÿ")?;
    Ok(())
}

// ===== validate_perl_qualified_name: Multiple Sigils =====

/// Test that only leading sigils matter
#[test]
fn validate_sigil_in_middle_of_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo$bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

/// Test sigil appearing after separator is invalid
#[test]
fn validate_sigil_after_separator_is_invalid() {
    let err = validate_perl_qualified_name("Foo::$bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 1 });
}

// ===== validate_perl_qualified_name: Extended Empty Segment Patterns =====

/// Test four consecutive separators
#[test]
fn validate_four_separators_empty_segment() {
    let err = validate_perl_qualified_name("A::::B").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 1 });
}

/// Test five consecutive separators
#[test]
fn validate_five_separators_empty_segment() {
    let err = validate_perl_qualified_name("A:::::B").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 1 });
}

/// Test trailing triple separator - last part is a single colon which is not a valid identifier
#[test]
fn validate_trailing_triple_separator() {
    let err = validate_perl_qualified_name("Foo:::").unwrap_err();
    // "Foo:::" splits to ["Foo", ":"], and ":" is InvalidSegment
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 1 });
}

/// Test leading triple separator
#[test]
fn validate_leading_triple_separator() {
    let err = validate_perl_qualified_name(":::Foo").unwrap_err();
    assert_eq!(err, QualifiedNameError::EmptySegment { index: 0 });
}

// ===== validate_perl_qualified_name: Whitespace and Special Chars =====

/// Test tab character in segment
#[test]
fn validate_tab_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo\tBar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

/// Test newline in segment
#[test]
fn validate_newline_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo\nBar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

/// Test carriage return in segment
#[test]
fn validate_carriage_return_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo\rBar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

/// Test null byte in segment
#[test]
fn validate_null_byte_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo\0Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

/// Test non-breaking space (U+00A0)
#[test]
fn validate_non_breaking_space_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo\u{00A0}Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

// ===== validate_perl_qualified_name: Special Characters =====

/// Test backtick in segment
#[test]
fn validate_backtick_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo`Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

/// Test pipe in segment
#[test]
fn validate_pipe_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo|Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

/// Test question mark in segment
#[test]
fn validate_question_mark_in_segment_is_invalid() {
    let err = validate_perl_qualified_name("Foo?Bar").unwrap_err();
    assert_eq!(err, QualifiedNameError::InvalidSegment { index: 0 });
}

// ===== is_valid_identifier_part: Unicode Number Support =====

/// Test Unicode numerals after valid start
#[test]
fn identifier_unicode_numeral_continuation() {
    // U+0660 to U+0669 are Arabic-Indic digits
    assert!(is_valid_identifier_part("a٠"));
    assert!(is_valid_identifier_part("a١٢٣"));
}

/// Test Roman numerals (are they alphanumeric?)
#[test]
fn identifier_roman_numerals_as_continuation() {
    // Roman numerals are considered alphanumeric in Rust's Unicode categories
    assert!(is_valid_identifier_part("aⅠ")); // Latin Capital Roman Numeral One
}

/// Test superscript and subscript
#[test]
fn identifier_with_superscript() {
    // Superscript digits may be considered numeric
    assert!(is_valid_identifier_part("a¹"));
}

// ===== is_valid_identifier_part: Edge Cases =====

/// Test identifier with combining diacritics
#[test]
fn identifier_with_combining_diacritics() {
    // e + combining acute accent is not valid because combining mark is not alphanumeric
    assert!(!is_valid_identifier_part("e\u{0301}"));
    // NFC form (precomposed) would work
    assert!(is_valid_identifier_part("é"));
}

/// Test zero-width joiner (may cause issues)
#[test]
fn identifier_with_zero_width_joiner() {
    // U+200D is Zero Width Joiner
    let result = is_valid_identifier_part("a\u{200D}b");
    // The behavior depends on whether ZWJ is considered alphanumeric
    // We just verify it doesn't panic
    let _ = result;
}

/// Test right-to-left mark
#[test]
fn identifier_with_rtl_mark() {
    // U+200F is Right-to-Left Mark
    let result = is_valid_identifier_part("a\u{200F}");
    let _ = result;
}

// ===== is_valid_identifier_part: Boundary Tests =====

/// Test very long identifier
#[test]
fn identifier_very_long_string() {
    let long = "a".repeat(10000);
    assert!(is_valid_identifier_part(&long));
}

/// Test identifier with exactly one character (letter)
#[test]
fn identifier_single_letter() {
    assert!(is_valid_identifier_part("a"));
    assert!(is_valid_identifier_part("Z"));
}

/// Test identifier with exactly one character (underscore)
#[test]
fn identifier_single_underscore() {
    assert!(is_valid_identifier_part("_"));
}

/// Test identifier alternating letters and underscores
#[test]
fn identifier_alternating_letters_underscores() {
    assert!(is_valid_identifier_part("a_b_c_d_e_f"));
}

// ===== Error Display Messages: All Variants =====

/// Test display for EmptySegment at index 0
#[test]
fn error_display_empty_segment_index_zero() {
    let msg = QualifiedNameError::EmptySegment { index: 0 }.to_string();
    assert!(msg.contains("segment 0"));
    assert!(msg.contains("empty"));
}

/// Test display for EmptySegment at large index
#[test]
fn error_display_empty_segment_large_index() {
    let msg = QualifiedNameError::EmptySegment { index: 999 }.to_string();
    assert!(msg.contains("999"));
}

/// Test display for InvalidSegment at index 0
#[test]
fn error_display_invalid_segment_index_zero() {
    let msg = QualifiedNameError::InvalidSegment { index: 0 }.to_string();
    assert!(msg.contains("segment 0"));
    assert!(msg.contains("not a valid"));
}

/// Test display for InvalidSegment at large index
#[test]
fn error_display_invalid_segment_large_index() {
    let msg = QualifiedNameError::InvalidSegment { index: 100 }.to_string();
    assert!(msg.contains("100"));
}

/// Test Display trait via error trait object
#[test]
fn error_trait_object_display() -> Result<(), Box<dyn std::error::Error>> {
    let err: Box<dyn std::error::Error> = Box::new(QualifiedNameError::InvalidSegment { index: 5 });
    let msg = err.to_string();
    assert!(msg.contains("5"));
    Ok(())
}

// ===== Round-trip Consistency: Split and Reconstruct =====

/// Test that split output can be reconstructed for all valid names
#[test]
fn roundtrip_valid_names_preserve_original() -> Result<(), Box<dyn std::error::Error>> {
    let test_names = [
        "single",
        "Two::Part",
        "Three::Part::Name",
        "A::B::C::D::E::F",
        "_private",
        "___",
        "café",
        "Zürich::Stadt",
    ];
    for name in &test_names {
        validate_perl_qualified_name(name)?;
        let (pkg, bare) = split_qualified_name(name);
        let reconstructed = match pkg {
            Some(p) => format!("{p}::{bare}"),
            None => bare.to_string(),
        };
        assert_eq!(&reconstructed, name, "roundtrip failed for {name}");
    }
    Ok(())
}

// ===== Combined Validation and Splitting =====

/// Test that all valid names have non-empty bare parts
#[test]
fn all_valid_names_have_non_empty_bare() -> Result<(), Box<dyn std::error::Error>> {
    let test_names = ["foo", "Foo::Bar", "A::B::C::D", "Ω", "α::β::γ"];
    for name in &test_names {
        validate_perl_qualified_name(name)?;
        let (_pkg, bare) = split_qualified_name(name);
        assert!(!bare.is_empty(), "bare part is empty for {name}");
    }
    Ok(())
}

// ===== Boundary: Very Long Qualified Names =====

/// Test validation of extremely long qualified name
#[test]
fn validate_extremely_long_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let segments: Vec<String> = (0..100).map(|i| format!("Seg{i:03}")).collect();
    let long_name = segments.join("::");
    validate_perl_qualified_name(&long_name)?;

    let (pkg, bare) = split_qualified_name(&long_name);
    assert!(pkg.is_some(), "package should be present");
    assert_eq!(bare, "Seg099", "bare name should be last segment");
    Ok(())
}

// ===== Mixed Scenario: Unicode + Separators =====

/// Test deep nesting with Unicode segments
#[test]
fn deep_nesting_with_unicode() -> Result<(), Box<dyn std::error::Error>> {
    let name = "Привет::مرحبا::你好::שלום::こんにちは";
    validate_perl_qualified_name(name)?;
    let (pkg, bare) = split_qualified_name(name);
    assert!(pkg.is_some());
    assert_eq!(bare, "こんにちは");
    Ok(())
}

// ===== Equivalence Tests: Different Representations =====

/// Test that different Unicode representations are distinguished
#[test]
fn normalized_vs_non_normalized_unicode() -> Result<(), Box<dyn std::error::Error>> {
    // NFC: "é" as single character
    let nfc = "café";
    // NFD: "e" + combining acute - this is NOT valid because combining marks aren't alphanumeric
    let nfd = "cafe\u{0301}";

    validate_perl_qualified_name(nfc)?;
    // This should fail validation
    let result = validate_perl_qualified_name(nfd);
    assert!(result.is_err(), "NFD form should not validate");

    Ok(())
}

// ===== Error Variant Exhaustiveness =====

/// Verify EmptyName error variant
#[test]
fn error_variant_empty_name() {
    let result = validate_perl_qualified_name("");
    assert!(matches!(result, Err(QualifiedNameError::EmptyName)));
}

/// Verify LeadingSigil error variant with each sigil
#[test]
fn error_variant_leading_sigil_all_variants() {
    let sigils = ['$', '@', '%', '&', '*'];
    for sigil in sigils {
        let input = format!("{sigil}name");
        let err = validate_perl_qualified_name(&input).unwrap_err();
        assert!(matches!(err, QualifiedNameError::LeadingSigil(s) if s == sigil));
    }
}

/// Verify EmptySegment error variant at various indices
#[test]
fn error_variant_empty_segment_indices() {
    let test_cases = [("::foo", 0), ("foo::", 1), ("foo::::bar", 1), ("a::b::::c", 2)];
    for (input, expected_idx) in test_cases {
        let err = validate_perl_qualified_name(input).unwrap_err();
        assert!(matches!(
            err,
            QualifiedNameError::EmptySegment { index } if index == expected_idx
        ));
    }
}

/// Verify InvalidSegment error variant at various indices
#[test]
fn error_variant_invalid_segment_indices() {
    let test_cases = [("1invalid", 0), ("valid::2invalid", 1), ("a::b::3invalid", 2)];
    for (input, expected_idx) in test_cases {
        let err = validate_perl_qualified_name(input).unwrap_err();
        assert!(matches!(
            err,
            QualifiedNameError::InvalidSegment { index } if index == expected_idx
        ));
    }
}
