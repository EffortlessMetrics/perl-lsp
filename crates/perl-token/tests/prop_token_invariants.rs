//! Property-based tests for `perl-token` invariants.
//!
//! Invariants tested:
//! - `TokenSpan`: `start <= end` ordering is enforced by `try_new`
//! - `TokenSpan::len` = `end - start` when ordered; 0 when empty
//! - `TokenSpan::is_empty` iff `len() == 0`
//! - `TokenSpan::contains` and `touches` are consistent with span bounds
//! - `TokenSpan::cover` result contains both input spans
//! - `TokenSpan::overlaps` is symmetric
//! - `Token` and `TokenRef` span ordering invariant: `try_new` rejects `end < start`
//! - Equality reflexivity for `TokenSpan`
//! - `TokenKind::from_keyword` / `from_operator` / `from_delimiter` / `from_sigil` round-trip:
//!   each spelled entry maps back to the correct kind

use perl_token::{
    DELIMITER_SPELLINGS, KEYWORD_SPELLINGS, OPERATOR_SPELLINGS, SIGIL_SPELLINGS, TokenKind,
    TokenRef, TokenSpan, TokenSpanError,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy for a valid (ordered) span: generates `(start, end)` with `start <= end`.
fn ordered_span() -> impl Strategy<Value = (usize, usize)> {
    (0usize..512, 0usize..512).prop_map(|(a, b)| if a <= b { (a, b) } else { (b, a) })
}

/// Strategy for a pair of ordered spans.
fn ordered_span_pair() -> impl Strategy<Value = ((usize, usize), (usize, usize))> {
    (ordered_span(), ordered_span())
}

/// Strategy for all token kinds (via `TokenKind::all()`).
fn any_token_kind() -> impl Strategy<Value = TokenKind> {
    let all = TokenKind::all();
    (0usize..all.len()).prop_map(move |i| all[i])
}

// ---------------------------------------------------------------------------
// TokenSpan ordering invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// `try_new` accepts ordered spans and preserves the endpoints.
    #[test]
    fn span_try_new_accepts_ordered_span((start, end) in ordered_span()) {
        let result = TokenSpan::try_new(start, end);
        prop_assert!(result.is_ok(), "try_new failed for ordered span ({}, {})", start, end);
        let span = match result {
            Ok(span) => span,
            Err(err) => {
                return Err(proptest::test_runner::TestCaseError::fail(format!(
                    "try_new failed for ordered span ({start}, {end}): {err}"
                )));
            }
        };
        prop_assert_eq!(span.start, start);
        prop_assert_eq!(span.end, end);
    }

    /// `try_new` rejects spans where `end < start`.
    #[test]
    fn span_try_new_rejects_inverted_span(start in 1usize..512, extra in 1usize..256) {
        let end = start.saturating_sub(extra.min(start));
        prop_assume!(end < start);
        let result = TokenSpan::try_new(start, end);
        prop_assert_eq!(result, Err(TokenSpanError::EndBeforeStart { start, end }));
    }

    /// `len()` equals `end - start` for ordered spans.
    #[test]
    fn span_len_equals_end_minus_start((start, end) in ordered_span()) {
        let span = TokenSpan::new(start, end);
        prop_assert_eq!(span.len(), end - start);
    }

    /// `is_empty()` is equivalent to `len() == 0`.
    #[test]
    // This property intentionally compares `is_empty` with the raw length predicate.
    #[allow(clippy::len_zero)]
    fn span_is_empty_iff_len_zero((start, end) in ordered_span()) {
        let span = TokenSpan::new(start, end);
        prop_assert_eq!(span.is_empty(), span.len() == 0);
    }

    /// `is_empty()` is true iff `start == end`.
    #[test]
    fn span_is_empty_iff_start_eq_end((start, end) in ordered_span()) {
        let span = TokenSpan::new(start, end);
        prop_assert_eq!(span.is_empty(), start == end);
    }

    /// Equality is reflexive.
    #[test]
    fn span_equality_is_reflexive((start, end) in ordered_span()) {
        let span = TokenSpan::new(start, end);
        prop_assert_eq!(span, span);
    }

    /// `range()` matches the `start..end` range.
    #[test]
    fn span_range_matches_start_end((start, end) in ordered_span()) {
        let span = TokenSpan::new(start, end);
        prop_assert_eq!(span.range(), start..end);
    }

    /// `contains(offset)` implies `offset >= start && offset < end`.
    #[test]
    fn span_contains_is_consistent_with_bounds(
        (start, end) in ordered_span(),
        offset in 0usize..600,
    ) {
        let span = TokenSpan::new(start, end);
        let contained = span.contains(offset);
        let expected = offset >= start && offset < end;
        prop_assert_eq!(contained, expected,
            "contains({}) = {} but expected {} for span [{}, {})", offset, contained, expected, start, end);
    }

    /// `touches(offset)` implies `offset >= start && offset <= end`.
    #[test]
    fn span_touches_is_consistent_with_bounds(
        (start, end) in ordered_span(),
        offset in 0usize..600,
    ) {
        let span = TokenSpan::new(start, end);
        let touched = span.touches(offset);
        let expected = offset >= start && offset <= end;
        prop_assert_eq!(touched, expected,
            "touches({}) = {} but expected {} for span [{}, {}]", offset, touched, expected, start, end);
    }

    /// `overlaps` is symmetric.
    #[test]
    fn span_overlaps_is_symmetric(((a0, a1), (b0, b1)) in ordered_span_pair()) {
        let a = TokenSpan::new(a0, a1);
        let b = TokenSpan::new(b0, b1);
        prop_assert_eq!(a.overlaps(b), b.overlaps(a),
            "overlaps not symmetric: a=[{},{}) b=[{},{})", a0, a1, b0, b1);
    }

    /// `cover` result contains both input spans.
    #[test]
    fn span_cover_contains_both(((a0, a1), (b0, b1)) in ordered_span_pair()) {
        let a = TokenSpan::new(a0, a1);
        let b = TokenSpan::new(b0, b1);
        let covered = a.cover(b);
        // cover(a, b).start <= min(a.start, b.start)
        prop_assert!(covered.start <= a0, "cover.start ({}) > a.start ({})", covered.start, a0);
        prop_assert!(covered.start <= b0, "cover.start ({}) > b.start ({})", covered.start, b0);
        // cover(a, b).end >= max(a.end, b.end)
        prop_assert!(covered.end >= a1, "cover.end ({}) < a.end ({})", covered.end, a1);
        prop_assert!(covered.end >= b1, "cover.end ({}) < b.end ({})", covered.end, b1);
    }

    /// `cover` is commutative.
    #[test]
    fn span_cover_is_commutative(((a0, a1), (b0, b1)) in ordered_span_pair()) {
        let a = TokenSpan::new(a0, a1);
        let b = TokenSpan::new(b0, b1);
        prop_assert_eq!(a.cover(b), b.cover(a));
    }
}

