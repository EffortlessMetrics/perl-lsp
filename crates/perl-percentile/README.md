# perl-percentile

Tiny single-responsibility helpers for percentile computation on integer samples.

## Provided API

- `nearest_rank_percentile(sorted_values, pct)` computes nearest-rank percentiles
  from pre-sorted `u64` samples.

The function is intentionally minimal and allocation-free so higher-level crates
can decide how to sort and manage sample windows.
