//! Perl module-name separator normalization and variant helpers.
//!
//! Normalize and project Perl module names across canonical (`::`) and
//! legacy (`'`) package separator forms.

use std::borrow::Cow;

/// Normalize legacy package separator `'` to canonical `::`.
#[must_use]
pub fn normalize_package_separator(module_name: &str) -> Cow<'_, str> {
    if module_name.contains('\'') {
        Cow::Owned(module_name.replace('\'', "::"))
    } else {
        Cow::Borrowed(module_name)
    }
}

/// Project canonical package separator `::` to legacy `'`.
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
