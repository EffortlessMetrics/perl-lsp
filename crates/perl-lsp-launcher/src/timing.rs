//! Lightweight startup phase timer for profiling server initialization.
//!
//! Records elapsed time between named checkpoints without external dependencies
//! beyond `std::time`. Produces human-readable and machine-readable reports.

use std::fmt;
use std::time::{Duration, Instant};

/// A single timed phase between two checkpoints.
#[derive(Debug, Clone)]
pub struct StartupPhase {
    /// Human-readable phase name (e.g. `"server_construction"`).
    pub name: &'static str,
    /// Wall-clock elapsed time for this phase.
    pub duration: Duration,
}

/// Accumulates elapsed durations between checkpoints.
///
/// Call [`checkpoint`](Self::checkpoint) at each initialization boundary, then
/// [`finish`](Self::finish) to produce an immutable [`StartupReport`].
///
/// # Overhead
///
/// `Instant::now()` is ~20 ns on x86-64. Even 20 checkpoints add ~400 ns —
/// negligible against typical 3–5 s server startup times.
pub struct StartupTimer {
    phases: Vec<StartupPhase>,
    start: Instant,
    last: Instant,
}

impl StartupTimer {
    /// Create a new timer that starts counting immediately.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            phases: Vec::with_capacity(8),
            start: now,
            last: now,
        }
    }

    /// Record elapsed time since the last checkpoint (or start) as `name`.
    pub fn checkpoint(&mut self, name: &'static str) {
        let now = Instant::now();
        self.phases.push(StartupPhase {
            name,
            duration: now - self.last,
        });
        self.last = now;
    }

    /// Consume the timer and produce an immutable report.
    pub fn finish(self) -> StartupReport {
        StartupReport {
            total: self.start.elapsed(),
            phases: self.phases,
        }
    }

    /// Create a timer with a pre-set elapsed duration, for testing.
    ///
    /// The timer is already "finished": calling `finish()` on it returns a
    /// report whose `total` equals `elapsed`. No checkpoints are recorded.
    /// This allows tests to assert on `total` without relying on wall-clock
    /// timing, eliminating the flakiness that occurs on loaded systems or
    /// platforms where `Instant` resolution is coarser than the test interval.
    #[cfg(test)]
    pub(crate) fn new_with_elapsed(elapsed: Duration) -> FrozenTimer {
        FrozenTimer {
            elapsed,
            phases: Vec::new(),
        }
    }
}

/// A timer whose elapsed time is fixed at construction, used in tests.
///
/// Returned by [`StartupTimer::new_with_elapsed`].
#[cfg(test)]
pub(crate) struct FrozenTimer {
    elapsed: Duration,
    phases: Vec<StartupPhase>,
}

#[cfg(test)]
impl FrozenTimer {
    pub(crate) fn finish(self) -> StartupReport {
        StartupReport {
            total: self.elapsed,
            phases: self.phases,
        }
    }
}

impl Default for StartupTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of completed startup timing.
#[derive(Debug, Clone)]
pub struct StartupReport {
    /// Total wall-clock time from timer creation to [`finish`](StartupTimer::finish).
    pub total: Duration,
    /// Ordered list of phase timings.
    pub phases: Vec<StartupPhase>,
}

impl StartupReport {
    /// Emit a human-readable breakdown to `stderr` via [`tracing`].
    ///
    /// Individual phases are logged at `debug` level; the total is logged at
    /// `info` level. This keeps normal output clean while allowing detailed
    /// profiling with `PERL_LSP_LOG=perl_lsp_launcher=debug`.
    pub fn log(&self) {
        for phase in &self.phases {
            tracing::debug!(
                startup_phase = phase.name,
                elapsed_ms = phase.duration.as_millis() as u64,
                "startup phase completed"
            );
        }
        tracing::info!(
            startup_total_ms = self.total.as_millis() as u64,
            phase_count = self.phases.len(),
            "server startup complete"
        );
    }

    /// Produce a compact JSON string for machine consumption (CI, benchmarks).
    ///
    /// Does not depend on `serde_json`; constructs JSON manually to avoid
    /// adding a crate dependency to the launcher.
    pub fn to_json(&self) -> String {
        let mut buf = String::with_capacity(256);
        buf.push_str("{\"total_ms\":");
        buf.push_str(&self.total.as_millis().to_string());
        buf.push_str(",\"phases\":[");
        for (i, phase) in self.phases.iter().enumerate() {
            if i > 0 {
                buf.push(',');
            }
            buf.push_str("{\"name\":\"");
            buf.push_str(phase.name);
            buf.push_str("\",\"elapsed_ms\":");
            buf.push_str(&phase.duration.as_millis().to_string());
            buf.push('}');
        }
        buf.push_str("]}");
        buf
    }
}

impl fmt::Display for StartupReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Startup timing (total: {:.1} ms)",
            self.total.as_secs_f64() * 1000.0
        )?;
        if self.phases.is_empty() {
            writeln!(f, "  (no phases recorded)")?;
            return Ok(());
        }
        for phase in &self.phases {
            let ms = phase.duration.as_secs_f64() * 1000.0;
            let pct = if self.total.as_nanos() > 0 {
                (phase.duration.as_nanos() as f64 / self.total.as_nanos() as f64) * 100.0
            } else {
                0.0
            };
            writeln!(f, "  {:<30}  {:>8.1} ms  ({:>5.1}%)", phase.name, ms, pct)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_records_phases() {
        let mut t = StartupTimer::new();
        std::thread::sleep(Duration::from_millis(10));
        t.checkpoint("phase_a");
        std::thread::sleep(Duration::from_millis(5));
        t.checkpoint("phase_b");
        let report = t.finish();

        assert!(report.total.as_millis() >= 15);
        assert_eq!(report.phases.len(), 2);
        assert_eq!(report.phases[0].name, "phase_a");
        assert_eq!(report.phases[1].name, "phase_b");
        assert!(report.phases[0].duration.as_millis() >= 10);
        assert!(report.phases[1].duration.as_millis() >= 5);
    }

    #[test]
    fn empty_timer_reports_total() {
        // Use a frozen timer with a known elapsed duration so the assertion
        // does not depend on wall-clock resolution.  On loaded systems or
        // platforms with coarse `Instant` granularity, two back-to-back
        // `Instant::now()` calls can return the same value, making
        // `elapsed().as_nanos() == 0` and the old `> 0` assertion flaky.
        let frozen = StartupTimer::new_with_elapsed(Duration::from_millis(42));
        let report = frozen.finish();
        assert_eq!(report.phases.len(), 0);
        assert_eq!(report.total, Duration::from_millis(42));
    }

    #[test]
    fn to_json_is_valid() {
        let mut t = StartupTimer::new();
        t.checkpoint("a");
        let report = t.finish();
        let json = report.to_json();
        assert!(json.starts_with("{\"total_ms\":"));
        assert!(json.contains("\"name\":\"a\""));
        assert!(json.ends_with("]}"));
        // Basic JSON structure: can round-trip through string search
        assert!(json.contains("\"elapsed_ms\":"));
    }

    #[test]
    fn display_contains_phase_names() {
        let mut t = StartupTimer::new();
        t.checkpoint("hello");
        t.checkpoint("world");
        let report = t.finish();
        let s = format!("{report}");
        assert!(s.contains("hello"));
        assert!(s.contains("world"));
        assert!(s.contains("Startup timing"));
    }
}
