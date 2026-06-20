//! Process allocation telemetry for xtask measurement commands.

use color_eyre::eyre::Result;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[cfg(target_os = "linux")]
use procfs::process::Process;

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

static CURRENT_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
static WINDOW_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static WINDOW_ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);

pub(crate) struct AllocationMeasurement {
    pub(crate) allocated_bytes: u64,
    pub(crate) allocation_count: u64,
    pub(crate) peak_delta_bytes: u64,
}

impl AllocationMeasurement {
    pub(crate) fn peak_delta_mb(&self) -> f64 {
        bytes_to_mb(self.peak_delta_bytes)
    }
}

struct TrackingAllocator;

pub(crate) fn measure_allocations<F, R>(operation: F) -> (R, AllocationMeasurement)
where
    F: FnOnce() -> R,
{
    let baseline = reset_allocation_window();
    let result = operation();
    (result, allocation_measurement(baseline))
}

/// Memory measurement helper that provides safe fallback behavior.
pub(crate) fn measure_memory_usage<F, R>(operation: F) -> (R, f64)
where
    F: FnOnce() -> R,
{
    let memory_before = get_current_memory_usage().unwrap_or(0.0);
    let baseline = reset_allocation_window();

    let result = operation();

    let memory_after = get_current_memory_usage().unwrap_or(0.0);
    let peak_memory_mb = bytes_to_mb(allocation_measurement(baseline).peak_delta_bytes);

    let memory_mb = measured_memory_mb(memory_before, memory_after, peak_memory_mb);

    (result, memory_mb)
}

/// Get current process memory usage in MB using procfs on Linux.
pub(crate) fn get_current_memory_usage() -> Result<f64> {
    get_platform_current_memory_usage()
}

fn reset_allocation_window() -> usize {
    let current = CURRENT_BYTES.load(Ordering::Relaxed);
    WINDOW_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    WINDOW_ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    PEAK_BYTES.store(current, Ordering::Relaxed);
    current
}

fn allocation_measurement(baseline: usize) -> AllocationMeasurement {
    let peak = PEAK_BYTES.load(Ordering::Relaxed);
    AllocationMeasurement {
        allocated_bytes: WINDOW_ALLOCATED_BYTES.load(Ordering::Relaxed),
        allocation_count: WINDOW_ALLOCATION_COUNT.load(Ordering::Relaxed),
        peak_delta_bytes: peak.saturating_sub(baseline) as u64,
    }
}

fn record_allocation(size: usize) {
    if size == 0 {
        return;
    }
    WINDOW_ALLOCATED_BYTES.fetch_add(size as u64, Ordering::Relaxed);
    WINDOW_ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
}

fn add_current(size: usize) {
    let previous = CURRENT_BYTES.fetch_add(size, Ordering::Relaxed);
    PEAK_BYTES.fetch_max(previous.saturating_add(size), Ordering::Relaxed);
}

fn subtract_current(size: usize) {
    let _ = CURRENT_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(size))
    });
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn measured_memory_mb(memory_before: f64, memory_after: f64, peak_memory_mb: f64) -> f64 {
    let memory_delta = memory_after - memory_before;
    if memory_delta > 0.0 { memory_delta } else { peak_memory_mb }
}

#[cfg(target_os = "linux")]
fn get_platform_current_memory_usage() -> Result<f64> {
    let pid = std::process::id() as i32;
    let process = Process::new(pid)?;
    let statm = process.statm()?;
    let page_size = procfs::page_size();
    let rss_bytes = statm.resident.saturating_mul(page_size);
    Ok(rss_bytes as f64 / (1024.0 * 1024.0))
}

#[cfg(not(target_os = "linux"))]
fn get_platform_current_memory_usage() -> Result<f64> {
    Ok(0.0)
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `GlobalAlloc::alloc` callers uphold `layout`; this forwards to `System`.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            add_current(layout.size());
            record_allocation(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `GlobalAlloc::dealloc` callers provide the original allocation layout.
        unsafe { System.dealloc(ptr, layout) };
        subtract_current(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `GlobalAlloc::alloc_zeroed` callers uphold `layout`; this forwards to `System`.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            add_current(layout.size());
            record_allocation(layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: `GlobalAlloc::realloc` callers provide the original allocation layout.
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            let old_size = layout.size();
            if new_size >= old_size {
                add_current(new_size - old_size);
            } else {
                subtract_current(old_size - new_size);
            }
            record_allocation(new_size);
        }
        new_ptr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::ensure;

    #[test]
    fn allocation_tracker_bytes_to_mb_uses_binary_mebibytes() -> Result<()> {
        let cases = [(0, 0.0), (524_288, 0.5), (1_048_576, 1.0), (2_621_440, 2.5)];

        for (bytes, expected) in cases {
            let actual = bytes_to_mb(bytes);
            ensure!(
                (actual - expected).abs() < f64::EPSILON,
                "expected {bytes} bytes to be {expected} MiB, got {actual}"
            );
        }

        Ok(())
    }

    #[test]
    fn allocation_tracker_measured_memory_prefers_rss_delta_before_allocator_fallback() -> Result<()>
    {
        let cases = [
            ("rss grew", 10.0, 12.5, 1.0, 2.5),
            ("rss flat", 10.0, 10.0, 1.25, 1.25),
            ("rss shrank", 10.0, 8.0, 0.75, 0.75),
        ];

        for (name, memory_before, memory_after, peak_memory_mb, expected) in cases {
            let actual = measured_memory_mb(memory_before, memory_after, peak_memory_mb);
            ensure!(
                (actual - expected).abs() < f64::EPSILON,
                "{name}: expected {expected}, got {actual}"
            );
        }

        Ok(())
    }

    #[test]
    fn allocation_tracker_peak_delta_mb_reports_mebibytes() -> Result<()> {
        let measurement = AllocationMeasurement {
            allocated_bytes: 0,
            allocation_count: 0,
            peak_delta_bytes: 1_572_864,
        };

        ensure!(
            (measurement.peak_delta_mb() - 1.5).abs() < f64::EPSILON,
            "expected 1.5 MiB, got {}",
            measurement.peak_delta_mb()
        );

        Ok(())
    }

    #[test]
    fn allocation_tracker_measure_allocations_preserves_result() -> Result<()> {
        let (result, _measurement) = measure_allocations(|| {
            let mut values = Vec::with_capacity(1024);
            values.extend(0..1024_usize);
            values.len()
        });

        ensure!(result == 1024, "operation result must be preserved");

        Ok(())
    }

    #[test]
    fn allocation_tracker_measurement_saturates_when_baseline_exceeds_peak() -> Result<()> {
        let measurement = allocation_measurement(usize::MAX);

        ensure!(
            measurement.peak_delta_bytes == 0,
            "peak delta must saturate instead of underflowing"
        );

        Ok(())
    }
}
