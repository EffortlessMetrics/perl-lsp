//! Query matching and ranking helpers for workspace symbol search.
//!
//! This crate has a single responsibility: provide reusable matching and
//! ranking primitives used by LSP symbol-search providers.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::cmp::Ordering;

/// Returns `true` when a symbol name matches the provided query.
///
/// Matching strategy order:
/// 1. Empty query (matches everything)
/// 2. Exact case-insensitive match
/// 3. Prefix case-insensitive match
/// 4. Contains case-insensitive match
/// 5. Subsequence/fuzzy case-insensitive match
#[must_use]
pub fn matches_query(name: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();

    if name_lower == query_lower {
        return true;
    }

    if name_lower.starts_with(&query_lower) {
        return true;
    }

    if name_lower.contains(&query_lower) {
        return true;
    }

    is_subsequence(&name_lower, &query_lower)
}

/// Compares two symbol names by query relevance.
///
/// Ordering:
/// 1. Exact match first
/// 2. Prefix match second
/// 3. Lexicographic name order
#[must_use]
pub fn compare_names_by_query(a: &str, b: &str, query: &str) -> Ordering {
    let query_lower = query.to_lowercase();
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let a_exact = a_lower == query_lower;
    let b_exact = b_lower == query_lower;
    let a_prefix = a_lower.starts_with(&query_lower);
    let b_prefix = b_lower.starts_with(&query_lower);

    match (a_exact, b_exact) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => match (a_prefix, b_prefix) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.cmp(b),
        },
    }
}

fn is_subsequence(haystack: &str, needle: &str) -> bool {
    let mut needle_chars = needle.chars();
    let mut current = needle_chars.next();

    for ch in haystack.chars() {
        if let Some(target) = current {
            if ch == target {
                current = needle_chars.next();
            }
        } else {
            return true;
        }
    }

    current.is_none()
}

#[cfg(test)]
mod tests {
    use super::{compare_names_by_query, matches_query};

    #[test]
    fn query_matching_covers_exact_prefix_contains_and_fuzzy() {
        assert!(matches_query("foo", "foo"));
        assert!(matches_query("foobar", "foo"));
        assert!(matches_query("foobar", "bar"));
        assert!(matches_query("foobar", "fb"));
        assert!(!matches_query("alpha", "zq"));
    }

    #[test]
    fn empty_query_matches_anything() {
        assert!(matches_query("anything", ""));
    }

    #[test]
    fn relevance_prefers_exact_then_prefix_then_name_order() {
        let mut names = ["foxtrot", "foo", "foobar", "alpha"];
        names.sort_by(|a, b| compare_names_by_query(a, b, "foo"));

        assert_eq!(names, ["foo", "foobar", "alpha", "foxtrot"]);
    }
}
