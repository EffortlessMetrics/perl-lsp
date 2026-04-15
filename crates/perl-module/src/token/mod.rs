//! Boundary-safe Perl module token replacement helpers.
//!
//! Handles canonical (`Foo::Bar`) and legacy (`Foo'Bar`) separator variants
//! and delegates standalone token scanning to the boundary module.

use crate::boundary::{contains_standalone_module_token, find_standalone_module_token_ranges};

/// Build canonical + legacy module rename pairs.
///
/// The returned vector always includes the canonical `::` pair. It also
/// includes the legacy `'` pair when it differs.
pub use crate::name::module_variant_pairs;

/// Returns `true` when `line` contains `module_name` as a standalone module
/// token, respecting module boundaries.
#[must_use]
pub fn contains_module_token(line: &str, module_name: &str) -> bool {
    contains_standalone_module_token(line, module_name)
}

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
