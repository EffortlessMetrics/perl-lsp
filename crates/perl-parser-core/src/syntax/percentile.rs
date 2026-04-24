//! Percentile helpers for integer metric samples (previously `perl-percentile`).
//!
//! Computes nearest-rank percentiles for pre-sorted `u64` samples.

/// Compute the nearest-rank percentile from a sorted sample slice.
///
/// The nearest-rank definition uses:
///
/// - `rank = ceil((pct / 100) * n)` where `n` is sample length.
/// - returned value is sample at `rank - 1` (1-based to 0-based conversion).
///
/// # Parameters
///
/// - `sorted_values`: sample values sorted in ascending order.
/// - `pct`: percentile in the range `0..=100`.
///
/// # Returns
///
/// - `0` when `sorted_values` is empty.
/// - nearest-rank percentile value otherwise.
#[must_use]
pub fn nearest_rank_percentile(sorted_values: &[u64], pct: u64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }

    let pct_clamped = pct.min(100);
    let rank = ((pct_clamped as f64 / 100.0) * sorted_values.len() as f64).ceil() as usize;
    sorted_values[rank.min(sorted_values.len()).saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use super::nearest_rank_percentile;

    #[test]
    fn nearest_rank_handles_empty_input() {
        assert_eq!(nearest_rank_percentile(&[], 95), 0);
    }

    #[test]
    fn nearest_rank_percentiles_match_expected_values() {
        let sorted = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(nearest_rank_percentile(&sorted, 50), 5);
        assert_eq!(nearest_rank_percentile(&sorted, 95), 10);
        assert_eq!(nearest_rank_percentile(&sorted, 99), 10);
    }

    #[test]
    fn nearest_rank_clamps_high_percentiles() {
        let sorted = [10, 20, 30];
        assert_eq!(nearest_rank_percentile(&sorted, 1000), 30);
    }
}
