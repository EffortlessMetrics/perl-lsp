//! Test Reliability Infrastructure for LSP Tests
//!
//! This module provides utilities to improve LSP test reliability through:
//! - Test environment validation
//! - Resource monitoring
//! - Health checks
//! - Graceful degradation
//! - Enhanced error reporting
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::common::test_reliability::{TestEnvironment, HealthCheck};
//!
//! #[test]
//! fn test_with_environment_validation() -> Result<(), String> {
//!     // Validate environment before running test
//!     let env = TestEnvironment::validate()?;
//!     eprintln!("Test environment: {}", env.summary());
//!
//!     let mut server = start_lsp_server();
//!
//!     // Perform health check
//!     HealthCheck::new(&mut server).verify()?;
//!
//!     // Run test...
//!     Ok(())
//! }
//! ```

#![allow(dead_code)]

use std::time::{Duration, Instant};

/// Test environment information for diagnostics
#[derive(Debug, Clone)]
pub struct TestEnvironment {
    /// Number of available threads
    pub thread_count: usize,
    /// Whether running in CI
    pub is_ci: bool,
    /// Whether running in containerized environment
    pub is_containerized: bool,
    /// Whether running in WSL
    pub is_wsl: bool,
    /// Available memory (if detectable)
    pub available_memory_mb: Option<u64>,
}

impl TestEnvironment {
    /// Detect and validate the current test environment
    pub fn validate() -> Result<Self, String> {
        let thread_count = std::env::var("RUST_TEST_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8));

        let is_ci = std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("CONTINUOUS_INTEGRATION").is_ok();

        let is_containerized = std::env::var("DOCKER_CONTAINER").is_ok()
            || std::path::Path::new("/.dockerenv").exists()
            || std::env::var("KUBERNETES_SERVICE_HOST").is_ok();

        let is_wsl = std::env::var("WSL_DISTRO_NAME").is_ok() || std::env::var("WSLENV").is_ok();

        let available_memory_mb = detect_available_memory();

        // Validate minimum requirements
        if thread_count == 0 {
            return Err("Invalid thread count (0) detected".to_string());
        }

        // Warn about constrained environments
        if thread_count <= 2 && !is_ci {
            eprintln!("⚠️  WARNING: Running with {} threads - tests may be slow", thread_count);
        }

        if is_containerized && available_memory_mb.is_some_and(|mem| mem < 512) {
            eprintln!(
                "⚠️  WARNING: Low memory detected ({} MB) - tests may fail",
                available_memory_mb.unwrap_or(0)
            );
        }

        Ok(Self { thread_count, is_ci, is_containerized, is_wsl, available_memory_mb })
    }

    /// Get a human-readable summary of the test environment
    pub fn summary(&self) -> String {
        format!(
            "threads={} CI={} container={} WSL={} mem={}MB",
            self.thread_count,
            self.is_ci,
            self.is_containerized,
            self.is_wsl,
            self.available_memory_mb
                .map(|m| m.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )
    }

    /// Check if the environment is constrained
    pub fn is_constrained(&self) -> bool {
        self.thread_count <= 2
            || self.is_containerized
            || self.available_memory_mb.is_some_and(|mem| mem < 1024)
    }

    /// Get recommended timeout multiplier for this environment
    pub fn timeout_multiplier(&self) -> f64 {
        let mut multiplier = 1.0;

        if self.is_ci {
            multiplier *= 1.5;
        }
        if self.is_containerized {
            multiplier *= 1.3;
        }
        if self.is_wsl {
            multiplier *= 1.2;
        }
        if self.thread_count <= 2 {
            multiplier *= 2.0;
        }

        multiplier
    }
}

/// Detect available system memory in MB
fn detect_available_memory() -> Option<u64> {
    // Try /proc/meminfo on Linux
    if cfg!(target_os = "linux") {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return Some(kb / 1024); // Convert to MB
                        }
                    }
                }
            }
        }
    }

    // Could add platform-specific detection for macOS/Windows here
    None
}

