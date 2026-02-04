## 2026-02-04 - Optimizing Regex in Hot Paths
**Learning:** `Regex::new` is expensive. Inside `SymbolExtractor::extract_vars_from_string` (called for every interpolated string), it caused a ~720x slowdown (17s vs 23ms for 5000 strings).
**Action:** Always hoist `Regex` compilation out of hot loops/functions using `std::sync::OnceLock`.
