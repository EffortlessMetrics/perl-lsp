use perl_percentile::nearest_rank_percentile;

#[test]
fn scenario_empty_input_returns_zero_for_any_percentile() {
    // Given an empty sorted sample window.
    let sample = [];

    // When we ask for several percentile ranks.
    let p0 = nearest_rank_percentile(&sample, 0);
    let p50 = nearest_rank_percentile(&sample, 50);
    let p95 = nearest_rank_percentile(&sample, 95);
    let p100 = nearest_rank_percentile(&sample, 100);

    // Then percentile lookup is always zero.
    assert_eq!(p0, 0);
    assert_eq!(p50, 0);
    assert_eq!(p95, 0);
    assert_eq!(p100, 0);
}

#[test]
fn scenario_single_value_returns_same_value_for_every_percentile() {
    // Given a single-value sorted sample window.
    let sample = [42_u64];

    // When we ask for any percentile including overflow percentile values.
    let p0 = nearest_rank_percentile(&sample, 0);
    let p1 = nearest_rank_percentile(&sample, 1);
    let p50 = nearest_rank_percentile(&sample, 50);
    let p99 = nearest_rank_percentile(&sample, 99);
    let p100 = nearest_rank_percentile(&sample, 100);
    let p1000 = nearest_rank_percentile(&sample, 1_000);

    // Then nearest-rank always resolves to that only element.
    assert_eq!(p0, 42);
    assert_eq!(p1, 42);
    assert_eq!(p50, 42);
    assert_eq!(p99, 42);
    assert_eq!(p100, 42);
    assert_eq!(p1000, 42);
}

#[test]
fn scenario_common_percentiles_follow_nearest_rank_on_sorted_samples() {
    // Given a deterministic sorted sample window.
    let sample = [10_u64, 20, 30, 40, 50];

    // When we request representative percentile buckets.
    let p0 = nearest_rank_percentile(&sample, 0); // ceil(0 * 5) => rank 0 => index 0 via saturating_sub
    let p1 = nearest_rank_percentile(&sample, 1); // ceil(0.05) => rank 1 => index 0
    let p20 = nearest_rank_percentile(&sample, 20); // ceil(1.0) => rank 1 => index 0
    let p21 = nearest_rank_percentile(&sample, 21); // ceil(1.05) => rank 2 => index 1
    let p50 = nearest_rank_percentile(&sample, 50); // ceil(2.5) => rank 3 => index 2
    let p95 = nearest_rank_percentile(&sample, 95); // ceil(4.75) => rank 5 => index 4
    let p100 = nearest_rank_percentile(&sample, 100); // ceil(5.0) => rank 5 => index 4

    // Then computed values match nearest-rank semantics exactly.
    assert_eq!(p0, 10);
    assert_eq!(p1, 10);
    assert_eq!(p20, 10);
    assert_eq!(p21, 20);
    assert_eq!(p50, 30);
    assert_eq!(p95, 50);
    assert_eq!(p100, 50);
}

#[test]
fn scenario_percentiles_above_hundred_are_clamped_to_hundred() {
    // Given a sorted sample window.
    let sample = [5_u64, 15, 25, 35];

    // When we request percentiles that exceed 100.
    let p101 = nearest_rank_percentile(&sample, 101);
    let p500 = nearest_rank_percentile(&sample, 500);
    let p_max = nearest_rank_percentile(&sample, u64::MAX);

    // Then all requests behave exactly like p100.
    let p100 = nearest_rank_percentile(&sample, 100);
    assert_eq!(p101, p100);
    assert_eq!(p500, p100);
    assert_eq!(p_max, p100);
    assert_eq!(p100, 35);
}

#[test]
fn scenario_duplicates_are_valid_percentile_outputs() {
    // Given sorted values with duplicate tail latencies.
    let sample = [1_u64, 2, 2, 2, 3, 10, 10, 10, 10];

    // When we query higher percentiles.
    let p80 = nearest_rank_percentile(&sample, 80);
    let p90 = nearest_rank_percentile(&sample, 90);
    let p99 = nearest_rank_percentile(&sample, 99);

    // Then nearest-rank can legitimately return repeated values.
    assert_eq!(p80, 10);
    assert_eq!(p90, 10);
    assert_eq!(p99, 10);
}
