//! Comprehensive unit tests for `perl-module-name` public API.

use std::borrow::Cow;

use perl_module::name::{
    legacy_package_separator, module_variant_pairs, normalize_package_separator,
};

// ──────────────────────────────────────────────────────────────
// normalize_package_separator
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_legacy_single_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
    Ok(())
}

#[test]
fn normalize_already_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo::Bar");
    assert_eq!(result, "Foo::Bar");
    // Should borrow when no transformation needed
    assert!(matches!(result, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn normalize_multiple_legacy_separators() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("A'B'C'D"), "A::B::C::D");
    Ok(())
}

#[test]
fn normalize_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("");
    assert_eq!(result, "");
    assert!(matches!(result, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn normalize_no_separator() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("strict");
    assert_eq!(result, "strict");
    assert!(matches!(result, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn normalize_leading_legacy_separator() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("'Foo"), "::Foo");
    Ok(())
}

#[test]
fn normalize_trailing_legacy_separator() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("Foo'"), "Foo::");
    Ok(())
}

#[test]
fn normalize_consecutive_legacy_separators() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("A''B"), "A::::B");
    Ok(())
}

#[test]
fn normalize_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    // Contains both ' and :: — only ' should be replaced
    assert_eq!(normalize_package_separator("A::B'C"), "A::B::C");
    Ok(())
}

#[test]
fn normalize_deeply_nested_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_package_separator("My'Very'Deep'Module'Name"),
        "My::Very::Deep::Module::Name"
    );
    Ok(())
}

#[test]
fn normalize_returns_owned_when_transformed() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo'Bar");
    assert!(matches!(result, Cow::Owned(_)));
    Ok(())
}

#[test]
fn normalize_unicode_module_name() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_package_separator("Ünîcödé'Módule"),
        "Ünîcödé::Módule"
    );
    Ok(())
}

#[test]
fn normalize_single_quote_only() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("'"), "::");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// legacy_package_separator
// ──────────────────────────────────────────────────────────────

#[test]
fn legacy_single_canonical_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
    Ok(())
}

#[test]
fn legacy_already_legacy() -> Result<(), Box<dyn std::error::Error>> {
    let result = legacy_package_separator("Foo'Bar");
    assert_eq!(result, "Foo'Bar");
    assert!(matches!(result, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn legacy_multiple_canonical_separators() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("A::B::C::D"), "A'B'C'D");
    Ok(())
}

#[test]
fn legacy_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let result = legacy_package_separator("");
    assert_eq!(result, "");
    assert!(matches!(result, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn legacy_no_separator() -> Result<(), Box<dyn std::error::Error>> {
    let result = legacy_package_separator("warnings");
    assert_eq!(result, "warnings");
    assert!(matches!(result, Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn legacy_leading_canonical_separator() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("::Foo"), "'Foo");
    Ok(())
}

#[test]
fn legacy_trailing_canonical_separator() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("Foo::"), "Foo'");
    Ok(())
}

#[test]
fn legacy_consecutive_canonical_separators() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("A::::B"), "A''B");
    Ok(())
}

#[test]
fn legacy_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    // Contains both :: and ' — only :: should be replaced
    assert_eq!(legacy_package_separator("A'B::C"), "A'B'C");
    Ok(())
}

#[test]
fn legacy_deeply_nested_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        legacy_package_separator("My::Very::Deep::Module::Name"),
        "My'Very'Deep'Module'Name"
    );
    Ok(())
}

#[test]
fn legacy_returns_owned_when_transformed() -> Result<(), Box<dyn std::error::Error>> {
    let result = legacy_package_separator("Foo::Bar");
    assert!(matches!(result, Cow::Owned(_)));
    Ok(())
}

#[test]
fn legacy_unicode_module_name() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        legacy_package_separator("Ünîcödé::Módule"),
        "Ünîcödé'Módule"
    );
    Ok(())
}

