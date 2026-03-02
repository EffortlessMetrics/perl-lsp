//! Boundary-safe Perl module token replacement helpers.
//!
//! This crate provides a small, focused API used by module-rename workflows.
//! It handles canonical (`Foo::Bar`) and legacy (`Foo'Bar`) separator variants
//! and delegates standalone token scanning to `perl-module-boundary`.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_boundary::contains_standalone_module_token;

/// Build canonical + legacy module rename pairs.
///
/// The returned vector always includes the canonical `::` pair. It also
/// includes the legacy `'` pair when it differs.
///
/// # Examples
///
/// ```
/// use perl_module_token::module_variant_pairs;
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
pub use perl_module_name::module_variant_pairs;

/// Returns `true` when `line` contains `module_name` as a standalone module
/// token, respecting module boundaries.
#[must_use]
pub fn contains_module_token(line: &str, module_name: &str) -> bool {
    contains_standalone_module_token(line, module_name)
}

/// Replace standalone `from` module token occurrences in `line` with `to`.
///
/// Returns `(rewritten_line, changed)`.
pub use perl_module_token_rewrite::replace_module_token;

#[cfg(test)]
mod tests {
    use super::{contains_module_token, module_variant_pairs, replace_module_token};

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
    fn canonicalizes_legacy_inputs_for_variant_pairs() {
        assert_eq!(
            module_variant_pairs("Foo'Bar", "New'Path"),
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

    #[test]
    fn replaces_only_standalone_module_tokens() {
        let (rewritten, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "X::Y");
        assert_eq!(rewritten, "use X::Y;");
        assert!(changed);

        let (rewritten, changed) = replace_module_token("use Foo::Barista;", "Foo::Bar", "X::Y");
        assert_eq!(rewritten, "use Foo::Barista;");
        assert!(!changed);
    }

    #[test]
    fn treats_legacy_separator_as_module_character_boundary() {
        let (rewritten, changed) = replace_module_token("use Foo'Bar'Baz;", "Foo'Bar", "X'Y");
        assert_eq!(rewritten, "use Foo'Bar'Baz;");
        assert!(!changed);
    }

    #[test]
    fn contains_matches_boundary_aware_token_presence() {
        assert!(contains_module_token("use parent 'Foo::Bar';", "Foo::Bar"));
        assert!(!contains_module_token("use Foo::Barista;", "Foo::Bar"));
    }
}
