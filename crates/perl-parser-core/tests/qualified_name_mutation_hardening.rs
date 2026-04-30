//! Mutation hardening tests for `qualified_name.rs`.
//!
//! Targets:
//!
//! * `split_qualified_name` — `rfind("::") + 2` offset (kills `+1` / `+3`
//!   arithmetic mutations).
//! * `validate_perl_qualified_name` — sigil set `['$', '@', '%', '&', '*']`
//!   (each sigil tested separately), empty-segment guard, empty-name guard.
//! * `is_valid_identifier_part` — `c.is_alphabetic() || c == '_'` start rule
//!   (kills `&&` mutation and drop of `|| c == '_'`), `c.is_alphanumeric() ||
//!   c == '_'` continuation rule.
//! * `container_name` — returns exact slice boundary.

use perl_parser_core::qualified_name::{
    container_name, is_valid_identifier_part, split_qualified_name, validate_perl_qualified_name,
};

// ---------------------------------------------------------------------------
// split_qualified_name — exact offset
//
// If `+2` is mutated to `+1`, the returned bare name starts with ":" instead
// of the identifier. If mutated to `+3`, the first character of the bare name
// is dropped.
// ---------------------------------------------------------------------------

#[test]
fn split_simple_package_returns_correct_bare_name() {
    let (pkg, bare) = split_qualified_name("Foo::Bar");
    assert_eq!(pkg, Some("Foo"), "package must be 'Foo'");
    // Bare name must NOT contain ':'.
    assert_eq!(bare, "Bar", "bare name must be 'Bar', not ':Bar' or 'ar'");
}

#[test]
fn split_deeply_qualified_name_uses_last_separator() {
    let (pkg, bare) = split_qualified_name("Foo::Bar::baz");
    assert_eq!(pkg, Some("Foo::Bar"));
    assert_eq!(bare, "baz", "bare name must be 'baz', not ':baz' or 'az'");
}

#[test]
fn split_unqualified_name_returns_none_package() {
    let (pkg, bare) = split_qualified_name("process");
    assert_eq!(pkg, None, "unqualified name must have no package");
    assert_eq!(bare, "process");
}

#[test]
fn split_name_with_trailing_separator_returns_empty_bare() {
    let (pkg, bare) = split_qualified_name("Package::");
    assert_eq!(pkg, Some("Package"));
    // The bare portion after "::" is empty — must not include the ':'s.
    assert_eq!(bare, "", "bare name after trailing '::' must be empty, not ':'");
}

#[test]
fn split_name_ending_in_double_colon_bare_has_no_colons() {
    let (_, bare) = split_qualified_name("A::B::C::");
    assert!(!bare.contains(':'), "bare name must not contain ':', got '{bare}'");
}

// ---------------------------------------------------------------------------
// container_name — uses split_qualified_name internally
// ---------------------------------------------------------------------------

#[test]
fn container_name_extracts_full_parent_path() {
    assert_eq!(container_name("Foo::Bar::baz"), Some("Foo::Bar"));
    assert_eq!(container_name("A::B"), Some("A"));
    assert_eq!(container_name("toplevel"), None);
    assert_eq!(container_name(""), None);
}

// ---------------------------------------------------------------------------
// validate_perl_qualified_name — each sigil individually
//
// Mutation: removing one character from the sigil set `['$','@','%','&','*']`
// would let that sigil pass validation.
// ---------------------------------------------------------------------------

#[test]
fn dollar_sigil_is_rejected() {
    let result = validate_perl_qualified_name("$foo");
    assert!(result.is_err(), "'$foo' must be rejected");
    assert!(
        matches!(
            result,
            Err(perl_parser_core::qualified_name::QualifiedNameError::LeadingSigil('$'))
        ),
        "error must be LeadingSigil('$'), got {result:?}"
    );
}

#[test]
fn at_sigil_is_rejected() {
    let result = validate_perl_qualified_name("@arr");
    assert!(result.is_err(), "'@arr' must be rejected");
    assert!(
        matches!(
            result,
            Err(perl_parser_core::qualified_name::QualifiedNameError::LeadingSigil('@'))
        ),
        "error must be LeadingSigil('@')"
    );
}

#[test]
fn percent_sigil_is_rejected() {
    let result = validate_perl_qualified_name("%hash");
    assert!(result.is_err(), "'%hash' must be rejected");
    assert!(
        matches!(
            result,
            Err(perl_parser_core::qualified_name::QualifiedNameError::LeadingSigil('%'))
        ),
        "error must be LeadingSigil('%')"
    );
}

#[test]
fn ampersand_sigil_is_rejected() {
    let result = validate_perl_qualified_name("&sub");
    assert!(result.is_err(), "'&sub' must be rejected");
    assert!(
        matches!(
            result,
            Err(perl_parser_core::qualified_name::QualifiedNameError::LeadingSigil('&'))
        ),
        "error must be LeadingSigil('&')"
    );
}