#[test]
fn legacy_double_colon_only() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("::"), "'");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Roundtrip: normalize ↔ legacy
// ──────────────────────────────────────────────────────────────

#[test]
fn roundtrip_normalize_then_legacy() -> Result<(), Box<dyn std::error::Error>> {
    let input = "Foo'Bar'Baz";
    let canonical = normalize_package_separator(input);
    let back = legacy_package_separator(&canonical);
    assert_eq!(back, input);
    Ok(())
}

#[test]
fn roundtrip_legacy_then_normalize() -> Result<(), Box<dyn std::error::Error>> {
    let input = "Foo::Bar::Baz";
    let legacy = legacy_package_separator(input);
    let back = normalize_package_separator(&legacy);
    assert_eq!(back, input);
    Ok(())
}

#[test]
fn roundtrip_bare_name_is_identity() -> Result<(), Box<dyn std::error::Error>> {
    let input = "strict";
    assert_eq!(normalize_package_separator(input).as_ref(), input);
    assert_eq!(legacy_package_separator(input).as_ref(), input);
    Ok(())
}

#[test]
fn roundtrip_empty_is_identity() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("").as_ref(), "");
    assert_eq!(legacy_package_separator("").as_ref(), "");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// module_variant_pairs
// ──────────────────────────────────────────────────────────────

#[test]
fn variant_pairs_canonical_inputs() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Foo::Bar", "New::Path");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("Foo::Bar".to_string(), "New::Path".to_string()));
    assert_eq!(pairs[1], ("Foo'Bar".to_string(), "New'Path".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_legacy_inputs() -> Result<(), Box<dyn std::error::Error>> {
    // Legacy inputs get normalized first, then both variants produced
    let pairs = module_variant_pairs("Foo'Bar", "New'Path");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("Foo::Bar".to_string(), "New::Path".to_string()));
    assert_eq!(pairs[1], ("Foo'Bar".to_string(), "New'Path".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_mixed_inputs() -> Result<(), Box<dyn std::error::Error>> {
    // One legacy, one canonical
    let pairs = module_variant_pairs("Old'Mod", "New::Mod");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("Old::Mod".to_string(), "New::Mod".to_string()));
    assert_eq!(pairs[1], ("Old'Mod".to_string(), "New'Mod".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_bare_names_deduplicated() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("strict", "warnings");
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], ("strict".to_string(), "warnings".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_empty_names() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("", "");
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], (String::new(), String::new()));
    Ok(())
}

#[test]
fn variant_pairs_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("A::B::C::D", "W::X::Y::Z");
    assert_eq!(pairs.len(), 2);
    assert_eq!(
        pairs[0],
        ("A::B::C::D".to_string(), "W::X::Y::Z".to_string())
    );
    assert_eq!(pairs[1], ("A'B'C'D".to_string(), "W'X'Y'Z".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_one_bare_one_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("strict", "My::Strict");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("strict".to_string(), "My::Strict".to_string()));
    assert_eq!(pairs[1], ("strict".to_string(), "My'Strict".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_same_name_rename() -> Result<(), Box<dyn std::error::Error>> {
    // Renaming to itself should still produce valid pairs
    let pairs = module_variant_pairs("My::Mod", "My::Mod");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("My::Mod".to_string(), "My::Mod".to_string()));
    assert_eq!(pairs[1], ("My'Mod".to_string(), "My'Mod".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_canonical_always_first() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Pkg'Sub", "New'Sub");
    // First pair should always be canonical form
    assert!(pairs[0].0.contains("::"));
    assert!(pairs[0].1.contains("::"));
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Edge cases: single-character components
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_single_char_components() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("A'B"), "A::B");
    Ok(())
}

#[test]
fn legacy_single_char_components() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("A::B"), "A'B");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Edge cases: whitespace and special characters
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_with_whitespace_around_separator() -> Result<(), Box<dyn std::error::Error>> {
    // Whitespace is preserved; only quote chars are replaced
    assert_eq!(normalize_package_separator("Foo ' Bar"), "Foo :: Bar");
    Ok(())
}

#[test]
fn legacy_with_spaces_in_name() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("Foo :: Bar"), "Foo ' Bar");
    Ok(())
}

#[test]
fn normalize_numeric_components() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_package_separator("Perl5'Module'V2"),
        "Perl5::Module::V2"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Real-world Perl module names
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_cpan_style_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_package_separator("File'Spec'Functions"),
        "File::Spec::Functions"
    );
    Ok(())
}

#[test]
fn legacy_cpan_style_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        legacy_package_separator("File::Spec::Functions"),
        "File'Spec'Functions"
    );
    Ok(())
}

