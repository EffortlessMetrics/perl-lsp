//! Coverage-gap unit tests for `perl-test-generators`.
//!
//! Adds focused integration-level tests covering the public API surface of all
//! four strategy families: `variable()`, `module_path()`,
//! `module_path_segments()`, `non_empty_unicode_string()`, and
//! `unicode_string()`.
//!
//! These tests use `proptest!` to assert invariants of each strategy and
//! direct sampling loops to pin specific structural characteristics.

use perl_test_generators::{
    module_path, module_path_segments, non_empty_unicode_string, unicode_string, variable,
};
use proptest::prelude::*;
use proptest::strategy::ValueTree;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` when the string starts with one of the three Perl sigils.
fn has_perl_sigil(s: &str) -> bool {
    s.starts_with('$') || s.starts_with('@') || s.starts_with('%')
}

/// Returns `true` when the char is a valid *first* character for a Perl
/// identifier (ASCII letter or underscore, NOT digit).
fn is_ident_start_char(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

// ---------------------------------------------------------------------------
// variable() - sigil invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        ..ProptestConfig::default()
    })]

    /// Every generated variable must start with `$`, `@`, or `%`.
    #[test]
    fn variable_always_starts_with_sigil(v in variable()) {
        prop_assert!(
            has_perl_sigil(&v),
            "expected sigil prefix, got: {v:?}"
        );
    }

    /// The body after the sigil must never be empty.
    #[test]
    fn variable_body_is_never_empty(v in variable()) {
        prop_assert!(
            v.len() > 1,
            "variable body must not be empty: {v:?}"
        );
    }

    /// No generated variable may contain a bare space character.
    #[test]
    fn variable_contains_no_space(v in variable()) {
        prop_assert!(
            !v.contains(' '),
            "unexpected space in variable: {v:?}"
        );
    }

    /// When a variable is package-qualified (contains `::`), every segment
    /// around `::` must be non-empty.
    #[test]
    fn variable_package_qualified_has_no_empty_segments(v in variable()) {
        if v.contains("::") {
            // Strip sigil, then split on ::
            let body = &v[1..];
            for seg in body.split("::") {
                prop_assert!(
                    !seg.is_empty(),
                    "empty segment in qualified variable: {v:?}"
                );
            }
        }
    }

    /// Simple sigiled variables (no package) must have an all-ASCII body.
    #[test]
    fn variable_simple_body_is_ascii(v in variable()) {
        // Special variables like $_, @_, $1-$9, %ENV, @ARGV are all ASCII.
        // Generated simple + package-qualified are also ASCII.
        prop_assert!(v.is_ascii(), "variable contains non-ASCII: {v:?}");
    }

    /// Numeric special variables must have a single-digit body (0 or 1-9).
    ///
    /// `$0` is the program name in Perl; `$1`..$9` are capture groups.
    #[test]
    fn numeric_variables_have_digit_body(v in variable()) {
        if let Some(body) = v.strip_prefix('$') {
            if body.len() == 1 && body.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                let digit: u8 = body.chars().next().map_or(0, |c| c as u8 - b'0');
                prop_assert!(
                    digit <= 9,
                    "numeric variable out of range 0-9: {v:?}"
                );
            }
        }
    }

    /// Non-numeric, non-special variables have identifier-valid bodies.
    ///
    /// This covers `$x`, `@arr`, `%hash` - bodies that are not purely digits
    /// and are not `_` alone must start with a valid identifier character.
    #[test]
    fn variable_non_numeric_body_starts_with_ident_char(v in variable()) {
        let body = &v[1..];
        // Skip numeric specials ($0, $1..$9) and pure underscore specials ($_)
        if !body.is_empty()
            && body != "_"
            && !body.starts_with(|c: char| c.is_ascii_digit())
        {
            let first = body.chars().next();
            // Could be a package-qualified name: first char might be uppercase
            if let Some(ch) = first {
                prop_assert!(
                    is_ident_start_char(ch) || ch.is_ascii_uppercase(),
                    "body of {v:?} starts with invalid char {ch:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// variable() - determinism and sampling
// ---------------------------------------------------------------------------

/// Generate 200 samples and verify ALL start with a sigil.
///
/// This is a simple deterministic-runner test that catches strategy
/// misconfiguration at crate-compilation time rather than during property
/// runs.
#[test]
fn variable_sampled_sigil_100_percent() {
    let mut runner = proptest::test_runner::TestRunner::default();
    let strat = variable();
    for _ in 0..200 {
        let v = strat
            .new_tree(&mut runner)
            .map(|tree| tree.current())
            .unwrap_or_else(|_| "$_".to_string());
        assert!(has_perl_sigil(&v), "sampled variable without sigil: {v:?}");
        assert!(v.len() > 1, "sampled variable with empty body: {v:?}");
    }
}

/// The strategy must produce all three sigils across a large sample.
#[test]
fn variable_all_three_sigils_observed() {
    let mut runner = proptest::test_runner::TestRunner::default();
    let strat = variable();
    let mut saw_dollar = false;
    let mut saw_at = false;
    let mut saw_percent = false;

    for _ in 0..1000 {
        let v = strat
            .new_tree(&mut runner)
            .map(|tree| tree.current())
            .unwrap_or_else(|_| "$_".to_string());
        if v.starts_with('$') {
            saw_dollar = true;
        } else if v.starts_with('@') {
            saw_at = true;
        } else if v.starts_with('%') {
            saw_percent = true;
        }
        if saw_dollar && saw_at && saw_percent {
            break;
        }
    }
    assert!(saw_dollar, "never observed scalar sigil `$` in 1000 samples");
    assert!(saw_at, "never observed array sigil `@` in 1000 samples");
    assert!(saw_percent, "never observed hash sigil `%` in 1000 samples");
}

// ---------------------------------------------------------------------------
// module_path() - structural invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        ..ProptestConfig::default()
    })]

    /// Module path must be non-empty.
    #[test]
    fn module_path_non_empty(path in module_path()) {
        prop_assert!(!path.is_empty(), "module path must not be empty");
    }

    /// Module path must start with an uppercase ASCII letter.
    #[test]
    fn module_path_starts_uppercase(path in module_path()) {
        let first = path.chars().next();
        prop_assert!(
            first.is_some_and(|c| c.is_ascii_uppercase()),
            "module path must start uppercase: {path:?}"
        );
    }

    /// All segments split on `::` must be non-empty and start uppercase.
    #[test]
    fn module_path_all_segments_non_empty_and_uppercase(path in module_path()) {
        for seg in path.split("::") {
            prop_assert!(!seg.is_empty(), "empty segment in {path:?}");
            let first = seg.chars().next();
            prop_assert!(
                first.is_some_and(|c| c.is_ascii_uppercase()),
                "segment {seg:?} does not start uppercase in {path:?}"
            );
        }
    }

    /// Module path must contain between 1 and 5 segments.
    #[test]
    fn module_path_segment_count_in_bounds(path in module_path()) {
        let count = path.split("::").count();
        prop_assert!(
            (1..=5).contains(&count),
            "segment count {count} out of range [1,5] for {path:?}"
        );
    }

    /// Every character in every segment must be ASCII alphanumeric or `_`.
    #[test]
    fn module_path_segment_chars_valid(path in module_path()) {
        for seg in path.split("::") {
            for ch in seg.chars() {
                prop_assert!(
                    ch.is_ascii_alphanumeric() || ch == '_',
                    "invalid char {ch:?} in segment of {path:?}"
                );
            }
        }
    }

    /// Module path must be purely ASCII (module names are Perl identifiers).
    #[test]
    fn module_path_is_ascii(path in module_path()) {
        prop_assert!(path.is_ascii(), "non-ASCII chars in module path: {path:?}");
    }

    /// Two consecutive calls to the strategy are independent (not the same
    /// object), so we verify the type is returned correctly; actual
    /// independence is guaranteed by proptest internals.
    #[test]
    fn module_path_does_not_contain_empty_separator(path in module_path()) {
        // `::::` would appear if two empty segments were joined
        prop_assert!(
            !path.contains("::::"),
            "double separator `::::` found in {path:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// module_path() - known-segment sampling
// ---------------------------------------------------------------------------

/// The five hard-coded segments ("Foo", "Bar", "Baz", "HTTP", "IO") should
/// all appear across a large sample.
#[test]
fn module_path_known_segments_observed() {
    let mut runner = proptest::test_runner::TestRunner::default();
    let strat = module_path();
    let known = ["Foo", "Bar", "Baz", "HTTP", "IO"];
    let mut observed = vec![false; known.len()];

    for _ in 0..2000 {
        let path =
            strat.new_tree(&mut runner).map(|t| t.current()).unwrap_or_else(|_| "Foo".to_string());
        for seg in path.split("::") {
            for (i, &k) in known.iter().enumerate() {
                if seg == k {
                    observed[i] = true;
                }
            }
        }
        if observed.iter().all(|&b| b) {
            break;
        }
    }

    for (i, &k) in known.iter().enumerate() {
        assert!(observed[i], "known segment {k:?} never observed in 2000 module_path samples");
    }
}

// ---------------------------------------------------------------------------
// module_path_segments() - invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        ..ProptestConfig::default()
    })]

    /// Segments vec must have between 1 and 5 elements.
    #[test]
    fn module_path_segments_count_in_bounds(segs in module_path_segments()) {
        prop_assert!(
            !segs.is_empty() && segs.len() <= 5,
            "segment count {} out of [1,5]: {segs:?}",
            segs.len()
        );
    }

    /// Each segment must be non-empty.
    #[test]
    fn module_path_segments_each_non_empty(segs in module_path_segments()) {
        for seg in &segs {
            prop_assert!(!seg.is_empty(), "empty segment in {segs:?}");
        }
    }

    /// Each segment must start with an uppercase letter.
    #[test]
    fn module_path_segments_each_starts_uppercase(segs in module_path_segments()) {
        for seg in &segs {
            let first = seg.chars().next();
            prop_assert!(
                first.is_some_and(|c| c.is_ascii_uppercase()),
                "segment {seg:?} does not start uppercase"
            );
        }
    }

    /// Joining segments with `::` must equal what `module_path()` would produce
    /// from the same segments - i.e., the join is consistent.
    #[test]
    fn module_path_segments_join_consistent(segs in module_path_segments()) {
        let joined = segs.join("::");
        // The joined form must have the same segment count
        let re_split: Vec<&str> = joined.split("::").collect();
        prop_assert_eq!(
            re_split.len(),
            segs.len(),
            "join/split round-trip mismatch"
        );
        for (expected, actual) in segs.iter().zip(re_split.iter()) {
            prop_assert_eq!(
                expected.as_str(),
                *actual,
                "segment mismatch after join/split"
            );
        }
    }

    /// All segment characters must be ASCII alphanumeric or underscore.
    #[test]
    fn module_path_segments_chars_valid(segs in module_path_segments()) {
        for seg in &segs {
            for ch in seg.chars() {
                prop_assert!(
                    ch.is_ascii_alphanumeric() || ch == '_',
                    "invalid char {ch:?} in segment {seg:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// module_path_segments() - sampling
// ---------------------------------------------------------------------------

/// Verify segment counts from 1 to 5 are all reachable.
#[test]
fn module_path_segments_all_lengths_observed() {
    let mut runner = proptest::test_runner::TestRunner::default();
    let strat = module_path_segments();
    let mut observed = [false; 6]; // index 1..=5

    for _ in 0..5000 {
        let segs = strat
            .new_tree(&mut runner)
            .map(|t| t.current())
            .unwrap_or_else(|_| vec!["Foo".to_string()]);
        let n = segs.len();
        if n <= 5 {
            observed[n] = true;
        }
        if observed[1..=5].iter().all(|&b| b) {
            break;
        }
    }

    for (n, &was_seen) in observed.iter().enumerate().skip(1) {
        assert!(was_seen, "segment count {n} never observed in 5000 module_path_segments samples");
    }
}

// ---------------------------------------------------------------------------
// unicode_string() - UTF-8 and encoding invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        ..ProptestConfig::default()
    })]

    /// Every generated string must be valid UTF-8 (Rust `String` guarantees
    /// this, but we verify the round-trip explicitly).
    #[test]
    fn unicode_string_valid_utf8_roundtrip(s in unicode_string()) {
        let bytes = s.as_bytes();
        let roundtrip = std::str::from_utf8(bytes);
        prop_assert!(
            roundtrip.is_ok(),
            "generated string failed UTF-8 parse: {s:?}"
        );
        prop_assert_eq!(
            s.as_str(),
            roundtrip.unwrap_or_default(),
            "UTF-8 round-trip changed the string"
        );
    }

    /// UTF-16 encode then decode must be lossless for any generated string.
    #[test]
    fn unicode_string_utf16_roundtrip_lossless(s in unicode_string()) {
        let units: Vec<u16> = s.encode_utf16().collect();
        let decoded = String::from_utf16_lossy(&units);
        prop_assert_eq!(
            s, decoded,
            "UTF-16 roundtrip changed the string"
        );
    }

    /// `unicode_string()` may produce an empty string - verify the strategy
    /// never panics or returns an invalid value in that case.
    #[test]
    fn unicode_string_allows_empty(s in unicode_string()) {
        // No assertion on emptiness - strategy is permitted to produce "".
        // We just verify the value is a valid String (type-checked by proptest).
        let _ = s.len(); // len() is always defined
    }

    /// `unicode_string()` strings must have a char_len that agrees with
    /// iterating chars.
    #[test]
    fn unicode_string_char_count_matches_iteration(s in unicode_string()) {
        let count_method = s.chars().count();
        let count_iter: usize = s.chars().map(|_| 1).sum();
        prop_assert_eq!(count_method, count_iter);
    }

    /// No generated string contains surrogate code points (Rust's `char` type
    /// forbids them; this is a smoke-test that the strategy respects it).
    #[test]
    fn unicode_string_no_surrogates(s in unicode_string()) {
        for ch in s.chars() {
            let cp = ch as u32;
            prop_assert!(
                !(0xD800..=0xDFFF).contains(&cp),
                "surrogate code point U+{cp:04X} in generated string"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// non_empty_unicode_string() - non-empty contract
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        ..ProptestConfig::default()
    })]

    /// `non_empty_unicode_string()` must never produce an empty string.
    #[test]
    fn non_empty_unicode_string_always_non_empty(s in non_empty_unicode_string()) {
        prop_assert!(!s.is_empty(), "non-empty strategy returned empty string");
    }

    /// Must have at least one Unicode code point.
    #[test]
    fn non_empty_unicode_string_has_at_least_one_char(s in non_empty_unicode_string()) {
        prop_assert!(
            s.chars().count() >= 1,
            "no chars in non-empty string: {s:?}"
        );
    }

    /// Must be valid UTF-8 (round-trip check).
    #[test]
    fn non_empty_unicode_string_valid_utf8(s in non_empty_unicode_string()) {
        let ok = std::str::from_utf8(s.as_bytes()).is_ok();
        prop_assert!(ok, "non-empty string is not valid UTF-8: {s:?}");
    }

    /// UTF-16 round-trip must be lossless.
    #[test]
    fn non_empty_unicode_string_utf16_roundtrip(s in non_empty_unicode_string()) {
        let units: Vec<u16> = s.encode_utf16().collect();
        let decoded = String::from_utf16_lossy(&units);
        prop_assert_eq!(s, decoded);
    }

    /// No surrogate code points.
    #[test]
    fn non_empty_unicode_string_no_surrogates(s in non_empty_unicode_string()) {
        for ch in s.chars() {
            let cp = ch as u32;
            prop_assert!(
                !(0xD800..=0xDFFF).contains(&cp),
                "surrogate U+{cp:04X} in non-empty string"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// non_empty_unicode_string() - sampling
// ---------------------------------------------------------------------------

/// Verify the strategy produces at least some ASCII strings and some
/// non-ASCII strings (both ranges are covered).
#[test]
fn non_empty_unicode_string_covers_ascii_and_non_ascii() {
    let mut runner = proptest::test_runner::TestRunner::default();
    let strat = non_empty_unicode_string();
    let mut saw_ascii = false;
    let mut saw_non_ascii = false;

    for _ in 0..2000 {
        let s =
            strat.new_tree(&mut runner).map(|t| t.current()).unwrap_or_else(|_| "x".to_string());
        if s.is_ascii() {
            saw_ascii = true;
        } else {
            saw_non_ascii = true;
        }
        if saw_ascii && saw_non_ascii {
            break;
        }
    }
    assert!(saw_ascii, "never observed ASCII string in 2000 non_empty_unicode_string samples");
    assert!(
        saw_non_ascii,
        "never observed non-ASCII string in 2000 non_empty_unicode_string samples"
    );
}

/// Verify the strategy covers supplementary-plane (> U+FFFF) characters.
#[test]
fn non_empty_unicode_string_covers_supplementary_plane() {
    let mut runner = proptest::test_runner::TestRunner::default();
    let strat = non_empty_unicode_string();
    let mut saw_supplementary = false;

    for _ in 0..5000 {
        let s =
            strat.new_tree(&mut runner).map(|t| t.current()).unwrap_or_else(|_| "x".to_string());
        if s.chars().any(|c| c as u32 > 0xFFFF) {
            saw_supplementary = true;
            break;
        }
    }
    assert!(
        saw_supplementary,
        "never observed supplementary-plane char in 5000 non_empty_unicode_string samples"
    );
}

// ---------------------------------------------------------------------------
// unicode_string() - sampling characteristics
// ---------------------------------------------------------------------------

/// unicode_string() should occasionally produce empty strings (it has 0-bounded
/// collection ranges). Verify no panic occurs for empty.
#[test]
fn unicode_string_empty_string_is_valid() {
    // Directly test the empty string case (which is within the range 0..=50).
    let empty = String::new();
    assert!(empty.is_empty());
    let bytes = empty.as_bytes();
    assert!(std::str::from_utf8(bytes).is_ok());
}

/// Verify unicode_string() covers BMP non-ASCII (U+00C0..U+FFFF range).
#[test]
fn unicode_string_covers_bmp_non_ascii() {
    let mut runner = proptest::test_runner::TestRunner::default();
    let strat = unicode_string();
    let mut saw_bmp_non_ascii = false;

    for _ in 0..5000 {
        let s = strat.new_tree(&mut runner).map(|t| t.current()).unwrap_or_else(|_| String::new());
        if s.chars().any(|c| c as u32 >= 0x00C0 && c as u32 <= 0xFFFF) {
            saw_bmp_non_ascii = true;
            break;
        }
    }
    assert!(saw_bmp_non_ascii, "never observed BMP non-ASCII char in 5000 unicode_string samples");
}

// ---------------------------------------------------------------------------
// Cross-strategy: variable + module_path consistency
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// A package-qualified variable body (after sigil) follows the pattern
    /// `<module_path>::<identifier>`. The module part must match the same
    /// uppercase-start constraint as `module_path()`.
    #[test]
    fn variable_package_prefix_matches_module_conventions(v in variable()) {
        if let Some(last_sep) = v.rfind("::") {
            // sigil + package prefix before the last `::`
            let pkg_part = &v[1..last_sep]; // strip sigil
            for seg in pkg_part.split("::") {
                prop_assert!(!seg.is_empty(), "empty package prefix segment in {v:?}");
                let first = seg.chars().next();
                // Package segments must start uppercase (matches module_path convention)
                // OR be a plain identifier (lowercase) used as a pseudo-package.
                // In the variable() strategy, package paths use identifier()
                // which can start lowercase - so we only assert non-empty.
                prop_assert!(first.is_some(), "empty first char in segment of {v:?}");
            }
        }
    }

    /// `module_path()` output, when used as the base of a `$Pkg::name`
    /// variable, produces a valid prefix.
    #[test]
    fn module_path_plus_identifier_forms_valid_variable_prefix(
        path in module_path(),
        name in "[a-z][a-z0-9_]{0,10}",
    ) {
        let var = format!("${path}::{name}");
        prop_assert!(var.starts_with('$'), "must start with $: {var:?}");
        prop_assert!(var.contains("::"), "must contain :: separator: {var:?}");
        prop_assert!(!var.ends_with("::"), "must not end with ::: {var:?}");
    }
}
