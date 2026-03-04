//! Reusable percentile and latency summary utilities for SLO tracking.

/// Latency summary derived from a set of durations measured in milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LatencySummary {
    /// P50 latency (median)
    pub p50_ms: u64,
    /// P95 latency
    pub p95_ms: u64,
    /// P99 latency
    pub p99_ms: u64,
    /// Average latency in milliseconds
    pub avg_ms: f64,
}

/// Compute percentile latency and average from millisecond durations.
///
/// Returns default values when `durations_ms` is empty.
pub fn summarize_latencies(durations_ms: &[u64]) -> LatencySummary {
    if durations_ms.is_empty() {
        return LatencySummary::default();
    }

    let mut sorted = durations_ms.to_vec();
    sorted.sort_unstable();

    let avg_ms = sorted.iter().map(|&d| d as f64).sum::<f64>() / sorted.len() as f64;

    LatencySummary {
        p50_ms: percentile(&sorted, 50),
        p95_ms: percentile(&sorted, 95),
        p99_ms: percentile(&sorted, 99),
        avg_ms,
    }
}

/// Calculate a percentile from a sorted slice of values using the nearest-rank method.
pub fn percentile(sorted_values: &[u64], pct: u64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }

    let rank = ((pct as f64 / 100.0) * sorted_values.len() as f64).ceil() as usize;
    sorted_values[rank.min(sorted_values.len()).saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_handles_empty_slice() {
        assert_eq!(percentile(&[], 50), 0);
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        let values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&values, 50), 5);
        assert_eq!(percentile(&values, 95), 10);
    }

    #[test]
    fn summarize_latencies_computes_expected_values() {
        let summary = summarize_latencies(&[10, 1, 7, 2]);
        assert_eq!(summary.p50_ms, 2);
        assert_eq!(summary.p95_ms, 10);
        assert_eq!(summary.p99_ms, 10);
        assert_eq!(summary.avg_ms, 5.0);
    }

    #[test]
    fn summarize_latencies_handles_empty_input() {
        assert_eq!(summarize_latencies(&[]), LatencySummary::default());
    }
}
