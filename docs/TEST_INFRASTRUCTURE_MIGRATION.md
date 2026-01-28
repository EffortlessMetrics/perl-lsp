# Test Infrastructure Migration Guide

This guide helps migrate existing LSP tests to use the enhanced test infrastructure for improved reliability.

## Migration Checklist

- [ ] Add environment validation
- [ ] Replace hardcoded timeouts with profiles
- [ ] Add health checks
- [ ] Use resource monitoring
- [ ] Implement graceful degradation
- [ ] Enhance error reporting

## Pattern: Add Environment Validation

### Before

```rust
#[test]
fn test_feature() {
    let mut server = start_lsp_server();
    // test...
}
```

### After

```rust
#[test]
fn test_feature() -> Result<(), String> {
    let env = TestEnvironment::validate()?;
    eprintln!("Environment: {}", env.summary());

    let mut server = start_lsp_server();
    // test...
    Ok(())
}
```

## Pattern: Replace Hardcoded Timeouts

### Before

```rust
let response = read_response_timeout(&mut server, Duration::from_secs(5))?;
```

### After

```rust
use crate::common::timeout_scaler::TimeoutProfile;

let response = read_response_timeout(
    &mut server,
    TimeoutProfile::Standard.timeout()
)?;
```

## Pattern: Add Health Checks

### Before

```rust
let mut server = start_lsp_server();
let _init = initialize_lsp(&mut server);
// immediately start testing
```

### After

```rust
use crate::common::test_reliability::HealthCheck;

let mut server = start_lsp_server();
let _init = initialize_lsp(&mut server);

// Verify server is responsive
HealthCheck::new(&mut server)
    .with_timeout(TimeoutProfile::Standard.timeout())
    .verify()?;

// now test
```

## Pattern: Monitor Resource Usage

### Before

```rust
#[test]
fn test_expensive_operation() {
    expensive_operation();
}
```

### After

```rust
use crate::common::test_reliability::ResourceMonitor;

#[test]
fn test_expensive_operation() {
    let _monitor = ResourceMonitor::start("expensive_operation");
    expensive_operation();
    // Monitor automatically warns if operation is slow
}
```

## Pattern: Graceful Degradation

### Before

```rust
// Might fail on first attempt
let result = flaky_operation()?;
```

### After

```rust
use crate::common::test_reliability::GracefulDegradation;

let mut degradation = GracefulDegradation::new(3);
let result = degradation.attempt(|| flaky_operation())?;
```

## Pattern: Enhanced Error Reporting

### Before

```rust
let result = operation()
    .map_err(|e| format!("Operation failed: {}", e))?;
```

### After

```rust
use crate::common::test_reliability::TestError;

let result = operation().map_err(|e| {
    let error = TestError::new("operation_context", e);
    eprintln!("{}", error);
    "Operation failed".to_string()
})?;
```

## Complete Example Migration

### Before

```rust
#[test]
fn test_hover_feature() {
    let mut server = start_lsp_server();
    let _init = initialize_lsp(&mut server);

    std::thread::sleep(Duration::from_millis(500));

    let response = send_request(
        &mut server,
        json!({"method": "textDocument/hover", "params": {...}})
    );

    assert!(response.is_some());
    shutdown_and_exit(&mut server);
}
```

### After

```rust
use crate::common::test_reliability::{TestEnvironment, HealthCheck, ResourceMonitor};
use crate::common::timeout_scaler::TimeoutProfile;

#[test]
fn test_hover_feature() -> Result<(), String> {
    // 1. Validate environment
    let env = TestEnvironment::validate()?;
    eprintln!("Test environment: {}", env.summary());

    // 2. Monitor resources
    let _monitor = ResourceMonitor::start("hover_feature_test");

    // 3. Start server
    let mut server = start_lsp_server();
    let _init = initialize_lsp(&mut server);

    // 4. Health check (replaces arbitrary sleep)
    HealthCheck::new(&mut server)
        .with_timeout(TimeoutProfile::Standard.timeout())
        .verify()?;

    // 5. Test with appropriate timeout
    let response = send_request(
        &mut server,
        json!({"method": "textDocument/hover", "params": {...}}),
        TimeoutProfile::Standard.timeout()
    )?;

    // 6. Validate
    assert!(response.is_some(), "Should receive hover response");

    // 7. Cleanup
    shutdown_and_exit(&mut server);

    Ok(())
}
```

## Migration Priority

### High Priority (Do First)