/// Health check utilities for LSP server
pub struct HealthCheck<'a> {
    server: &'a mut crate::common::LspServer,
    timeout: Duration,
}

impl<'a> HealthCheck<'a> {
    /// Create a new health check for the given server
    pub fn new(server: &'a mut crate::common::LspServer) -> Self {
        Self { server, timeout: Duration::from_secs(5) }
    }

    /// Set custom timeout for health check
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Verify server is responsive and healthy
    pub fn verify(&mut self) -> Result<(), String> {
        // Check if process is alive
        if !self.server.is_alive() {
            return Err("LSP server process is not alive".to_string());
        }

        // Try a simple request to verify responsiveness
        let start = Instant::now();
        let test_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 999_999,
            "method": "workspace/symbol",
            "params": { "query": "__health_check__" }
        });

        use crate::common::{read_response_timeout, send_request_no_wait};
        send_request_no_wait(self.server, test_request);

        match read_response_timeout(self.server, self.timeout) {
            Some(_) => {
                let elapsed = start.elapsed();
                if elapsed > Duration::from_secs(2) {
                    eprintln!(
                        "⚠️  WARNING: Health check took {:?} - server may be under load",
                        elapsed
                    );
                }
                Ok(())
            }
            None => {
                Err(format!("Server did not respond to health check within {:?}", self.timeout))
            }
        }
    }
}

/// Resource usage tracker for tests
pub struct ResourceMonitor {
    start_time: Instant,
    operation: String,
}

impl ResourceMonitor {
    /// Start monitoring a test operation
    pub fn start(operation: impl Into<String>) -> Self {
        Self { start_time: Instant::now(), operation: operation.into() }
    }

    /// Record completion of the operation
    pub fn complete(self) {
        let elapsed = self.start_time.elapsed();
        if elapsed > Duration::from_secs(5) {
            eprintln!("⚠️  SLOW: {} took {:?}", self.operation, elapsed);
        }
    }

    /// Record completion with custom threshold
    pub fn complete_with_threshold(self, threshold: Duration) {
        let elapsed = self.start_time.elapsed();
        if elapsed > threshold {
            eprintln!(
                "⚠️  SLOW: {} took {:?} (threshold: {:?})",
                self.operation, elapsed, threshold
            );
        }
    }
}

/// Enhanced error reporting for test failures
pub struct TestError {
    pub context: String,
    pub error: String,
    pub environment: TestEnvironment,
}

impl TestError {
    /// Create a new test error with environment context
    pub fn new(context: impl Into<String>, error: impl Into<String>) -> Self {
        let environment = TestEnvironment::validate().unwrap_or_else(|e| {
            eprintln!("Failed to validate environment: {}", e);
            TestEnvironment {
                thread_count: 1,
                is_ci: false,
                is_containerized: false,
                is_wsl: false,
                available_memory_mb: None,
            }
        });

        Self { context: context.into(), error: error.into(), environment }
    }

    /// Format error with full diagnostic information
    pub fn format(&self) -> String {
        format!(
            "╔════════════════════════════════════════════════════════════════════╗\n\
             ║ TEST FAILURE                                                       ║\n\
             ╠════════════════════════════════════════════════════════════════════╣\n\
             ║ Context: {:<58} ║\n\
             ║ Error:   {:<58} ║\n\
             ╠════════════════════════════════════════════════════════════════════╣\n\
             ║ Environment:                                                       ║\n\
             ║   {:<64} ║\n\
             ╠════════════════════════════════════════════════════════════════════╣\n\
             ║ Suggestions:                                                       ║\n\
             {}\
             ╚════════════════════════════════════════════════════════════════════╝",
            truncate(&self.context, 58),
            truncate(&self.error, 58),
            self.environment.summary(),
            self.generate_suggestions()
        )
    }

