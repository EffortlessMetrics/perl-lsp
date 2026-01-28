//! Adaptive timeout helper for LSP tests that scales based on available threads.
//!
//! This module provides thread-aware timeout scaling to handle CI environments
//! with varying parallelism constraints. The timeouts automatically adjust based on:
//!
//! - Available system parallelism (`std::thread::available_parallelism`)
//! - RUST_TEST_THREADS environment variable
//! - CI environment detection (GITHUB_ACTIONS, CI, etc.)
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::common::timeout_scaler::{get_adaptive_timeout, get_scaled_timeout, TimeoutProfile};
//!
//! // Get the default adaptive timeout
//! let timeout = get_adaptive_timeout();
//!
//! // Get a timeout scaled by a factor (e.g., 2x for initialization)
//! let init_timeout = get_scaled_timeout(2.0);
//!
//! // Use a specific profile for different test types
//! let perf_timeout = TimeoutProfile::Performance.timeout();
//! let stress_timeout = TimeoutProfile::Stress.timeout();
//! ```
//!
//! # Thread Scaling Strategy
//!
//! | Thread Count | Base Timeout | Rationale                          |
//! |--------------|--------------|-------------------------------------|
//! | 1-2          | 15s          | Heavily constrained (single/dual)  |
//! | 3-4          | 10s          | Moderately constrained             |
//! | 5-8          | 7.5s         | Lightly constrained                |
//! | 9+           | 5s           | Unconstrained (many cores)         |
//!
//! # CI Detection
//!
//! When running in CI environments (detected via environment variables),
//! an additional 1.5x multiplier is applied to account for shared resources
//! and unpredictable load.

#![allow(dead_code)] // These are utilities, not all may be used by every test file

use std::time::Duration;

/// Get the effective thread count for timeout scaling.
///
/// This considers:
/// 1. RUST_TEST_THREADS environment variable (explicit limit)
/// 2. System available parallelism (hardware capability)
///
/// Returns at least 1 to avoid division issues.
pub fn effective_thread_count() -> usize {
    std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8))
        .max(1)
}

/// Check if we're running in a CI environment.
///
/// Detects common CI systems via environment variables.
pub fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("TRAVIS").is_ok()
}

/// Get adaptive timeout based on available parallelism.
///
/// This is the primary function for obtaining test timeouts. It automatically
/// scales based on thread count and CI environment detection.
///
/// # Returns
///
/// A `Duration` appropriate for the current execution environment:
/// - 15 seconds for 1-2 threads (heavily constrained)
/// - 10 seconds for 3-4 threads (moderately constrained)
/// - 7.5 seconds for 5-8 threads (lightly constrained)
/// - 5 seconds for 9+ threads (unconstrained)
///
/// These values are further increased by 1.5x in CI environments.
///
/// # Example
///
/// ```rust,ignore
/// let timeout = get_adaptive_timeout();
/// let response = server.rx.recv_timeout(timeout);
/// ```
pub fn get_adaptive_timeout() -> Duration {
    let base = match effective_thread_count() {
        1..=2 => Duration::from_secs(15),
        3..=4 => Duration::from_secs(10),
        5..=8 => Duration::from_millis(7500),
        _ => Duration::from_secs(5),
    };

    // Apply CI multiplier if running in CI
    if is_ci_environment() { Duration::from_secs_f64(base.as_secs_f64() * 1.5) } else { base }
}

/// Get timeout scaled by a factor.
///
/// Useful for operations that need more or less time than the baseline.
///
/// # Arguments
///
/// * `factor` - Multiplier for the base timeout (e.g., 2.0 for 2x, 0.5 for half)
///
/// # Returns
///
/// The adaptive timeout multiplied by the given factor.
///
/// # Example
///
/// ```rust,ignore
/// // Initialization typically needs 2-3x the normal timeout
/// let init_timeout = get_scaled_timeout(2.0);
///
/// // Quick checks can use a reduced timeout
/// let quick_timeout = get_scaled_timeout(0.25);
/// ```
pub fn get_scaled_timeout(factor: f64) -> Duration {
    let base = get_adaptive_timeout();
    Duration::from_secs_f64(base.as_secs_f64() * factor)
}

/// Get a short timeout for expected non-responses or quick operations.
///
/// This is useful for tests that verify timeout behavior or check for
/// expected failures.
///
/// # Returns
///
/// A short timeout scaled by thread count:
/// - 1500ms for 1-2 threads
/// - 1000ms for 3-4 threads
/// - 750ms for 5-8 threads
/// - 500ms for 9+ threads
pub fn get_short_timeout() -> Duration {
    let base = match effective_thread_count() {
        1..=2 => Duration::from_millis(1500),
        3..=4 => Duration::from_millis(1000),
        5..=8 => Duration::from_millis(750),
        _ => Duration::from_millis(500),
    };

    if is_ci_environment() { Duration::from_secs_f64(base.as_secs_f64() * 1.5) } else { base }
}

/// Get an initialization timeout suitable for LSP server startup.
///
/// Initialization is typically the slowest operation and needs extra buffer
/// for first-time compilation, workspace indexing, etc.
///
/// # Returns
///
/// A timeout 3x the base adaptive timeout, with additional CI scaling.
pub fn get_init_timeout() -> Duration {
    get_scaled_timeout(3.0)
}