// ---------------------------------------------------------------------------
// TokenRef span invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// `TokenRef::try_new` accepts ordered spans.
    #[test]
    fn token_ref_try_new_accepts_ordered(
        (start, end) in ordered_span(),
        kind in any_token_kind(),
    ) {
        // try_new only checks ordering, not empty-span rules
        let result = TokenRef::try_new(kind, "x", start, end);
        prop_assert!(result.is_ok(),
            "try_new failed for ordered ({},{}) kind={:?}", start, end, kind);
        let r = match result {
            Ok(token_ref) => token_ref,
            Err(err) => {
                return Err(proptest::test_runner::TestCaseError::fail(format!(
                    "try_new failed for ordered ({start},{end}) kind={kind:?}: {err}"
                )));
            }
        };
        prop_assert_eq!(r.start, start);
        prop_assert_eq!(r.end, end);
    }

    /// `TokenRef::try_new` rejects inverted spans.
    #[test]
    fn token_ref_try_new_rejects_inverted(
        base in 1usize..256,
        extra in 1usize..128,
        kind in any_token_kind(),
    ) {
        // Construct start > end
        let start = base + extra;
        let end = base.saturating_sub(1);
        prop_assume!(start > end);
        let result = TokenRef::try_new(kind, "x", start, end);
        prop_assert_eq!(result, Err(TokenSpanError::EndBeforeStart { start, end }));
    }

    /// `len()` on `TokenRef` equals `end - start` for ordered spans.
    /// `TokenRef::new` does not validate spans, so we can pass arbitrary positions
    /// with "x" as dummy text.
    #[test]
    fn token_ref_len_is_end_minus_start(
        (start, end) in ordered_span(),
        kind in any_token_kind(),
    ) {
        // TokenRef::new is unchecked: accepts any (start, end) regardless of text length
        let r = TokenRef::new(kind, "x", start, end);
        prop_assert_eq!(r.len(), end - start);
    }
}

