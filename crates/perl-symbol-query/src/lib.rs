//! Symbol query matching and relevance ordering.
//!
//! This crate has a single responsibility: evaluate whether symbol names match
//! a user query and rank those matches for stable workspace symbol results.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::cmp::Ordering;

/// Returns true when a symbol name matches a lowercased query.
///
/// Matching strategy order:
/// 1. Empty query (always match)
/// 2. Exact match (case-insensitive)
/// 3. Prefix match
/// 4. Contains match
/// 5. Fuzzy subsequence match
#[must_use]
pub fn matches_query_lowercase(name: &str, query_lower: &str) -> bool {
    if query_lower.is_empty() {
        return true;
    }

    let name_lower = name.to_lowercase();

    if name_lower == query_lower {
        return true;
    }

    if name_lower.starts_with(query_lower) {
        return true;
    }

    if name_lower.contains(query_lower) {
        return true;
    }

    let mut query_chars = query_lower.chars();
    let mut current_char = query_chars.next();

    for ch in name_lower.chars() {
        if let Some(qch) = current_char {
            if ch == qch {
                current_char = query_chars.next();
            }
        } else {
            return true;
        }
    }

    current_char.is_none()
}

/// Compares two symbol names by query relevance.
///
/// Relevance order:
/// 1. Exact matches first
/// 2. Prefix matches second
/// 3. Alphabetical order as stable tie-breaker
#[must_use]
pub fn compare_names_by_query(a: &str, b: &str, query_lower: &str) -> Ordering {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let a_exact = a_lower == query_lower;
    let b_exact = b_lower == query_lower;

    match (a_exact, b_exact) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => {
            let a_prefix = a_lower.starts_with(query_lower);
            let b_prefix = b_lower.starts_with(query_lower);

            match (a_prefix, b_prefix) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a.cmp(b),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{compare_names_by_query, matches_query_lowercase};
    use std::cmp::Ordering;

    #[test]
    fn match_strategies_work_in_order() {
        assert!(matches_query_lowercase("foobar", ""));
        assert!(matches_query_lowercase("Foo", "foo"));
        assert!(matches_query_lowercase("Foobar", "foo"));
        assert!(matches_query_lowercase("Foobar", "oba"));
        assert!(matches_query_lowercase("foobar", "fbr"));
        assert!(!matches_query_lowercase("foobar", "fzr"));
    }

    #[test]
    fn relevance_sort_prefers_exact_then_prefix() {
        assert_eq!(compare_names_by_query("foo", "foobar", "foo"), Ordering::Less);
        assert_eq!(compare_names_by_query("foobar", "barfoo", "foo"), Ordering::Less);
        assert_eq!(compare_names_by_query("alpha", "beta", "zzz"), Ordering::Less);
    }
}