#[test]
fn variant_pairs_cpan_rename() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("CGI::Cookie", "HTTP::Cookie");
    assert_eq!(pairs.len(), 2);
    assert_eq!(
        pairs[0],
        ("CGI::Cookie".to_string(), "HTTP::Cookie".to_string())
    );
    assert_eq!(
        pairs[1],
        ("CGI'Cookie".to_string(), "HTTP'Cookie".to_string())
    );
    Ok(())
}

#[test]
fn variant_pairs_moose_like_module() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Moose::Role", "Moo::Role");
    assert_eq!(pairs.len(), 2);
    assert_eq!(
        pairs[0],
        ("Moose::Role".to_string(), "Moo::Role".to_string())
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Cow borrow semantics contract
// ──────────────────────────────────────────────────────────────

#[test]
fn cow_borrowed_when_no_transformation_normalize() -> Result<(), Box<dyn std::error::Error>> {
    let names = ["Foo::Bar", "strict", "", "A::B::C"];
    for name in &names {
        let result = normalize_package_separator(name);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "Expected Cow::Borrowed for normalize({name:?}), got Cow::Owned"
        );
    }
    Ok(())
}

#[test]
fn cow_borrowed_when_no_transformation_legacy() -> Result<(), Box<dyn std::error::Error>> {
    let names = ["Foo'Bar", "strict", "", "A'B'C"];
    for name in &names {
        let result = legacy_package_separator(name);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "Expected Cow::Borrowed for legacy({name:?}), got Cow::Owned"
        );
    }
    Ok(())
}

#[test]
fn cow_owned_when_transformation_needed_normalize() -> Result<(), Box<dyn std::error::Error>> {
    let names = ["Foo'Bar", "A'B'C", "'"];
    for name in &names {
        let result = normalize_package_separator(name);
        assert!(
            matches!(result, Cow::Owned(_)),
            "Expected Cow::Owned for normalize({name:?}), got Cow::Borrowed"
        );
    }
    Ok(())
}

