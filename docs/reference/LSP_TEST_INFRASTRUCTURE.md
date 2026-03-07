# LSP Test Infrastructure Guide

This document describes the enhanced LSP test infrastructure for reliable, deterministic testing of the Perl LSP server.

## Overview

The LSP test infrastructure provides:

1. **Environment Validation** - Detect and adapt to different test environments
2. **Adaptive Timeouts** - Scale timeouts based on system resources
3. **Resource Monitoring** - Track test performance and detect slowdowns
4. **Health Checks** - Verify server responsiveness
5. **Graceful Degradation** - Handle transient failures with retry logic
6. **Enhanced Error Reporting** - Rich diagnostics for test failures

## Quick Start

### Basic Test Setup

```rust
use crate::common::{start_lsp_server, initialize_lsp, shutdown_and_exit};
use crate::common::test_reliability::{TestEnvironment, HealthCheck, ResourceMonitor};
use crate::common::timeout_scaler::TimeoutProfile;

#[test]
fn test_lsp_feature() -> Result<(), String> {
    // Validate environment
    let env = TestEnvironment::validate()?;
    eprintln!("Test environment: {}", env.summary());

    // Monitor resource usage
    let _monitor = ResourceMonitor::start("lsp_feature_test");

    // Start server
    let mut server = start_lsp_server();
    let _init_response = initialize_lsp(&mut server);

    // Verify server health
    HealthCheck::new(&mut server)
        .with_timeout(TimeoutProfile::Standard.timeout())
        .verify()?;

    // Run your test...

    shutdown_and_exit(&mut server);
    Ok(())
}
```

## Environment Validation

The `TestEnvironment` struct detects and provides information about the test environment:

```rust
let env = TestEnvironment::validate()?;

println!("Threads: {}", env.thread_count);
println!("CI: {}", env.is_ci);
println!("Containerized: {}", env.is_containerized);
println!("WSL: {}", env.is_wsl);
println!("Memory: {:?} MB", env.available_memory_mb);

// Check if environment is constrained
if env.is_constrained() {
    eprintln!("⚠️  WARNING: Running in constrained environment");
}

// Get recommended timeout multiplier
let multiplier = env.timeout_multiplier();
let adjusted_timeout = base_timeout * multiplier as u32;
```

## Adaptive Timeouts

### Timeout Profiles

Use semantic timeout profiles instead of hardcoded durations:

```rust
use crate::common::timeout_scaler::TimeoutProfile;

// Standard operations (hover, completion)
let timeout = TimeoutProfile::Standard.timeout();

// Initialization (slower)
let init_timeout = TimeoutProfile::Initialization.timeout();

// Performance tests (stricter)
let perf_timeout = TimeoutProfile::Performance.timeout();

// Stress tests (more lenient)
let stress_timeout = TimeoutProfile::Stress.timeout();

// Quick checks
let quick_timeout = TimeoutProfile::Quick.timeout();

// Cross-file operations
let cross_file_timeout = TimeoutProfile::CrossFile.timeout();
```

### Manual Timeout Scaling

For custom timeout scaling:

```rust
use crate::common::timeout_scaler::{get_adaptive_timeout, get_scaled_timeout};

// Get base adaptive timeout
let base = get_adaptive_timeout();

// Scale by factor
let doubled = get_scaled_timeout(2.0);
let halved = get_scaled_timeout(0.5);
```

### Thread-Aware Scaling

The infrastructure automatically detects `RUST_TEST_THREADS` and scales timeouts:

| Thread Count | Base Timeout | CI Multiplier | Final Range    |
|--------------|--------------|---------------|----------------|
| 1-2          | 15s          | 1.5x          | 15s - 22.5s    |
| 3-4          | 10s          | 1.5x          | 10s - 15s      |
| 5-8          | 7.5s         | 1.5x          | 7.5s - 11.25s  |
| 9+           | 5s           | 1.5x          | 5s - 7.5s      |

## Resource Monitoring

Track test performance to identify slow tests:

```rust
use crate::common::test_reliability::ResourceMonitor;

let monitor = ResourceMonitor::start("expensive_operation");

// Do work...

monitor.complete(); // Warns if > 5s

// Or with custom threshold
monitor.complete_with_threshold(Duration::from_secs(1));
```

Output:
```
⚠️  SLOW: expensive_operation took 6.2s
```