1. **Flaky Tests** - Add graceful degradation and health checks
2. **Slow Tests** - Add resource monitoring to identify bottlenecks
3. **Initialization Tests** - Use `TimeoutProfile::Initialization`

### Medium Priority

4. **Standard Tests** - Add environment validation
5. **Timeout-Sensitive Tests** - Replace hardcoded timeouts

### Low Priority

6. **Stable Tests** - Add resource monitoring for metrics
7. **Error Handling Tests** - Enhance error reporting

## Common Migration Patterns

### Pattern: Replace `std::thread::sleep` with Health Checks

```rust
// Before: Arbitrary sleep hoping server is ready
std::thread::sleep(Duration::from_millis(500));

// After: Deterministic health check
HealthCheck::new(&mut server).verify()?;
```

### Pattern: Replace `panic!` with Result

```rust
// Before
#[test]
fn test() {
    let server = start_lsp_server();
    assert!(server.is_alive());
}

// After
#[test]
fn test() -> Result<(), String> {
    let env = TestEnvironment::validate()?;
    let server = start_lsp_server();
    Ok(())
}
```

### Pattern: Add Retry for Network Operations

```rust
// Before
let response = network_request()?;

// After
let mut degradation = GracefulDegradation::new(3);
let response = degradation.attempt(|| network_request())?;
```

## Validation Checklist

After migrating a test, verify:

- [ ] Test returns `Result<(), String>`
- [ ] Environment validated at start
- [ ] Health check after initialization
- [ ] Semantic timeout profiles used
- [ ] Resource monitoring for expensive operations
- [ ] Error handling with TestError
- [ ] Graceful degradation for flaky operations

## Testing the Migration

Run your migrated test with different thread counts to verify reliability:

```bash
# Test with single thread (most constrained)
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test your_test

# Test with dual thread (CI-like)
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test your_test

# Test with full parallelism
cargo test -p perl-lsp --test your_test
```

## Common Issues and Solutions

### Issue: Test Fails with "Environment validation failed"

**Solution:** The test environment doesn't meet minimum requirements. Check:

```rust
let env = TestEnvironment::validate()?;
if env.is_constrained() {
    eprintln!("⚠️  WARNING: Constrained environment detected");
    eprintln!("Timeout multiplier: {:.2}x", env.timeout_multiplier());
}
```

### Issue: Test Times Out in CI but Works Locally

**Solution:** Use adaptive timeouts that scale for CI:

```rust
// Automatically scales for CI
let timeout = TimeoutProfile::Standard.timeout();
```

### Issue: Health Check Fails

**Solution:** Add more time for server initialization:

```rust
// Give server more time to start
drain_until_quiet(&mut server,
    Duration::from_millis(100),
    Duration::from_secs(2));

// Then health check
HealthCheck::new(&mut server)
    .with_timeout(TimeoutProfile::Initialization.timeout())
    .verify()?;
```

### Issue: "Server did not respond to health check"

**Solution:** Check if server is actually running:

```rust
if !server.is_alive() {
    return Err("Server process died".to_string());
}
```

## Gradual Migration Strategy

You don't need to migrate everything at once. Follow this order:

1. **Week 1:** Add environment validation to all tests
2. **Week 2:** Replace hardcoded timeouts in flaky tests
3. **Week 3:** Add health checks to initialization sequences
4. **Week 4:** Add resource monitoring to identify bottlenecks
5. **Week 5:** Add graceful degradation to known flaky operations
6. **Week 6:** Enhance error reporting across test suite

## Tools and Commands

### Check Test Infrastructure Coverage

```bash
# Count tests using infrastructure
grep -r "TestEnvironment::validate" crates/perl-lsp/tests/ | wc -l

# Find tests with hardcoded timeouts
grep -r "Duration::from_secs\|Duration::from_millis" crates/perl-lsp/tests/ \
  | grep -v "TimeoutProfile" | wc -l
```

### Run Only Infrastructure Tests

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_test_infrastructure_validation
```

### Run with Debug Output

```bash
LSP_TEST_ECHO_STDERR=1 cargo test -p perl-lsp --test your_test -- --nocapture
```

## Questions and Support

- Review [LSP_TEST_INFRASTRUCTURE.md](./LSP_TEST_INFRASTRUCTURE.md) for detailed API documentation
- Check [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines
- Open an issue for questions or problems

## Success Metrics

After migration, you should see:

- ✅ Reduced test flakiness
- ✅ Better failure diagnostics
- ✅ Consistent behavior across environments
- ✅ Faster identification of slow tests
- ✅ More reliable CI runs