    fn generate_suggestions(&self) -> String {
        let mut suggestions = Vec::new();

        if self.environment.is_constrained() {
            suggestions.push("║   • Try running with more threads: RUST_TEST_THREADS=4          ║");
        }

        if self.error.contains("timeout") {
            suggestions.push("║   • Increase timeout: LSP_TEST_TIMEOUT_MS=10000                 ║");
        }

        if self.environment.is_ci && self.error.contains("resource") {
            suggestions.push("║   • CI detected: Resource constraints may be the cause       ║");
        }

        if suggestions.is_empty() {
            suggestions.push("║   • Check server logs with: LSP_TEST_ECHO_STDERR=1           ║");
        }

        suggestions.join("\n") + "\n"
    }
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Truncate string to fit in formatted output
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len { s.to_string() } else { format!("{}...", &s[..max_len - 3]) }
}

/// Graceful degradation helper
pub struct GracefulDegradation {
    retry_count: usize,
    max_retries: usize,
    backoff_ms: u64,
}

impl GracefulDegradation {
    /// Create a new graceful degradation helper
    pub fn new(max_retries: usize) -> Self {
        Self { retry_count: 0, max_retries, backoff_ms: 100 }
    }

    /// Attempt an operation with graceful degradation
    pub fn attempt<T, E, F>(&mut self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: std::fmt::Debug,
    {
        loop {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) if self.retry_count < self.max_retries => {
                    self.retry_count += 1;
                    eprintln!(
                        "⚠️  Operation failed (attempt {}/{}), retrying after {}ms: {:?}",
                        self.retry_count,
                        self.max_retries + 1,
                        self.backoff_ms,
                        e
                    );
                    std::thread::sleep(Duration::from_millis(self.backoff_ms));
                    self.backoff_ms = (self.backoff_ms * 2).min(5000); // Cap at 5s
                }
                Err(e) => {
                    eprintln!("❌ Operation failed after {} attempts", self.retry_count + 1);
                    return Err(e);
                }
            }
        }
    }
}

/// Test stability helpers
pub mod stability {
    use std::time::Duration;

    /// Wait for a condition to become true with timeout
    pub fn wait_for_condition<F>(
        mut condition: F,
        timeout: Duration,
        check_interval: Duration,
    ) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if condition() {
                return Ok(());
            }
            std::thread::sleep(check_interval);
        }
        Err(format!("Condition not met within {:?}", timeout))
    }

    /// Retry an operation with exponential backoff
    pub fn retry_with_backoff<T, E, F>(
        mut operation: F,
        max_attempts: usize,
        initial_delay: Duration,
    ) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
    {
        let mut delay = initial_delay;
        for attempt in 0..max_attempts {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) if attempt == max_attempts - 1 => return Err(e),
                Err(_) => {
                    std::thread::sleep(delay);
                    delay = delay.saturating_mul(2);
                }
            }
        }
        // Safety: loop always returns in the match arms above
        unreachable!("retry_with_backoff should return in loop")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_validation() -> Result<(), String> {
        let env = TestEnvironment::validate()?;
        assert!(env.thread_count > 0);
        eprintln!("Test environment: {}", env.summary());
        Ok(())
    }

    #[test]
    fn test_timeout_multiplier_reasonable() {
        let env = TestEnvironment::validate().unwrap();
        let multiplier = env.timeout_multiplier();
        assert!(multiplier >= 1.0);
        assert!(multiplier <= 10.0);
    }

    #[test]
    fn test_resource_monitor_tracks_time() {
        let monitor = ResourceMonitor::start("test_operation");
        std::thread::sleep(Duration::from_millis(10));
        monitor.complete();
    }

    #[test]
    fn test_graceful_degradation_succeeds_eventually() {
        let mut degradation = GracefulDegradation::new(3);
        let mut attempt_count = 0;

        let result = degradation.attempt(|| {
            attempt_count += 1;
            if attempt_count < 2 { Err("simulated failure") } else { Ok(42) }
        });

        assert_eq!(result, Ok(42));
        assert_eq!(attempt_count, 2);
    }

    #[test]
    fn test_truncate_long_strings() {
        let long_str = "a".repeat(100);
        let truncated = truncate(&long_str, 10);
        assert_eq!(truncated.len(), 10);
        assert!(truncated.ends_with("..."));
    }
}
