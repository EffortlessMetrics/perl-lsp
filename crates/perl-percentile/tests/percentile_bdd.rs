use perl_percentile::nearest_rank_percentile;

#[test]
fn given_empty_samples_when_requesting_any_percentile_then_returns_zero() {
    // Given
    let sorted_values: [u64; 0] = [];

    // When
    let result = nearest_rank_percentile(&sorted_values, 95);

    // Then
    assert_eq!(result, 0);
}

#[test]
fn given_sorted_samples_when_requesting_p0_then_returns_first_value() {
    // Given
    let sorted_values = [3, 8, 13, 21];

    // When
    let result = nearest_rank_percentile(&sorted_values, 0);

    // Then
    assert_eq!(result, 3);
}

#[test]
fn given_sorted_samples_when_requesting_p50_then_returns_nearest_rank_median() {
    // Given
    let sorted_values = [10, 20, 30, 40, 50, 60];

    // When
    let result = nearest_rank_percentile(&sorted_values, 50);

    // Then
    // rank = ceil((50 / 100) * 6) = 3 => index 2 => 30
    assert_eq!(result, 30);
}

#[test]
fn given_sorted_samples_when_requesting_p95_then_returns_upper_tail_value() {
    // Given
    let sorted_values = [100, 110, 130, 160, 200, 280, 310, 400, 700, 1_200];

    // When
    let result = nearest_rank_percentile(&sorted_values, 95);

    // Then
    // rank = ceil((95 / 100) * 10) = 10 => index 9 => 1200
    assert_eq!(result, 1_200);
}

#[test]
fn given_duplicate_values_when_requesting_percentile_then_result_can_be_duplicate_member() {
    // Given
    let sorted_values = [5, 5, 5, 10, 10, 20];

    // When
    let result = nearest_rank_percentile(&sorted_values, 60);

    // Then
    // rank = ceil((60 / 100) * 6) = 4 => index 3 => 10
    assert_eq!(result, 10);
}

#[test]
fn given_percentile_over_100_when_requesting_percentile_then_value_is_clamped_to_last_sample() {
    // Given
    let sorted_values = [7, 14, 21];

    // When
    let result = nearest_rank_percentile(&sorted_values, 250);

    // Then
    assert_eq!(result, 21);
}
