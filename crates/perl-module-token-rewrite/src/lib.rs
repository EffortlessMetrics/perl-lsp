//! Boundary-safe Perl module token replacement helper.
//!
//! This crate has a single responsibility: rewrite standalone occurrences of a
//! module token on one source line.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_boundary::find_standalone_module_token_ranges;

/// Replace standalone `from` module token occurrences in `line` with `to`.
///
/// Returns `(rewritten_line, changed)`.
#[must_use]
pub fn replace_module_token(line: &str, from: &str, to: &str) -> (String, bool) {
    if from.is_empty() || line.is_empty() {
        return (line.to_string(), false);
    }

    let mut ranges = find_standalone_module_token_ranges(line, from).peekable();
    if ranges.peek().is_none() {
        return (line.to_string(), false);
    }

    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;

    for range in ranges {
        out.push_str(&line[cursor..range.start]);
        out.push_str(to);
        cursor = range.end;
    }

    out.push_str(&line[cursor..]);
    (out, true)
}

#[cfg(test)]
mod tests {
    use super::replace_module_token;

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
}
