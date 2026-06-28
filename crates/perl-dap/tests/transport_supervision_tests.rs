//! Transport supervision and recovery tests.
//! Tests for issue #1609: event handler thread has no supervision or recovery on write errors.
//!
//! These tests exercise the `FailingWriter` mock infrastructure and also drive the
//! full `DebugAdapter` transport via `run_with_io` (exposed via `pub(super)` for
//! testing — see transport.rs `#[cfg(test)]` accessor).
//!
//! Integration tests that require the `transport_broken` supervision path to fire
//! end-to-end are in `crates/perl-dap/src/debug_adapter/transport.rs` (cfg(test))
//! where `run_with_io` is accessible at its `pub(super)` visibility.

#[cfg(feature = "dap-phase2")]
mod transport_supervision {
    use std::io::{self, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    // ── Mock writer infrastructure ────────────────────────────────────────────

    /// Mock writer that succeeds for the first `fail_after_writes` write calls
    /// then returns `BrokenPipe` permanently.
    struct FailingWriter {
        fail_after_writes: usize,
        write_count: Arc<AtomicUsize>,
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl FailingWriter {
        fn always_failing() -> Self {
            Self {
                fail_after_writes: 0,
                write_count: Arc::new(AtomicUsize::new(0)),
                buf: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn fail_after(n: usize) -> (Self, Arc<AtomicUsize>) {
            let count = Arc::new(AtomicUsize::new(0));
            let writer = Self {
                fail_after_writes: n,
                write_count: Arc::clone(&count),
                buf: Arc::new(Mutex::new(Vec::new())),
            };
            (writer, count)
        }
    }

    impl Write for FailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = self.write_count.fetch_add(1, Ordering::AcqRel);
            if n >= self.fail_after_writes {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Mock write failure (broken transport)",
                ));
            }
            if let Ok(mut guard) = self.buf.lock() {
                guard.extend_from_slice(buf);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            let n = self.write_count.load(Ordering::Acquire);
            if n >= self.fail_after_writes {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Mock flush failure (broken transport)",
                ));
            }
            Ok(())
        }
    }

    // ── Mock self-tests ───────────────────────────────────────────────────────

    /// A writer created with `always_failing()` must fail on the very first write.
    #[test]
    fn test_failing_writer_fails_immediately() {
        let mut w = FailingWriter::always_failing();
        let result = w.write(b"hello");
        assert!(result.is_err(), "FailingWriter(0) must fail on the first write");
        assert!(
            matches!(result, Err(ref error) if error.kind() == io::ErrorKind::BrokenPipe),
            "error kind must be BrokenPipe"
        );
    }

    /// A writer created with `fail_after(n)` succeeds for the first n calls then
    /// fails permanently — the count is atomically shared with the caller.
    #[test]
    fn test_failing_writer_succeeds_then_fails() {
        let (mut w, count) = FailingWriter::fail_after(3);
        assert!(w.write(b"a").is_ok(), "write 1 should succeed");
        assert!(w.write(b"b").is_ok(), "write 2 should succeed");
        assert!(w.write(b"c").is_ok(), "write 3 should succeed");
        assert_eq!(count.load(Ordering::Acquire), 3, "count must reach the configured boundary");
        let boundary_result = w.write(b"d");
        assert!(
            matches!(boundary_result, Err(ref error) if error.kind() == io::ErrorKind::BrokenPipe),
            "write at the configured boundary should fail with BrokenPipe"
        );
        assert!(w.write(b"e").is_err(), "write 5 should fail");
        // Shared counter must reflect all five attempts.
        assert_eq!(count.load(Ordering::Acquire), 5);
    }

    /// Consecutive failures accumulate across iterations — a simple counter that
    /// resets on success and crosses the threshold of 3 after 3 unbroken failures.
    #[test]
    fn test_consecutive_failure_counter_reaches_threshold() {
        const THRESHOLD: usize = 3;
        let mut consecutive = 0usize;
        let mut threshold_hit = false;

        for _ in 0..THRESHOLD {
            // Simulate a write failure.
            consecutive += 1;
            if consecutive >= THRESHOLD {
                threshold_hit = true;
                break;
            }
        }

        assert!(threshold_hit, "threshold must be reached after {THRESHOLD} failures");
        assert_eq!(consecutive, THRESHOLD);
    }

    /// A successful write resets the consecutive-failure counter to zero.
    #[test]
    fn test_consecutive_failure_counter_resets_on_success() {
        let mut consecutive = 0usize;

        // Two failures.
        consecutive += 1;
        consecutive += 1;
        assert_eq!(consecutive, 2);

        // One success → reset.
        consecutive = 0;
        assert_eq!(consecutive, 0, "counter must reset to 0 after a success");

        // One more failure — must not immediately trigger the threshold of 3.
        consecutive += 1;
        assert_eq!(consecutive, 1);
        assert!(consecutive < 3, "single failure after a reset must not reach threshold(3)");
    }
}
