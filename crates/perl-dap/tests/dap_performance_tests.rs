//! DAP Performance Tests (AC15)
//!
//! Practical performance checks for critical adapter paths.
//!
//! Run with: `cargo test -p perl-dap --features dap-phase2`

#[cfg(feature = "dap-phase2")]
mod dap_performance {
    use anyhow::Result;
    use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
    use serde_json::json;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::NamedTempFile;

    fn p95(mut samples: Vec<Duration>) -> Duration {
        samples.sort_unstable();
        let idx = ((samples.len() as f64) * 0.95).ceil() as usize;
        samples[idx.saturating_sub(1).min(samples.len().saturating_sub(1))]
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac15-step-continue-latency
    #[tokio::test]
    // AC:15
    async fn test_step_continue_latency_p95() -> Result<()> {
        let mut adapter = DebugAdapter::new();
        let mut samples = Vec::new();

        for i in 0..50 {
            let start = Instant::now();
            let response = if i % 2 == 0 {
                adapter.handle_request(i + 1, "continue", Some(json!({ "threadId": 1 })))
            } else {
                adapter.handle_request(i + 1, "next", Some(json!({ "threadId": 1 })))
            };
            match response {
                DapMessage::Response { success, .. } => assert!(success),
                _ => anyhow::bail!("expected response"),
            }
            samples.push(start.elapsed());
        }

        let p95_latency = p95(samples);
        assert!(
            p95_latency < Duration::from_millis(100),
            "p95 step/continue latency too high: {:?}",
            p95_latency
        );
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac15-breakpoint-verification-latency
    #[tokio::test]
    // AC:15
    async fn test_breakpoint_verification_latency() -> Result<()> {
        let mut fixture = NamedTempFile::new()?;
        for i in 0..200 {
            writeln!(fixture, "my $v{} = {};", i, i)?;
        }
        fixture.flush()?;
        let fixture_path = fixture.path().to_string_lossy().to_string();

        let mut adapter = DebugAdapter::new();
        let start = Instant::now();
        let response = adapter.handle_request(
            1,
            "setBreakpoints",
            Some(json!({
                "source": { "path": fixture_path },
                "breakpoints": (1..=100).map(|line| json!({ "line": line })).collect::<Vec<_>>()
            })),
        );
        let elapsed = start.elapsed();
        match response {
            DapMessage::Response { success, .. } => assert!(success),
            _ => anyhow::bail!("expected setBreakpoints response"),
        }
        assert!(
            elapsed < Duration::from_millis(250),
            "breakpoint verification took too long: {:?}",
            elapsed
        );
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac15-variable-expansion-latency
    #[tokio::test]
    // AC:15
    async fn test_variable_expansion_latency() -> Result<()> {
        let mut adapter = DebugAdapter::new();

        let first_start = Instant::now();
        let first = adapter.handle_request(
            1,
            "variables",
            Some(json!({
                "variablesReference": 11,
                "start": 0,
                "count": 50
            })),
        );
        let first_elapsed = first_start.elapsed();
        let child_ref = match first {
            DapMessage::Response { success, body, .. } => {
                assert!(success);
                let vars = body
                    .and_then(|b| b.get("variables").cloned())
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                vars.iter()
                    .find_map(|v| v.get("variablesReference").and_then(|n| n.as_i64()))
                    .unwrap_or(0)
            }
            _ => anyhow::bail!("expected variables response"),
        };
        assert!(
            first_elapsed < Duration::from_millis(200),
            "initial variables request too slow: {:?}",
            first_elapsed
        );

        if child_ref > 0 {
            let child_start = Instant::now();
            let child = adapter.handle_request(
                2,
                "variables",
                Some(json!({
                    "variablesReference": child_ref,
                    "start": 0,
                    "count": 100
                })),
            );
            let child_elapsed = child_start.elapsed();
            match child {
                DapMessage::Response { success, .. } => assert!(success),
                _ => anyhow::bail!("expected child variables response"),
            }
            assert!(
                child_elapsed < Duration::from_millis(100),
                "child variable expansion too slow: {:?}",
                child_elapsed
            );
        }

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac15-large-file-benchmarks
    #[tokio::test]
    // AC:15
    async fn test_large_file_benchmarks() -> Result<()> {
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/performance/large_file.pl");
        let fixture_path = fixture.to_string_lossy().to_string();

        let mut adapter = DebugAdapter::new();
        let start = Instant::now();
        let response = adapter.handle_request(
            1,
            "setBreakpoints",
            Some(json!({
                "source": { "path": fixture_path },
                "breakpoints": [{ "line": 10 }, { "line": 100 }, { "line": 500 }]
            })),
        );
        let elapsed = start.elapsed();
        match response {
            DapMessage::Response { success, .. } => assert!(success),
            _ => anyhow::bail!("expected setBreakpoints response"),
        }
        assert!(
            elapsed < Duration::from_millis(300),
            "large-file breakpoint operation too slow: {:?}",
            elapsed
        );
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac15-memory-footprint
    #[tokio::test]
    // AC:15
    async fn test_memory_footprint_baseline() -> Result<()> {
        let mut fixture = NamedTempFile::new()?;
        fixture.write_all(b"my $x = 1;\nmy $y = 2;\n")?;
        fixture.flush()?;
        let fixture_path = fixture.path().to_string_lossy().to_string();

        let mut adapter = DebugAdapter::new();
        for seq in 1..=200 {
            let response = adapter.handle_request(
                seq,
                "setBreakpoints",
                Some(json!({
                    "source": { "path": fixture_path },
                    "breakpoints": [{ "line": 1 }, { "line": 2 }]
                })),
            );
            match response {
                DapMessage::Response { success, .. } => assert!(success),
                _ => anyhow::bail!("expected setBreakpoints response"),
            }
        }
        // If replace semantics are working, the adapter should remain responsive and bounded.
        let response = adapter.handle_request(201, "threads", None);
        match response {
            DapMessage::Response { success, .. } => assert!(success),
            _ => anyhow::bail!("expected threads response"),
        }
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac15-concurrent-sessions
    #[tokio::test]
    // AC:15
    async fn test_concurrent_session_performance() -> Result<()> {
        let workers = 8usize;
        let results = Arc::new(Mutex::new(Vec::with_capacity(workers)));
        let start = Instant::now();

        let mut handles = Vec::new();
        for idx in 0..workers {
            let results = Arc::clone(&results);
            handles.push(thread::spawn(move || {
                let mut adapter = DebugAdapter::new();
                let init = adapter.handle_request((idx as i64) + 1, "initialize", None);
                let threads = adapter.handle_request((idx as i64) + 100, "threads", None);
                let mut ok = false;
                if let DapMessage::Response { success, .. } = init
                    && success
                    && let DapMessage::Response { success: true, .. } = threads
                {
                    ok = true;
                }
                results.lock().unwrap_or_else(|e| e.into_inner()).push(ok);
            }));
        }

        for handle in handles {
            handle.join().map_err(|_| anyhow::anyhow!("worker thread panicked"))?;
        }
        let elapsed = start.elapsed();
        let results = results.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(results.len(), workers);
        assert!(results.iter().all(|ok| *ok), "all concurrent workers must succeed");
        assert!(
            elapsed < Duration::from_secs(2),
            "concurrent session check exceeded budget: {:?}",
            elapsed
        );
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac15-regression-detection
    #[test]
    // AC:15
    fn test_performance_regression_detection() -> Result<()> {
        let mut adapter = DebugAdapter::new();
        let mut samples = Vec::new();

        for seq in 1..=100 {
            let start = Instant::now();
            let response = adapter.handle_request(seq, "threads", None);
            match response {
                DapMessage::Response { success, .. } => assert!(success),
                _ => anyhow::bail!("expected threads response"),
            }
            samples.push(start.elapsed());
        }

        let p95_latency = p95(samples);
        // Regression guardrail for a trivial in-memory request.
        assert!(
            p95_latency < Duration::from_millis(10),
            "threads regression detected, p95 {:?} exceeds baseline guardrail",
            p95_latency
        );
        Ok(())
    }
}
