//! Perl module-name separator normalization and variant helpers.
//!
//! This crate has a single responsibility: normalize and project Perl module
//! names across canonical (`::`) and legacy (`'`) package separator forms.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::borrow::Cow;

/// Normalize legacy package separator `'` to canonical `::`.
///
/// # Examples
///
/// ```
/// use perl_module_name::normalize_package_separator;
///
/// assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
/// assert_eq!(normalize_package_separator("Foo::Bar"), "Foo::Bar");
/// ```
#[must_use]
pub fn normalize_package_separator(module_name: &str) -> Cow<'_, str> {
    if module_name.contains('\'') {
        Cow::Owned(module_name.replace('\'', "::"))
    } else {
        Cow::Borrowed(module_name)
    }
}

/// Project canonical package separator `::` to legacy `'`.
///
/// # Examples
///
/// ```
/// use perl_module_name::legacy_package_separator;
///
/// assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
/// assert_eq!(legacy_package_separator("Foo'Bar"), "Foo'Bar");
/// ```
#[must_use]
pub fn legacy_package_separator(module_name: &str) -> Cow<'_, str> {
    if module_name.contains("::") {
        Cow::Owned(module_name.replace("::", "'"))
    } else {
        Cow::Borrowed(module_name)
    }
}

/// Build canonical + legacy module rename pairs.
///
/// The returned vector always includes the canonical `::` pair. It also
/// includes the legacy `'` pair when it differs.
///
/// # Examples
///
/// ```
/// use perl_module_name::module_variant_pairs;
///
/// let variants = module_variant_pairs("Foo::Bar", "New::Path");
/// assert_eq!(
///     variants,
///     vec![
///         ("Foo::Bar".to_string(), "New::Path".to_string()),
///         ("Foo'Bar".to_string(), "New'Path".to_string()),
///     ]
/// );
/// ```
#[must_use]
pub fn module_variant_pairs(old_module: &str, new_module: &str) -> Vec<(String, String)> {
    let canonical_old = normalize_package_separator(old_module).into_owned();
    let canonical_new = normalize_package_separator(new_module).into_owned();

    let canonical = (canonical_old.clone(), canonical_new.clone());
    let legacy = (
        legacy_package_separator(&canonical_old).into_owned(),
        legacy_package_separator(&canonical_new).into_owned(),
    );

    if legacy == canonical { vec![canonical] } else { vec![canonical, legacy] }
}

#[cfg(test)]
mod tests {
    use super::{legacy_package_separator, module_variant_pairs, normalize_package_separator};

    #[test]
    fn normalizes_legacy_separator() {
        assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
        assert_eq!(normalize_package_separator("Foo::Bar"), "Foo::Bar");
    }

    #[test]
    fn projects_canonical_separator_to_legacy() {
        assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
        assert_eq!(legacy_package_separator("Foo'Bar"), "Foo'Bar");
    }

    #[test]
    fn builds_canonical_and_legacy_variant_pairs() {
        assert_eq!(
            module_variant_pairs("Foo::Bar", "New::Path"),
            vec![
                ("Foo::Bar".to_string(), "New::Path".to_string()),
                ("Foo'Bar".to_string(), "New'Path".to_string()),
            ]
        );
    }

    #[test]
    fn deduplicates_pair_when_no_separator_variants_exist() {
        assert_eq!(module_variant_pairs("strict", "warnings").len(), 1);
    }
}