#[test]
fn asterisk_sigil_is_rejected() {
    let result = validate_perl_qualified_name("*glob");
    assert!(result.is_err(), "'*glob' must be rejected");
    assert!(
        matches!(
            result,
            Err(perl_parser_core::qualified_name::QualifiedNameError::LeadingSigil('*'))
        ),
        "error must be LeadingSigil('*')"
    );
}

// ---------------------------------------------------------------------------
// validate_perl_qualified_name — empty-name and empty-segment guards
//
// Mutation: flip `name.is_empty()` to `!name.is_empty()`, or flip
// `part.is_empty()` to `!part.is_empty()`.
// ---------------------------------------------------------------------------

#[test]
fn empty_name_returns_empty_name_error() {
    let result = validate_perl_qualified_name("");
    assert!(
        matches!(result, Err(perl_parser_core::qualified_name::QualifiedNameError::EmptyName)),
        "empty string must return EmptyName, got {result:?}"
    );
}

#[test]
fn valid_name_passes_validation() {
    assert!(validate_perl_qualified_name("Foo").is_ok());
    assert!(validate_perl_qualified_name("Foo::Bar").is_ok());
    assert!(validate_perl_qualified_name("_Private").is_ok());
}

#[test]
fn trailing_separator_returns_empty_segment_error() {
    let result = validate_perl_qualified_name("Foo::");
    assert!(
        matches!(
            result,
            Err(perl_parser_core::qualified_name::QualifiedNameError::EmptySegment { .. })
        ),
        "'Foo::' must return EmptySegment, got {result:?}"
    );
}

#[test]
fn leading_separator_returns_empty_segment_error() {
    let result = validate_perl_qualified_name("::Bar");
    assert!(
        matches!(
            result,
            Err(perl_parser_core::qualified_name::QualifiedNameError::EmptySegment { index: 0 })
        ),
        "'::Bar' must return EmptySegment at index 0, got {result:?}"
    );
}

#[test]
fn double_separator_returns_empty_segment_error() {
    let result = validate_perl_qualified_name("Foo::::Bar");
    assert!(result.is_err(), "'Foo::::Bar' must be rejected");
}

// ---------------------------------------------------------------------------
// is_valid_identifier_part — start-character rule
//
// Mutations:
//   `c.is_alphabetic() || c == '_'`  →  `c.is_alphabetic() && c == '_'`
//   `c.is_alphabetic() || c == '_'`  →  `c.is_alphabetic()` (drop underscore)
// ---------------------------------------------------------------------------

#[test]
fn underscore_start_is_valid_identifier() {
    // If `|| c == '_'` is removed, `_Private` would be rejected.
    assert!(is_valid_identifier_part("_Private"), "'_Private' must be a valid identifier part");
    assert!(is_valid_identifier_part("_"), "bare '_' must be valid");
}

#[test]
fn alphabetic_start_is_valid_identifier() {
    assert!(is_valid_identifier_part("Foo"));
    assert!(is_valid_identifier_part("a"));
    assert!(is_valid_identifier_part("MyPkg"));
}

#[test]
fn digit_start_is_not_valid_identifier() {
    // Neither `is_alphabetic()` nor `== '_'` matches a digit.
    assert!(!is_valid_identifier_part("1foo"), "'1foo' must be invalid");
    assert!(!is_valid_identifier_part("123"), "'123' must be invalid");
}

#[test]
fn empty_string_is_not_valid_identifier() {
    assert!(!is_valid_identifier_part(""), "empty string must be invalid");
}

// ---------------------------------------------------------------------------
// is_valid_identifier_part — continuation-character rule
//
// Mutation: `c.is_alphanumeric() || c == '_'`  →  `c.is_alphanumeric()`
// ---------------------------------------------------------------------------

#[test]
fn underscore_in_continuation_is_valid() {
    // If `|| c == '_'` in continuation is dropped, `foo_bar` would be invalid.
    assert!(is_valid_identifier_part("foo_bar"), "'foo_bar' must be valid");
    assert!(is_valid_identifier_part("My_Pkg_Name"), "'My_Pkg_Name' must be valid");
}

#[test]
fn digits_in_continuation_are_valid() {
    assert!(is_valid_identifier_part("Foo2"), "'Foo2' must be valid");
    assert!(is_valid_identifier_part("pkg42"), "'pkg42' must be valid");
}

#[test]
fn hyphen_in_continuation_is_not_valid() {
    // Hyphens are not alphanumeric and not `_`.
    assert!(!is_valid_identifier_part("foo-bar"), "'foo-bar' must be invalid");
}
