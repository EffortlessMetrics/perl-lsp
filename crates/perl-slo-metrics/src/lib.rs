//! Shared latency metric helpers for SLO-oriented crates.

/// Aggregated latency metrics derived from a sample set in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatencySummary {
    /// P50 latency (median).
    pub p50_ms: u64,
    /// P95 latency.
    pub p95_ms: u64,
    /// P99 latency.
    pub p99_ms: u64,
    /// Average latency.
    pub avg_ms: f64,
}

impl LatencySummary {
    /// Build a latency summary from sorted millisecond values.
    pub fn from_sorted_ms(sorted_values: &[u64]) -> Self {
        if sorted_values.is_empty() {
            return Self::default();
        }

        let p50_ms = percentile_nearest_rank(sorted_values, 50);
        let p95_ms = percentile_nearest_rank(sorted_values, 95);
        let p99_ms = percentile_nearest_rank(sorted_values, 99);
        let avg_ms = sorted_values.iter().map(|&duration| duration as f64).sum::<f64>()
            / sorted_values.len() as f64;

        Self { p50_ms, p95_ms, p99_ms, avg_ms }
    }
}

impl Default for LatencySummary {
    fn default() -> Self {
        Self { p50_ms: 0, p95_ms: 0, p99_ms: 0, avg_ms: 0.0 }
    }
}

/// Calculate a percentile from sorted values using the nearest-rank method.
#[must_use]
pub fn percentile_nearest_rank(sorted_values: &[u64], percentile: u64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }

    let rank = ((percentile as f64 / 100.0) * sorted_values.len() as f64).ceil() as usize;
    sorted_values[rank.min(sorted_values.len()).saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_returns_zero_for_empty_input() {
        assert_eq!(percentile_nearest_rank(&[], 95), 0);
    }

    #[test]
    fn percentile_calculates_nearest_rank() {
        let sorted_values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile_nearest_rank(&sorted_values, 50), 5);
        assert_eq!(percentile_nearest_rank(&sorted_values, 95), 10);
    }

    #[test]
    fn summary_calculates_expected_percentiles_and_average() {
        let sorted_values = [1, 2, 3, 4, 5];
        let summary = LatencySummary::from_sorted_ms(&sorted_values);
        assert_eq!(summary.p50_ms, 3);
        assert_eq!(summary.p95_ms, 5);
        assert_eq!(summary.p99_ms, 5);
        assert_eq!(summary.avg_ms, 3.0);
    }
}
