//! Property-based tests for `ByteSpan` algebra and slicing invariants.

use perl_position_tracking::ByteSpan;
use proptest::prelude::*;

fn span_pair_strategy() -> impl Strategy<Value = (ByteSpan, ByteSpan)> {
    (0usize..256, 0usize..256, 0usize..256, 0usize..256).prop_map(|(a0, a1, b0, b1)| {
        let (a_start, a_end) = if a0 <= a1 { (a0, a1) } else { (a1, a0) };
        let (b_start, b_end) = if b0 <= b1 { (b0, b1) } else { (b1, b0) };
        (ByteSpan::new(a_start, a_end), ByteSpan::new(b_start, b_end))
    })
}

proptest! {
    #[test]
    fn intersection_is_commutative_and_bounded((a, b) in span_pair_strategy()) {
        let ab = a.intersection(b);
        let ba = b.intersection(a);

        prop_assert_eq!(ab, ba);

        if let Some(shared) = ab {
            prop_assert!(a.contains_span(shared));
            prop_assert!(b.contains_span(shared));
            prop_assert!(shared.len() <= a.len());
            prop_assert!(shared.len() <= b.len());
        }
    }

    #[test]
    fn union_covers_both_spans((a, b) in span_pair_strategy()) {
        let union = a.union(b);

        prop_assert!(union.contains_span(a));
        prop_assert!(union.contains_span(b));
        prop_assert!(union.len() >= a.len());
        prop_assert!(union.len() >= b.len());
        prop_assert_eq!(union.start, a.start.min(b.start));
        prop_assert_eq!(union.end, a.end.max(b.end));
    }

    #[test]
    fn try_slice_matches_slice_on_valid_ascii_boundaries(source in "[ -~]{0,128}", start in 0usize..129, end in 0usize..129) {
        let len = source.len();
        let start = start.min(len);
        let end = end.min(len);
        let span = if start <= end {
            ByteSpan::new(start, end)
        } else {
            ByteSpan::new(end, start)
        };

        let from_try_slice = span.try_slice(&source);
        let from_slice = span.slice(&source);

        prop_assert_eq!(from_try_slice, Some(from_slice));
        prop_assert_eq!(from_slice.len(), span.len());
    }
}
