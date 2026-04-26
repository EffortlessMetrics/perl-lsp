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
            assert!(v.starts_with('$') || v.starts_with('@') || v.starts_with('%'));
        }

        #[test]
        fn variable_body_is_valid(v in variable()) {
            let body = &v[1..];
            assert!(!body.is_empty(), "variable body must not be empty: {}", v);
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
    }
}