// ---------------------------------------------------------------------------
// Keyword / operator / delimiter / sigil spelling round-trips
// ---------------------------------------------------------------------------

#[test]
fn keyword_spellings_round_trip() {
    for (spelling, expected_kind) in KEYWORD_SPELLINGS {
        let got = TokenKind::from_keyword(spelling);
        assert_eq!(
            got,
            Some(*expected_kind),
            "from_keyword({spelling:?}) should be {expected_kind:?}"
        );
    }
}

#[test]
fn operator_spellings_round_trip() {
    for (spelling, expected_kind) in OPERATOR_SPELLINGS {
        let got = TokenKind::from_operator(spelling);
        assert_eq!(
            got,
            Some(*expected_kind),
            "from_operator({spelling:?}) should be {expected_kind:?}"
        );
    }
}

#[test]
fn delimiter_spellings_round_trip() {
    for (spelling, expected_kind) in DELIMITER_SPELLINGS {
        let got = TokenKind::from_delimiter(spelling);
        assert_eq!(
            got,
            Some(*expected_kind),
            "from_delimiter({spelling:?}) should be {expected_kind:?}"
        );
    }
}

#[test]
fn sigil_spellings_round_trip() {
    for (spelling, expected_kind) in SIGIL_SPELLINGS {
        let got = TokenKind::from_sigil(spelling);
        assert_eq!(
            got,
            Some(*expected_kind),
            "from_sigil({spelling:?}) should be {expected_kind:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// TokenKind category invariants (property over all kinds)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// Every kind has a non-empty `display_name`.
    #[test]
    fn every_kind_has_nonempty_display_name(kind in any_token_kind()) {
        prop_assert!(!kind.display_name().is_empty(),
            "display_name() is empty for {:?}", kind);
    }

    /// `metadata()` round-trips through `category()` and `display_name()`.
    #[test]
    fn metadata_round_trips_category_and_display(kind in any_token_kind()) {
        let meta = kind.metadata();
        prop_assert_eq!(meta.category, kind.category());
        prop_assert_eq!(meta.display_name, kind.display_name());
    }
}

// ---------------------------------------------------------------------------
// Regression / targeted cases
// ---------------------------------------------------------------------------

/// Empty span with `Eof` is allowed by `try_new` (ordering is satisfied).
#[test]
fn empty_span_for_eof_is_valid() {
    let result = TokenRef::try_new(TokenKind::Eof, "", 0, 0);
    assert!(result.is_ok());
}

/// Zero-length span with non-Eof kind is accepted by `try_new` but rejected by `new_checked`.
#[test]
fn empty_span_non_eof_rejected_by_new_checked() {
    let result = TokenRef::new_checked(TokenKind::Identifier, "", 5, 5);
    assert_eq!(
        result,
        Err(TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 5 })
    );
}

/// `TokenSpan::new(0,0)` is empty.
#[test]
fn span_zero_zero_is_empty() {
    let span = TokenSpan::new(0, 0);
    assert!(span.is_empty());
    assert_eq!(span.len(), 0);
}

/// `contains` is false for all offsets on an empty span.
#[test]
fn empty_span_contains_nothing() {
    let span = TokenSpan::new(5, 5);
    for offset in 0..10 {
        assert!(!span.contains(offset));
    }
}

/// `touches` is true only at the single boundary offset of an empty span.
#[test]
fn empty_span_touches_only_its_boundary() {
    let span = TokenSpan::new(5, 5);
    assert!(span.touches(5));
    assert!(!span.touches(4));
    assert!(!span.touches(6));
}
