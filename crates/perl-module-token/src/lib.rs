//! Boundary-safe Perl module token replacement helpers.
//!
//! This crate provides a small, focused API used by module-rename workflows.
//! It handles canonical (`Foo::Bar`) and legacy (`Foo'Bar`) separator variants
//! and performs boundary-aware token replacement on a single line.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

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
#[must_use]
pub fn module_variant_pairs(old_module: &str, new_module: &str) -> Vec<(String, String)> {
    let canonical_old = canonicalize_module_name(old_module);
    let canonical_new = canonicalize_module_name(new_module);

    let canonical = (canonical_old.clone(), canonical_new.clone());
    let legacy = (canonical_old.replace("::", "'"), canonical_new.replace("::", "'"));

    if legacy == canonical { vec![canonical] } else { vec![canonical, legacy] }
}

/// Returns `true` when `line` contains `module_name` as a standalone module
/// token, respecting module boundaries.
#[must_use]
pub fn contains_module_token(line: &str, module_name: &str) -> bool {
    replace_module_token(line, module_name, module_name).1
}

/// Replace standalone `from` module token occurrences in `line` with `to`.
///
/// Returns `(rewritten_line, changed)`.
#[must_use]
pub fn replace_module_token(line: &str, from: &str, to: &str) -> (String, bool) {
    if from.is_empty() || line.is_empty() {
        return (line.to_string(), false);
    }

    let mut out = String::with_capacity(line.len());
    let mut search_start = 0usize;
    let mut replaced = false;

    while let Some(rel_pos) = line[search_start..].find(from) {
        let start = search_start + rel_pos;
        let end = start + from.len();

        if has_module_boundaries(line, start, end) {
            out.push_str(&line[search_start..start]);
            out.push_str(to);
            replaced = true;
        } else {
            out.push_str(&line[search_start..end]);
        }

        search_start = end;
    }

    if replaced {
        out.push_str(&line[search_start..]);
        (out, true)
    } else {
        (line.to_string(), false)
    }
}

fn canonicalize_module_name(module_name: &str) -> String {
    module_name.replace('\'', "::")
}

fn has_module_boundaries(line: &str, start: usize, end: usize) -> bool {
    let left_ok = !left_context_is_module_char(line, start);
    let right_ok = !right_context_is_module_char(line, end);

    left_ok && right_ok
}

fn is_module_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn left_context_is_module_char(line: &str, start: usize) -> bool {
    if start == 0 {
        return false;
    }

    let mut left = line[..start].char_indices();
    let Some((left_idx, ch)) = left.next_back() else {
        return false;
    };

    if ch != '\'' {
        return is_module_char(ch);
    }

    if left_idx == 0 {
        return false;
    }

    line[..left_idx].chars().next_back().is_some_and(is_identifier_char)
}

fn right_context_is_module_char(line: &str, end: usize) -> bool {
    if end >= line.len() {
        return false;
    }

    let mut right = line[end..].chars();
    let Some(ch) = right.next() else {
        return false;
    };

    if ch != '\'' {
        return is_module_char(ch);
    }

    right.next().is_some_and(is_identifier_char)
}

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
