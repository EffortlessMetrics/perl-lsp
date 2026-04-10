use perl_percentile::nearest_rank_percentile;

#[test]
fn given_an_empty_sample_when_any_percentile_is_requested_then_zero_is_returned() {
    let empty: [u64; 0] = [];

    let p50 = nearest_rank_percentile(&empty, 50);
    let p95 = nearest_rank_percentile(&empty, 95);
    let p999 = nearest_rank_percentile(&empty, 999);

    assert_eq!(p50, 0);
    assert_eq!(p95, 0);
    assert_eq!(p999, 0);
}

#[test]
fn given_a_single_value_when_any_percentile_up_to_or_above_100_is_requested_then_that_value_is_returned()
 {
    let sorted = [42];

    assert_eq!(nearest_rank_percentile(&sorted, 0), 42);
    assert_eq!(nearest_rank_percentile(&sorted, 50), 42);
    assert_eq!(nearest_rank_percentile(&sorted, 100), 42);
    assert_eq!(nearest_rank_percentile(&sorted, 10_000), 42);
}

#[test]
fn given_evenly_spaced_sorted_values_when_common_percentiles_are_requested_then_nearest_rank_is_used()
 {
    let sorted = [10, 20, 30, 40, 50];

    // rank = ceil((pct / 100) * n)
    assert_eq!(nearest_rank_percentile(&sorted, 20), 10); // ceil(1.0) => index 0
    assert_eq!(nearest_rank_percentile(&sorted, 40), 20); // ceil(2.0) => index 1
    assert_eq!(nearest_rank_percentile(&sorted, 41), 30); // ceil(2.05) => index 2
    assert_eq!(nearest_rank_percentile(&sorted, 80), 40); // ceil(4.0) => index 3
    assert_eq!(nearest_rank_percentile(&sorted, 100), 50); // ceil(5.0) => index 4
}

#[test]
fn given_repeated_values_when_percentiles_cross_duplicate_regions_then_the_returned_value_can_repeat()
 {
    let sorted = [1, 1, 1, 5, 5, 9];

    assert_eq!(nearest_rank_percentile(&sorted, 50), 1);
    assert_eq!(nearest_rank_percentile(&sorted, 67), 5);
    assert_eq!(nearest_rank_percentile(&sorted, 84), 9);
}

#[test]
fn given_percentiles_above_100_when_requested_then_they_are_clamped_to_100() {
    let sorted = [3, 6, 9, 12];

    let p100 = nearest_rank_percentile(&sorted, 100);
    let p101 = nearest_rank_percentile(&sorted, 101);
    let p500 = nearest_rank_percentile(&sorted, 500);

    assert_eq!(p100, 12);
    assert_eq!(p101, p100);
    assert_eq!(p500, p100);
}
