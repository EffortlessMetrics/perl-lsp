use perl_percentile::nearest_rank_percentile;

#[derive(Debug)]
struct PercentileScenario {
    name: &'static str,
    given_sorted_values: &'static [u64],
    when_pct: u64,
    then_expected: u64,
}

fn assert_scenario(scenario: &PercentileScenario) {
    let actual = nearest_rank_percentile(scenario.given_sorted_values, scenario.when_pct);
    assert_eq!(actual, scenario.then_expected, "scenario '{}' failed", scenario.name);
}

#[test]
fn bdd_nearest_rank_core_scenarios() {
    let scenarios = [
        PercentileScenario {
            name: "Given an empty sample when percentile requested then returns zero",
            given_sorted_values: &[],
            when_pct: 95,
            then_expected: 0,
        },
        PercentileScenario {
            name: "Given sorted values when percentile is zero then first element is returned",
            given_sorted_values: &[5, 15, 25, 35],
            when_pct: 0,
            then_expected: 5,
        },
        PercentileScenario {
            name: "Given sorted values when percentile is 50 then nearest-rank median is returned",
            given_sorted_values: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            when_pct: 50,
            then_expected: 5,
        },
        PercentileScenario {
            name: "Given duplicate values when percentile lands on duplicate rank then duplicate value is returned",
            given_sorted_values: &[10, 10, 20, 20, 30],
            when_pct: 40,
            then_expected: 10,
        },
        PercentileScenario {
            name: "Given sorted values when percentile is over 100 then percentile is clamped to max",
            given_sorted_values: &[2, 4, 6, 8],
            when_pct: 500,
            then_expected: 8,
        },
    ];

    for scenario in &scenarios {
        assert_scenario(scenario);
    }
}

#[test]
fn bdd_nearest_rank_is_monotonic_across_percentiles() {
    // Given
    let sorted_values = [10, 20, 30, 40, 50, 60, 70, 80];

    // When
    let checkpoints = [0, 1, 10, 25, 50, 75, 95, 100, 200];
    let results: Vec<u64> =
        checkpoints.iter().map(|pct| nearest_rank_percentile(&sorted_values, *pct)).collect();

    // Then
    for pair in results.windows(2) {
        assert!(pair[0] <= pair[1], "expected monotonic percentile outputs, got {results:?}");
    }
}
