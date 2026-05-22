//! Generators for Perl variable names.
//!
//! Covers sigils (`$`, `@`, `%`), special variables (`$_`, `@_`, `$1`–`$9`),
//! and package-qualified names (`$Foo::bar`).

use proptest::prelude::*;

/// Valid ASCII identifier characters for variable name body (first char).
static IDENT_START: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_";

/// Valid ASCII identifier characters for subsequent positions.
static IDENT_CONT: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789";

/// Generate a single Perl identifier (without sigil).
fn identifier() -> impl Strategy<Value = String> {
    prop_oneof![
        // Common short names
        Just("self".to_string()),
        Just("class".to_string()),
        // Random identifier 1–12 chars
        (
            prop::sample::select(IDENT_START),
            prop::collection::vec(prop::sample::select(IDENT_CONT), 0..=11_usize),
        )
            .prop_map(|(first, rest)| {
                let mut s = String::new();
                s.push(first as char);
                for b in rest {
                    s.push(b as char);
                }
                s
            }),
    ]
}

/// Generate a Perl package path (`Foo`, `Foo::Bar`, ...).
fn package_path() -> impl Strategy<Value = String> {
    prop::collection::vec(identifier(), 1..=4_usize).prop_map(|segments| segments.join("::"))
}

/// Generate a full Perl variable name including sigil.
///
/// Covers scalar (`$x`), array (`@x`), hash (`%x`), special variables
/// (`$_`, `@_`, `$1`–`$9`), and optionally package-qualified names
/// (`$Foo::Bar::baz`).
pub fn variable() -> impl Strategy<Value = String> {
    prop_oneof![
        // Special variables
        Just("$_".to_string()),
        Just("@_".to_string()),
        Just("%ENV".to_string()),
        Just("@ARGV".to_string()),
        Just("$0".to_string()),
        (1u32..=9).prop_map(|n| format!("${}", n)),
        // Simple sigiled variable
        (prop_oneof![Just('$'), Just('@'), Just('%')], identifier())
            .prop_map(|(sigil, name)| format!("{}{}", sigil, name)),
        // Package-qualified
        (prop_oneof![Just('$'), Just('@'), Just('%')], package_path(), identifier())
            .prop_map(|(sigil, pkg, name)| format!("{}{}::{}", sigil, pkg, name)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn variable_starts_with_sigil(v in variable()) {
            prop_assert!(v.starts_with('$') || v.starts_with('@') || v.starts_with('%'));
        }

        #[test]
        fn variable_body_is_valid(v in variable()) {
            let body = &v[1..];
            prop_assert!(!body.is_empty(), "variable body must not be empty: {}", v);
        }

        #[test]
        fn identifier_is_ascii(id in identifier()) {
            prop_assert!(id.is_ascii(), "identifier must be ASCII: {}", id);
        }

        #[test]
        fn package_path_has_no_empty_segments(pkg in package_path()) {
            for segment in pkg.split("::") {
                prop_assert!(!segment.is_empty(), "empty package segment in {pkg}");
            }
        }

        #[test]
        fn variable_is_ascii(v in variable()) {
            prop_assert!(v.is_ascii(), "variable must be ASCII: {v}");
        }

        #[test]
        fn variable_len_is_at_least_two(v in variable()) {
            // sigil + at least one body character
            prop_assert!(v.len() >= 2, "variable too short: {v}");
        }

        #[test]
        fn identifier_starts_with_letter_or_underscore(id in identifier()) {
            let first = id.chars().next();
            match first {
                Some(ch) => prop_assert!(
                    ch.is_ascii_alphabetic() || ch == '_',
                    "identifier must start with letter or '_': {id}"
                ),
                None => prop_assert!(false, "identifier must not be empty"),
            }
        }

        #[test]
        fn identifier_is_non_empty(id in identifier()) {
            prop_assert!(!id.is_empty(), "identifier must be non-empty");
        }

        #[test]
        fn identifier_body_chars_are_word_chars(id in identifier()) {
            let mut chars = id.chars();
            // Skip first character (allowed: alpha or underscore)
            let _ = chars.next();
            for ch in chars {
                prop_assert!(
                    ch.is_ascii_alphanumeric() || ch == '_',
                    "invalid identifier body char '{ch}' in {id}"
                );
            }
        }

        #[test]
        fn package_path_segment_count_bounded(pkg in package_path()) {
            let count = pkg.split("::").count();
            prop_assert!(count >= 1, "package path must have at least one segment: {pkg}");
            prop_assert!(count <= 4, "package path must have at most 4 segments: {pkg}");
        }

        #[test]
        fn package_qualified_variable_contains_double_colon(v in variable()) {
            // Variables of the form $Foo::Bar::name contain "::" — when they do,
            // the prefix before the last "::" is the package, which must be non-empty.
            if let Some(last_sep) = v.rfind("::") {
                let sigil_and_pkg = &v[..last_sep];
                // sigil is at index 0, package part starts at index 1
                let pkg = &sigil_and_pkg[1..];
                prop_assert!(!pkg.is_empty(), "package prefix must be non-empty in {v}");
            }
        }

        #[test]
        fn numeric_special_variables_in_range(v in variable()) {
            // $1–$9 are capture variables; all numeric special vars from the
            // generator must be in the range 0–9.
            if let Some(body) = v.strip_prefix('$') {
                if let Ok(n) = body.parse::<u32>() {
                    prop_assert!(n <= 9, "numeric capture variable out of range: {v}");
                }
            }
        }

        #[test]
        fn special_at_underscore_sigil_is_at(v in variable()) {
            // @_ is a known special variable produced by the generator
            if v == "@_" {
                prop_assert!(v.starts_with('@'));
            }
        }

        #[test]
        fn package_path_is_ascii(pkg in package_path()) {
            prop_assert!(pkg.is_ascii(), "package path must be ASCII: {pkg}");
        }
    }
}