#[test]
fn cow_owned_when_transformation_needed_legacy() -> Result<(), Box<dyn std::error::Error>> {
    let names = ["Foo::Bar", "A::B::C", "::"];
    for name in &names {
        let result = legacy_package_separator(name);
        assert!(
            matches!(result, Cow::Owned(_)),
            "Expected Cow::Owned for legacy({name:?}), got Cow::Borrowed"
        );
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Idempotency: applying the same transform twice yields same result
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = ["Foo'Bar", "A::B::C", "strict", "", "X'Y::Z"];
    for input in &inputs {
        let once = normalize_package_separator(input).into_owned();
        let twice = normalize_package_separator(&once).into_owned();
        assert_eq!(once, twice, "normalize not idempotent for {input:?}");
    }
    Ok(())
}

#[test]
fn legacy_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = ["Foo::Bar", "A'B'C", "strict", "", "X'Y::Z"];
    for input in &inputs {
        let once = legacy_package_separator(input).into_owned();
        let twice = legacy_package_separator(&once).into_owned();
        assert_eq!(once, twice, "legacy not idempotent for {input:?}");
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Stability: variant_pairs output is deterministic
// ──────────────────────────────────────────────────────────────

#[test]
fn variant_pairs_deterministic_across_calls() -> Result<(), Box<dyn std::error::Error>> {
    let a = module_variant_pairs("Foo::Bar", "Baz::Qux");
    let b = module_variant_pairs("Foo::Bar", "Baz::Qux");
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn variant_pairs_legacy_input_matches_canonical_input() -> Result<(), Box<dyn std::error::Error>> {
    let from_canonical = module_variant_pairs("Foo::Bar", "Baz::Qux");
    let from_legacy = module_variant_pairs("Foo'Bar", "Baz'Qux");
    assert_eq!(from_canonical, from_legacy);
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Real-world CPAN module names
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_dbi_module() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("DBI");
    assert!(matches!(result, Cow::Borrowed(_)));
    assert_eq!(result, "DBI");
    Ok(())
}

#[test]
fn normalize_dbd_mysql() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("DBD'mysql"), "DBD::mysql");
    Ok(())
}

#[test]
fn legacy_test_more() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("Test::More"), "Test'More");
    Ok(())
}

#[test]
fn variant_pairs_lwp_useragent() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("LWP::UserAgent", "HTTP::Tiny");
    assert_eq!(pairs.len(), 2);
    assert_eq!(
        pairs[0],
        ("LWP::UserAgent".to_string(), "HTTP::Tiny".to_string())
    );
    assert_eq!(
        pairs[1],
        ("LWP'UserAgent".to_string(), "HTTP'Tiny".to_string())
    );
    Ok(())
}

#[test]
fn variant_pairs_datetime_format() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("DateTime::Format::Strptime", "Time::Piece");
    assert_eq!(pairs.len(), 2);
    assert_eq!(
        pairs[0],
        (
            "DateTime::Format::Strptime".to_string(),
            "Time::Piece".to_string()
        )
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Boundary: very long module names
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_very_long_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let segments: Vec<&str> = (0..20).map(|_| "Segment").collect();
    let legacy = segments.join("'");
    let canonical = segments.join("::");
    assert_eq!(normalize_package_separator(&legacy), canonical);
    Ok(())
}

#[test]
fn legacy_very_long_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let segments: Vec<&str> = (0..20).map(|_| "Segment").collect();
    let canonical = segments.join("::");
    let legacy = segments.join("'");
    assert_eq!(legacy_package_separator(&canonical), legacy);
    Ok(())
}

#[test]
fn variant_pairs_very_long_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let segments: Vec<&str> = (0..10).map(|_| "A").collect();
    let canonical = segments.join("::");
    let pairs = module_variant_pairs(&canonical, &canonical);
    assert_eq!(pairs.len(), 2);
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Separator-only strings
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_multiple_quotes_only() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("'''"), "::::::");
    Ok(())
}

#[test]
fn legacy_multiple_colons_only() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("::::::"), "'''");
    Ok(())
}

#[test]
fn variant_pairs_separator_only() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("::", "::");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("::".to_string(), "::".to_string()));
    assert_eq!(pairs[1], ("'".to_string(), "'".to_string()));
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Asymmetric variant pairs: one has separator, other doesn't
// ──────────────────────────────────────────────────────────────

#[test]
fn variant_pairs_old_qualified_new_bare() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("My::Module", "newname");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("My::Module".to_string(), "newname".to_string()));
    assert_eq!(pairs[1], ("My'Module".to_string(), "newname".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_old_bare_new_bare() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("oldname", "newname");
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], ("oldname".to_string(), "newname".to_string()));
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Normalize preserves non-separator characters exactly
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_preserves_underscores() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_package_separator("My_Module'Sub_Pkg"),
        "My_Module::Sub_Pkg"
    );
    Ok(())
}

#[test]
fn normalize_preserves_hyphens() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_package_separator("My-Module'Sub"),
        "My-Module::Sub"
    );
    Ok(())
}

