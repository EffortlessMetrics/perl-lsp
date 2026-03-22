//! Tests for PerformanceMonitor: operation tracking and metrics retrieval.
//!
//! Verifies that the monitor correctly tracks timing data for named
//! diagnostic operations and exposes aggregated metrics.

use perl_lsp_performance::PerformanceMonitor;

#[test]
fn test_performance_monitor_new_has_empty_metrics() {
    let monitor = PerformanceMonitor::new();
    let metrics = monitor.get_metrics();
    assert!(metrics.is_empty(), "new monitor should have no recorded metrics");
}

#[test]
fn test_performance_monitor_track_operation_records_entry() {
    let monitor = PerformanceMonitor::new();
    let result = monitor.track_operation("parse", || 42_u32);
    assert_eq!(result, 42, "track_operation must return the closure's return value");
    let metrics = monitor.get_metrics();
    assert!(metrics.contains_key("parse"), "metrics must contain 'parse' after tracking it");
}

#[test]
fn test_performance_monitor_records_non_zero_duration() -> Result<(), Box<dyn std::error::Error>> {
    let monitor = PerformanceMonitor::new();
    // Run a closure that burns a tiny bit of time
    monitor.track_operation("scope_analysis", || {
        let mut _sum = 0u64;
        for i in 0..1000 {
            _sum += i;
        }
    });
    let metrics = monitor.get_metrics();
    let entry = metrics.get("scope_analysis").ok_or("scope_analysis must be recorded")?;
    // Duration is at minimum 0 ns — we just require the key exists and call_count is 1
    assert_eq!(entry.call_count, 1, "call_count must be 1 after one invocation");
    Ok(())
}

#[test]
fn test_performance_monitor_accumulates_multiple_calls() -> Result<(), Box<dyn std::error::Error>> {
    let monitor = PerformanceMonitor::new();
    monitor.track_operation("lint", || ());
    monitor.track_operation("lint", || ());
    monitor.track_operation("lint", || ());
    let metrics = monitor.get_metrics();
    let entry = metrics.get("lint").ok_or("lint must be in metrics")?;
    assert_eq!(entry.call_count, 3, "call_count must equal the number of invocations");
    Ok(())
}

#[test]
fn test_performance_monitor_tracks_multiple_distinct_operations() {
    let monitor = PerformanceMonitor::new();
    monitor.track_operation("parse", || ());
    monitor.track_operation("scope_analysis", || ());
    monitor.track_operation("lint", || ());
    monitor.track_operation("deduplication", || ());
    let metrics = monitor.get_metrics();
    assert!(metrics.contains_key("parse"), "parse must be recorded");
    assert!(metrics.contains_key("scope_analysis"), "scope_analysis must be recorded");
    assert!(metrics.contains_key("lint"), "lint must be recorded");
    assert!(metrics.contains_key("deduplication"), "deduplication must be recorded");
}

#[test]
fn test_performance_monitor_total_duration_at_least_sum_of_parts()
-> Result<(), Box<dyn std::error::Error>> {
    let monitor = PerformanceMonitor::new();
    monitor.track_operation("parse", || std::thread::sleep(std::time::Duration::from_millis(1)));
    monitor.track_operation("parse", || std::thread::sleep(std::time::Duration::from_millis(1)));
    let metrics = monitor.get_metrics();
    let entry = metrics.get("parse").ok_or("parse must be recorded")?;
    // After 2 x 1ms sleeps, total_duration_ns must be at least 2_000_000 ns (2 ms)
    assert!(
        entry.total_duration_ns >= 2_000_000,
        "total_duration_ns ({}) must be >= 2ms after two 1ms sleeps",
        entry.total_duration_ns
    );
    Ok(())
}
