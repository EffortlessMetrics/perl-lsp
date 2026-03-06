use perl_percentile::nearest_rank_percentile;
use proptest::prelude::*;

proptest! {
    #[test]
    fn percentile_result_is_member_of_sorted_input(mut values in proptest::collection::vec(0_u64..10_000, 1..128), pct in 0_u64..1000) {
        values.sort_unstable();
        let result = nearest_rank_percentile(&values, pct);
        prop_assert!(values.binary_search(&result).is_ok());
    }
}
