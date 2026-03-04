//! Comprehensive unit tests for `perl-module-name` public API.

use std::borrow::Cow;

use perl_module_name::{
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
    assert_eq!(normalize_package_separator("Ünîcödé'Módule"), "Ünîcödé::Módule");
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
    assert_eq!(legacy_package_separator("Ünîcödé::Módule"), "Ünîcödé'Módule");
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
    assert_eq!(pairs[0], ("A::B::C::D".to_string(), "W::X::Y::Z".to_string()));
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
    assert_eq!(normalize_package_separator("Perl5'Module'V2"), "Perl5::Module::V2");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Real-world Perl module names
// ──────────────────────────────────────────────────────────────

#[test]
fn normalize_cpan_style_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("File'Spec'Functions"), "File::Spec::Functions");
    Ok(())
}

#[test]
fn legacy_cpan_style_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(legacy_package_separator("File::Spec::Functions"), "File'Spec'Functions");
    Ok(())
}

#[test]
fn variant_pairs_cpan_rename() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("CGI::Cookie", "HTTP::Cookie");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("CGI::Cookie".to_string(), "HTTP::Cookie".to_string()));
    assert_eq!(pairs[1], ("CGI'Cookie".to_string(), "HTTP'Cookie".to_string()));
    Ok(())
}

#[test]
fn variant_pairs_moose_like_module() -> Result<(), Box<dyn std::error::Error>> {
    let pairs = module_variant_pairs("Moose::Role", "Moo::Role");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], ("Moose::Role".to_string(), "Moo::Role".to_string()));
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