#[test]
fn normalize_preserves_digits() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_package_separator("Module123'V456"),
        "Module123::V456"
    );
    Ok(())
}

#[test]
fn legacy_preserves_underscores() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        legacy_package_separator("My_Module::Sub_Pkg"),
        "My_Module'Sub_Pkg"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Cow lifetime: borrowed result references original input
// ──────────────────────────────────────────────────────────────

#[test]
fn cow_borrowed_points_to_original_normalize() -> Result<(), Box<dyn std::error::Error>> {
    let input = "Foo::Bar";
    let result = normalize_package_separator(input);
    if let Cow::Borrowed(s) = result {
        assert!(std::ptr::eq(s.as_bytes(), input.as_bytes()));
    }
    Ok(())
}

#[test]
fn cow_borrowed_points_to_original_legacy() -> Result<(), Box<dyn std::error::Error>> {
    let input = "Foo'Bar";
    let result = legacy_package_separator(input);
    if let Cow::Borrowed(s) = result {
        assert!(std::ptr::eq(s.as_bytes(), input.as_bytes()));
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Comparison: Cow<str> equality with &str
// ──────────────────────────────────────────────────────────────

#[test]
fn cow_result_compares_equal_to_str() -> Result<(), Box<dyn std::error::Error>> {
    let borrowed = normalize_package_separator("Foo::Bar");
    let owned = normalize_package_separator("Foo'Bar");
    assert_eq!(borrowed, "Foo::Bar");
    assert_eq!(owned, "Foo::Bar");
    assert_eq!(borrowed, owned);
    Ok(())
}

#[test]
fn cow_result_can_be_used_as_str_ref() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("A'B");
    let s: &str = &result;
    assert_eq!(s, "A::B");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Variant pairs: vector content guarantees
// ──────────────────────────────────────────────────────────────

#[test]
fn variant_pairs_always_has_at_least_one() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = [
        ("", ""),
        ("strict", "warnings"),
        ("A::B", "C::D"),
        ("A'B", "C'D"),
    ];
    for (old, new) in &test_cases {
        let pairs = module_variant_pairs(old, new);
        assert!(
            !pairs.is_empty(),
            "variant_pairs({old:?}, {new:?}) returned empty vec"
        );
    }
    Ok(())
}

#[test]
fn variant_pairs_never_more_than_two() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = [
        ("", ""),
        ("strict", "warnings"),
        ("A::B", "C::D"),
        ("A'B", "C'D"),
        ("A'B::C", "D'E::F"),
    ];
    for (old, new) in &test_cases {
        let pairs = module_variant_pairs(old, new);
        assert!(
            pairs.len() <= 2,
            "variant_pairs({old:?}, {new:?}) returned {} pairs",
            pairs.len()
        );
    }
    Ok(())
}

#[test]
fn variant_pairs_first_element_is_always_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("X'Y'Z", "A'B'C");
    assert!(!pairs[0].0.contains('\''), "first pair old contains quote");
    assert!(!pairs[0].1.contains('\''), "first pair new contains quote");
    Ok(())
}

#[test]
fn variant_pairs_second_element_is_always_legacy() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("X::Y::Z", "A::B::C");
    if pairs.len() == 2 {
        assert!(!pairs[1].0.contains("::"), "second pair old contains ::");
        assert!(!pairs[1].1.contains("::"), "second pair new contains ::");
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Mixed legacy+canonical in single input
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_mixed_legacy_and_canonical() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("A'B::C'D"), "A::B::C::D");
    Ok(())
}

#[test]
fn legacy_mixed_canonical_and_legacy() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("A::B'C::D"), "A'B'C'D");
    Ok(())
}

