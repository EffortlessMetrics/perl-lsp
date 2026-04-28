//! Query matching and ranking helpers for workspace symbol search.
//!
//! This crate has a single responsibility: provide reusable matching and
//! ranking primitives used by LSP symbol-search providers.

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
/// Ordering (highest to lowest relevance):
/// 1. Exact match (case-insensitive)
/// 2. Prefix match
/// 3. Contains (substring) match
/// 4. Fuzzy/subsequence match
///
/// Within the same tier, shorter names rank higher (closer to the query
/// length), with lexicographic order as the final tiebreaker.
#[must_use]
pub fn compare_names_by_query(a: &str, b: &str, query: &str) -> Ordering {
    let query_lower = query.to_lowercase();
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let a_tier = match_tier(&a_lower, &query_lower);
    let b_tier = match_tier(&b_lower, &query_lower);

    // Lower tier number = better match
    match a_tier.cmp(&b_tier) {
        Ordering::Equal => {
            // Within the same tier, prefer shorter names (closer to the query)
            match a.len().cmp(&b.len()) {
                Ordering::Equal => a.cmp(b),
                len_ord => len_ord,
            }
        }
        tier_ord => tier_ord,
    }
}

/// Assigns a numeric tier to a symbol name based on how well it matches the query.
///
/// Lower tier = better match:
/// - 0: exact match
/// - 1: prefix match
/// - 2: contains (substring) match
/// - 3: fuzzy/subsequence or no match (fallback)
fn match_tier(name_lower: &str, query_lower: &str) -> u8 {
    if name_lower == query_lower {
        0
    } else if name_lower.starts_with(query_lower) {
        1
    } else if name_lower.contains(query_lower) {
        2
    } else {
        3
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

    #[test]
    fn contains_matches_rank_above_fuzzy_matches() {
        // "get_bar" contains "bar" (tier 2)
        // "baz_art" has "bar" as subsequence b-a-z-a-r-t: b..a..r (tier 3)
        let mut names = ["baz_art", "get_bar"];
        names.sort_by(|a, b| compare_names_by_query(a, b, "bar"));

        assert_eq!(names[0], "get_bar", "substring match should rank above fuzzy");
    }

    #[test]
    fn exact_match_beats_everything() {
        let mut names = ["get_log", "getLogger", "log", "logging"];
        names.sort_by(|a, b| compare_names_by_query(a, b, "log"));

        assert_eq!(names[0], "log", "exact match should be first");
    }

    #[test]
    fn shorter_names_preferred_within_same_tier() {
        // Both are prefix matches (tier 1), shorter should come first
        let mut names = ["foobarqux", "foobar"];
        names.sort_by(|a, b| compare_names_by_query(a, b, "foo"));

        assert_eq!(names[0], "foobar");
        assert_eq!(names[1], "foobarqux");
    }

    #[test]
    fn four_tier_ranking_order() {
        // exact=0, prefix=1, contains=2, fuzzy=3
        // "lxoxg" is a fuzzy match for "log" (l..o..g subsequence)
        let mut names = ["get_log", "lxoxg", "log", "logger"];
        names.sort_by(|a, b| compare_names_by_query(a, b, "log"));

        assert_eq!(names[0], "log", "tier 0: exact");
        assert_eq!(names[1], "logger", "tier 1: prefix");
        assert_eq!(names[2], "get_log", "tier 2: contains");
        assert_eq!(names[3], "lxoxg", "tier 3: fuzzy");
    }
}