## Health Checks

Verify server is responsive before running tests:

```rust
use crate::common::test_reliability::HealthCheck;

let mut server = start_lsp_server();
initialize_lsp(&mut server);

// Basic health check
HealthCheck::new(&mut server).verify()?;

// With custom timeout
HealthCheck::new(&mut server)
    .with_timeout(Duration::from_secs(3))
    .verify()?;
```

## Graceful Degradation

Handle transient failures with automatic retry:

```rust
use crate::common::test_reliability::GracefulDegradation;

let mut degradation = GracefulDegradation::new(3); // max 3 retries

let result = degradation.attempt(|| {
    // Try operation
    risky_operation()
})?;
```

Output on transient failure:
```
⚠️  Operation failed (attempt 1/4), retrying after 100ms: connection refused
⚠️  Operation failed (attempt 2/4), retrying after 200ms: connection refused
```

## Enhanced Error Reporting

Get rich diagnostics for test failures:

```rust
use crate::common::test_reliability::TestError;

let error = TestError::new(
    "LSP initialization",
    "Timeout waiting for response"
);

eprintln!("{}", error);
```

Output:
```
╔════════════════════════════════════════════════════════════════════╗
║ TEST FAILURE                                                       ║
╠════════════════════════════════════════════════════════════════════╣
║ Context: LSP initialization                                        ║
║ Error:   Timeout waiting for response                             ║
╠════════════════════════════════════════════════════════════════════╣
║ Environment:                                                       ║
║   threads=2 CI=true container=false WSL=false mem=4096MB          ║
╠════════════════════════════════════════════════════════════════════╣
║ Suggestions:                                                       ║
║   • Try running with more threads: RUST_TEST_THREADS=4            ║
║   • Increase timeout: LSP_TEST_TIMEOUT_MS=10000                   ║
║   • CI detected: Resource constraints may be the cause            ║
╚════════════════════════════════════════════════════════════════════╝
```

## Stability Helpers

### Wait for Condition

Poll until a condition becomes true:

```rust
use crate::common::test_reliability::stability::wait_for_condition;

wait_for_condition(
    || server.is_ready(),
    Duration::from_secs(5),
    Duration::from_millis(100)
)?;
```

### Retry with Backoff

Retry with exponential backoff:

```rust
use crate::common::test_reliability::stability::retry_with_backoff;

let result = retry_with_backoff(
    || attempt_connection(),
    5, // max attempts
    Duration::from_millis(100) // initial delay
)?;
```

## Environment Variables

### Test Configuration

- `RUST_TEST_THREADS` - Limit test parallelism (e.g., `2` for constrained CI)
- `LSP_TEST_TIMEOUT_MS` - Override default timeout (milliseconds)
- `LSP_TEST_SHORT_MS` - Override short timeout for quick checks
- `LSP_TEST_ECHO_STDERR` - Echo LSP server stderr to test output
- `LSP_TEST_DEBUG_READER` - Debug reader thread for troubleshooting
- `PERL_LSP_BIN` - Explicit path to `perl-lsp` binary

### CI Detection

The infrastructure automatically detects:
- `CI`
- `GITHUB_ACTIONS`
- `CONTINUOUS_INTEGRATION`
- `JENKINS_URL`
- `GITLAB_CI`
- `CIRCLECI`
- `TRAVIS`

### Container Detection

Detects containerized environments via:
- `DOCKER_CONTAINER`
- `/.dockerenv` file
- `KUBERNETES_SERVICE_HOST`

### WSL Detection

Detects Windows Subsystem for Linux via:
- `WSL_DISTRO_NAME`
- `WSLENV`

## Running Tests

### Local Development

```bash
# Standard test run
cargo test -p perl-lsp

# With adaptive threading (recommended)
RUST_TEST_THREADS=2 cargo test -p perl-lsp

# With verbose output
LSP_TEST_ECHO_STDERR=1 cargo test -p perl-lsp -- --nocapture
```

### CI Environment

```bash
# CI-optimized test run
RUST_TEST_THREADS=2 cargo test -p perl-lsp --lib

# With extended timeout
LSP_TEST_TIMEOUT_MS=15000 RUST_TEST_THREADS=2 cargo test -p perl-lsp
```

### Resource-Constrained Systems