/// Get a quick drain timeout for waiting for server to settle.
///
/// Used for brief pauses while waiting for the server to process notifications.
pub fn get_drain_timeout() -> Duration {
    match effective_thread_count() {
        1..=2 => Duration::from_millis(200),
        3..=4 => Duration::from_millis(150),
        5..=8 => Duration::from_millis(100),
        _ => Duration::from_millis(50),
    }
}

/// Predefined timeout profiles for different test scenarios.
///
/// These profiles provide semantic names for common timeout patterns,
/// making test code more readable and maintainable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutProfile {
    /// Standard operations (hover, completion, etc.)
    Standard,
    /// Initialization and workspace indexing
    Initialization,
    /// Performance benchmarks (stricter timeouts)
    Performance,
    /// Stress tests and heavy workloads
    Stress,
    /// Quick checks and expected failures
    Quick,
    /// Cross-file operations (definition, references)
    CrossFile,
}

impl TimeoutProfile {
    /// Get the timeout for this profile.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let timeout = TimeoutProfile::Initialization.timeout();
    /// let response = server.initialize_with_timeout(timeout);
    /// ```
    pub fn timeout(self) -> Duration {
        match self {
            Self::Standard => get_adaptive_timeout(),
            Self::Initialization => get_init_timeout(),
            Self::Performance => get_scaled_timeout(0.5), // Stricter for perf tests
            Self::Stress => get_scaled_timeout(4.0),      // More lenient for stress
            Self::Quick => get_short_timeout(),
            Self::CrossFile => get_scaled_timeout(1.5), // Cross-file ops need extra time
        }
    }

    /// Get the timeout in milliseconds (for APIs that need u64).
    pub fn timeout_ms(self) -> u64 {
        self.timeout().as_millis() as u64
    }
}

/// Configuration for adaptive retry behavior.
///
/// Used when operations might need multiple attempts with increasing timeouts.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Initial timeout for the first attempt
    pub initial_timeout: Duration,
    /// Multiplier applied to timeout after each failure
    pub backoff_factor: f64,
    /// Maximum timeout (caps the exponential growth)
    pub max_timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_timeout: get_adaptive_timeout(),
            backoff_factor: 1.5,
            max_timeout: get_scaled_timeout(4.0),
        }
    }
}

impl RetryConfig {
    /// Create a config optimized for CI environments.
    pub fn for_ci() -> Self {
        Self {
            max_retries: 4,
            initial_timeout: get_scaled_timeout(1.5),
            backoff_factor: 2.0,
            max_timeout: get_scaled_timeout(6.0),
        }
    }

    /// Create a config for performance-sensitive tests.
    pub fn for_performance() -> Self {
        Self {
            max_retries: 2,
            initial_timeout: get_scaled_timeout(0.5),
            backoff_factor: 1.2,
            max_timeout: get_adaptive_timeout(),
        }
    }

    /// Get the timeout for a specific attempt number (0-indexed).
    pub fn timeout_for_attempt(&self, attempt: usize) -> Duration {
        let multiplier = self.backoff_factor.powi(attempt as i32);
        let timeout = Duration::from_secs_f64(self.initial_timeout.as_secs_f64() * multiplier);
        timeout.min(self.max_timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_thread_count_returns_at_least_one() {
        assert!(effective_thread_count() >= 1);
    }

    #[test]
    fn test_adaptive_timeout_returns_reasonable_duration() {
        let timeout = get_adaptive_timeout();
        assert!(timeout >= Duration::from_secs(5));
        assert!(timeout <= Duration::from_secs(30)); // Even with CI multiplier
    }

    #[test]
    fn test_scaled_timeout_applies_factor() {
        let base = get_adaptive_timeout();
        let doubled = get_scaled_timeout(2.0);
        let halved = get_scaled_timeout(0.5);

        // Allow for floating-point imprecision
        assert!((doubled.as_secs_f64() - base.as_secs_f64() * 2.0).abs() < 0.01);
        assert!((halved.as_secs_f64() - base.as_secs_f64() * 0.5).abs() < 0.01);
    }

    #[test]
    fn test_short_timeout_is_shorter_than_adaptive() {
        let short = get_short_timeout();
        let adaptive = get_adaptive_timeout();
        assert!(short < adaptive);
    }

    #[test]
    fn test_init_timeout_is_longer_than_adaptive() {
        let init = get_init_timeout();
        let adaptive = get_adaptive_timeout();
        assert!(init > adaptive);
    }

    #[test]
    fn test_timeout_profiles_have_correct_ordering() {
        let quick = TimeoutProfile::Quick.timeout();
        let perf = TimeoutProfile::Performance.timeout();
        let standard = TimeoutProfile::Standard.timeout();
        let init = TimeoutProfile::Initialization.timeout();
        let stress = TimeoutProfile::Stress.timeout();

        // Quick should be shortest, stress should be longest
        assert!(quick <= perf);
        assert!(standard <= init);
        assert!(init <= stress);
    }

    #[test]
    fn test_retry_config_exponential_backoff() {
        let config = RetryConfig::default();

        let t0 = config.timeout_for_attempt(0);
        let t1 = config.timeout_for_attempt(1);
        let t2 = config.timeout_for_attempt(2);

        // Each attempt should be longer than the previous
        assert!(t1 > t0);
        assert!(t2 > t1);

        // But capped at max_timeout
        let t10 = config.timeout_for_attempt(10);
        assert!(t10 <= config.max_timeout);
    }
}