#[test]
fn variant_pairs_mixed_input_normalizes() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("A'B::C", "X'Y::Z");
    assert_eq!(pairs[0].0, "A::B::C");
    assert_eq!(pairs[0].1, "X::Y::Z");
    if pairs.len() == 2 {
        assert_eq!(pairs[1].0, "A'B'C");
        assert_eq!(pairs[1].1, "X'Y'Z");
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Single-segment (no separator) module names
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_common_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    for pragma in &["strict", "warnings", "utf8", "feature", "constant"] {
        let result = normalize_package_separator(pragma);
        assert_eq!(result.as_ref(), *pragma);
        assert!(matches!(result, Cow::Borrowed(_)));
    }
    Ok(())
}

#[test]
fn legacy_common_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    for pragma in &["strict", "warnings", "utf8", "feature", "constant"] {
        let result = legacy_package_separator(pragma);
        assert_eq!(result.as_ref(), *pragma);
        assert!(matches!(result, Cow::Borrowed(_)));
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Adjacent separators
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_adjacent_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    // 3 quotes become 3 × "::" = 6 colons, plus the existing "::" = 8 colons
    // Actually: "'" → "::", then "::" stays, then "'" → "::"
    // So "'::'" = "::" + "::" + "::" = "::::::"
    assert_eq!(normalize_package_separator("'::'"), "::::::");
    Ok(())
}

#[test]
fn legacy_adjacent_colons_triple() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator(":::"), "':");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Cow into_owned
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_into_owned_produces_string() -> Result<(), Box<dyn std::error::Error>> {
    let owned: String = normalize_package_separator("A'B").into_owned();
    assert_eq!(owned, "A::B");
    Ok(())
}

#[test]
fn legacy_into_owned_produces_string() -> Result<(), Box<dyn std::error::Error>> {
    let owned: String = legacy_package_separator("A::B").into_owned();
    assert_eq!(owned, "A'B");
    Ok(())
}

#[test]
fn normalize_borrowed_into_owned_clones() -> Result<(), Box<dyn std::error::Error>> {
    let owned: String = normalize_package_separator("A::B").into_owned();
    assert_eq!(owned, "A::B");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Variant pairs: old == new (identity rename)
// ──────────────────────────────────────────────────────────────

#[test]
fn variant_pairs_identity_bare() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("strict", "strict");
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, pairs[0].1);
    Ok(())
}

#[test]
fn variant_pairs_identity_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("A::B::C", "A::B::C");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].0, pairs[0].1);
    assert_eq!(pairs[1].0, pairs[1].1);
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Variant pairs: depth mismatch
// ──────────────────────────────────────────────────────────────

#[test]
fn variant_pairs_different_depth() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("A::B", "X::Y::Z::W");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("A::B".to_string(), "X::Y::Z::W".to_string()));
    assert_eq!(pairs[1], ("A'B".to_string(), "X'Y'Z'W".to_string()));
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Unicode: multi-byte characters near separators
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_cjk_module_name() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("模块'名称"), "模块::名称");
    Ok(())
}

#[test]
fn legacy_cjk_module_name() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("模块::名称"), "模块'名称");
    Ok(())
}

#[test]
fn roundtrip_cjk_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let input = "模块'名称'子包";
    let canonical = normalize_package_separator(input);
    let back = legacy_package_separator(&canonical);
    assert_eq!(back, input);
    Ok(())
}

#[test]
fn normalize_emoji_module_name() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("🦀'Module"), "🦀::Module");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Variant pairs with Unicode
// ──────────────────────────────────────────────────────────────

#[test]
fn variant_pairs_unicode() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Ünïcödë::Mödülé", "Néw::Nàmé");
    assert_eq!(pairs.len(), 2);
    assert_eq!(
        pairs[0],
        ("Ünïcödë::Mödülé".to_string(), "Néw::Nàmé".to_string())
    );
    assert_eq!(
        pairs[1],
        ("Ünïcödë'Mödülé".to_string(), "Néw'Nàmé".to_string())
    );
    Ok(())
}