```bash
# Minimal resource usage
RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 cargo test -p perl-lsp
```

## Best Practices

### 1. Always Validate Environment

Start tests with environment validation:

```rust
#[test]
fn my_test() -> Result<(), String> {
    let env = TestEnvironment::validate()?;
    eprintln!("Environment: {}", env.summary());
    // ...
}
```

### 2. Use Semantic Timeout Profiles

Prefer `TimeoutProfile` over hardcoded durations:

```rust
// ✅ Good
let timeout = TimeoutProfile::Initialization.timeout();

// ❌ Bad
let timeout = Duration::from_secs(10);
```

### 3. Monitor Expensive Operations

Track performance of slow operations:

```rust
let _monitor = ResourceMonitor::start("workspace_indexing");
// expensive operation
```

### 4. Verify Server Health

Always check server health after initialization:

```rust
initialize_lsp(&mut server);
HealthCheck::new(&mut server).verify()?;
```

### 5. Handle Transient Failures

Use graceful degradation for flaky operations:

```rust
let mut degradation = GracefulDegradation::new(2);
let result = degradation.attempt(|| network_request())?;
```

### 6. Provide Rich Error Context

Use `TestError` for detailed diagnostics:

```rust
operation().map_err(|e| {
    let error = TestError::new("operation_context", e);
    eprintln!("{}", error);
    "operation failed".to_string()
})?;
```

## Troubleshooting

### Tests Timing Out

1. Check environment: `let env = TestEnvironment::validate()?;`
2. Increase threads: `RUST_TEST_THREADS=4 cargo test`
3. Increase timeout: `LSP_TEST_TIMEOUT_MS=15000 cargo test`
4. Enable diagnostics: `LSP_TEST_ECHO_STDERR=1 cargo test -- --nocapture`

### Flaky Tests

1. Use health checks: `HealthCheck::new(&mut server).verify()?`
2. Add graceful degradation: `GracefulDegradation::new(3)`
3. Wait for conditions: `wait_for_condition(|| ready(), timeout, interval)`
4. Increase test isolation: `RUST_TEST_THREADS=1 cargo test`

### Resource Constraints

1. Detect constraints: `if env.is_constrained() { ... }`
2. Scale timeouts: `env.timeout_multiplier()`
3. Add delays: `std::thread::sleep(Duration::from_millis(100))`
4. Reduce parallelism: `RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=1`

## Example: Complete Test

```rust
#[test]
fn test_comprehensive_lsp_feature() -> Result<(), String> {
    // 1. Validate environment
    let env = TestEnvironment::validate()?;
    eprintln!("Test environment: {}", env.summary());

    // 2. Monitor resources
    let _monitor = ResourceMonitor::start("comprehensive_test");

    // 3. Start server with retry
    let mut server = {
        let mut degradation = GracefulDegradation::new(2);
        degradation.attempt(|| Ok(start_lsp_server()))?
    };

    // 4. Initialize with adaptive timeout
    let _init = initialize_lsp(&mut server);

    // 5. Health check
    HealthCheck::new(&mut server)
        .with_timeout(TimeoutProfile::Standard.timeout())
        .verify()
        .map_err(|e| {
            let err = TestError::new("health_check", e);
            eprintln!("{}", err);
            "Health check failed".to_string()
        })?;

    // 6. Test feature with appropriate timeout
    let result = send_request(
        &mut server,
        request_params,
        TimeoutProfile::Standard.timeout()
    )?;

    // 7. Validate result
    assert_eq!(result.status, "success");

    // 8. Cleanup
    shutdown_and_exit(&mut server);

    Ok(())
}
```

## Related Documentation

- [LSP Implementation Guide](./LSP_IMPLEMENTATION_GUIDE.md) - Server architecture
- [Commands Reference](./COMMANDS_REFERENCE.md) - Test commands
- [Debt Tracking](./DEBT_TRACKING.md) - Known test issues
- [Current Status](./CURRENT_STATUS.md) - Test metrics

## Contributing

When adding new LSP tests:

1. Use the test infrastructure utilities
2. Validate environment at test start
3. Use semantic timeout profiles
4. Add health checks after initialization
5. Monitor expensive operations
6. Handle transient failures gracefully
7. Provide rich error context

For questions or improvements, see [CONTRIBUTING.md](../CONTRIBUTING.md).
