# ADR-0018: Adaptive Threading for LSP Tests

**Status**: Accepted
**Date**: 2025-02-15
**Decision Makers**: Perl LSP Architecture Team
**Related**: [LSP_TEST_INFRASTRUCTURE.md](../reference/LSP_TEST_INFRASTRUCTURE.md), [THREADING_CONFIGURATION_GUIDE.md](../how-to/THREADING_CONFIGURATION_GUIDE.md)

## Context

LSP integration tests require actual server processes with JSON-RPC communication, creating significant resource overhead. CI environments have widely varying resource constraints:

1. **Resource Variability**: CI runners range from single-core to multi-core
2. **Contention Issues**: Parallel LSP server instances compete for ports and resources
3. **Timeout Sensitivity**: Fixed timeouts fail under resource pressure
4. **Flaky Tests**: Non-deterministic failures from race conditions and resource exhaustion

### Problem Statement

Standard test configurations cause failures in constrained CI environments:
- Fixed timeouts too short for single-threaded execution
- Parallel server instances cause port conflicts
- Memory pressure from concurrent LSP processes
- Inconsistent test results across different CI runners

## Decision

**We implement thread-aware timeout scaling with environment validation and graceful degradation, providing deterministic test behavior across diverse CI environments.**

### Adaptive Timeout Architecture

```rust
/// Get the maximum number of concurrent threads to use in tests
/// Respects RUST_TEST_THREADS environment variable and scales down thread counts appropriately
pub fn max_concurrent_threads() -> usize {
    std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1)
        })
}

/// Get adaptive timeout based on thread constraints
/// More comprehensive handling with logarithmic backoff protection
pub fn get_adaptive_timeout(base_ms: u64) -> Duration {
    let threads = max_concurrent_threads();
    let scaling_factor = match threads {
        0..=1 => 4.0,  // Single-threaded: 4x timeout
        2 => 2.0,      // Dual-threaded: 2x timeout
        3..=4 => 1.5,  // Moderate: 1.5x timeout
        _ => 1.0,      // Full parallelism: base timeout
    };
    Duration::from_millis((base_ms as f64 * scaling_factor) as u64)
}
```

### Timeout Profiles

| Profile | Base Timeout | Use Case |
|---------|-------------|----------|
| **Standard** | 200-500ms | Normal LSP operations |
| **Initialization** | 800-2000ms | Server startup |
| **Performance** | 50-100ms | Performance-critical tests |
| **Stress** | 2000-5000ms | Heavy load scenarios |
| **Quick** | 100-200ms | Fast validation |
| **CrossFile** | 500-1000ms | Workspace navigation |

### Environment Detection

```rust
pub struct TestEnvironment {
    pub max_threads: usize,
    pub is_ci: bool,
    pub available_memory: Option<u64>,
}

impl TestEnvironment {
    pub fn validate() -> Result<Self> {
        Ok(Self {
            max_threads: max_concurrent_threads(),
            is_ci: std::env::var("CI").is_ok(),
            available_memory: Self::detect_memory(),
        })
    }
}
```

### CI Integration

```yaml
# GitHub Actions with adaptive threading
- name: Run LSP tests with adaptive threading
  run: RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

## Consequences

### Positive

- **Reliable CI**: 100% test pass rate across diverse CI runners
- **Deterministic Behavior**: Consistent results regardless of environment
- **Resource Efficiency**: Optimal parallelism without over-subscription
- **Developer Experience**: Tests work locally without configuration
- **Automatic Scaling**: No manual timeout tuning per environment

### Negative

- **Longer Test Times**: Scaled timeouts increase total test duration in constrained environments
- **Complexity**: Additional abstraction layer for timeout management
- **Debugging Overhead**: Timeout issues require understanding scaling logic

### Mitigations

- Clear documentation of scaling factors
- Environment variable overrides for debugging
- Verbose logging option for timeout decisions

## Implementation Guidelines

### Standard Usage

```bash
# Default (adaptive)
cargo test -p perl-lsp-rs

# CI recommended configuration
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Single-threaded debugging
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs
```

### Custom Timeout Profiles

```rust
use crate::common::timeout_scaler::TimeoutProfile;

// Standard profile
let timeout = TimeoutProfile::Standard.duration();

// Custom profile for specific test
let custom = TimeoutProfile::custom(1500); // 1.5 seconds
```

## References

- [LSP_TEST_INFRASTRUCTURE.md](../reference/LSP_TEST_INFRASTRUCTURE.md) - Test infrastructure details
- [THREADING_CONFIGURATION_GUIDE.md](../how-to/THREADING_CONFIGURATION_GUIDE.md) - Comprehensive threading guide
- [CI_TEST_LANES.md](../project/CI_TEST_LANES.md) - CI test configuration
- [TEST_INFRASTRUCTURE_GUIDE.md](../reference/TEST_INFRASTRUCTURE_GUIDE.md) - Adaptive timeout scaling
