//! LSP Test Infrastructure Validation Tests
//!
//! This test file demonstrates and validates the enhanced test infrastructure
//! for LSP tests, including:
//! - Environment validation
//! - Resource monitoring
//! - Health checks
//! - Graceful degradation
//! - Enhanced error reporting

mod common;

use common::test_reliability::{
    GracefulDegradation, HealthCheck, ResourceMonitor, TestEnvironment, TestError,
};
use common::timeout_scaler::{TimeoutProfile, get_adaptive_timeout, is_ci_environment};
use common::{drain_until_quiet, initialize_lsp, shutdown_and_exit, start_lsp_server};

/// Test that environment validation works correctly
#[test]
fn test_environment_validation() -> Result<(), String> {
    let env = TestEnvironment::validate()?;

    // Log environment for debugging
    eprintln!("╔════════════════════════════════════════════════════════════════════╗");
    eprintln!("║ Test Environment Validation                                        ║");
    eprintln!("╠════════════════════════════════════════════════════════════════════╣");
    eprintln!("║ {:<66} ║", env.summary());
    eprintln!("║ Constrained: {:<54} ║", env.is_constrained());
    eprintln!("║ Timeout multiplier: {:<47} ║", format!("{:.2}x", env.timeout_multiplier()));
    eprintln!("╚════════════════════════════════════════════════════════════════════╝");

    // Validate basic properties
    assert!(env.thread_count > 0, "Thread count should be positive");

    Ok(())
}

/// Test that health checks can verify server responsiveness
#[test]
fn test_health_check_server_responsiveness() -> Result<(), String> {
    let _env = TestEnvironment::validate()?;
    let _monitor = ResourceMonitor::start("health_check_test");

    let mut server = start_lsp_server();
    let _init_response = initialize_lsp(&mut server);

    // Give server time to fully initialize
    drain_until_quiet(
        &mut server,
        std::time::Duration::from_millis(100),
        std::time::Duration::from_secs(1),
    );

    // Perform health check
    let health_result =
        HealthCheck::new(&mut server).with_timeout(TimeoutProfile::Standard.timeout()).verify();

    shutdown_and_exit(&mut server);

    match health_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let error = TestError::new("Health check", e);
            eprintln!("{}", error);
            Err("Health check failed".to_string())
        }
    }
}

/// Test that resource monitoring tracks operation timing
#[test]
fn test_resource_monitoring() {
    let env = TestEnvironment::validate().unwrap();
    let monitor = ResourceMonitor::start("resource_monitoring_test");

    // Simulate some work
    std::thread::sleep(std::time::Duration::from_millis(50));

    eprintln!("Test completed in environment: {}", env.summary());
    monitor.complete();
}

/// Test that graceful degradation handles transient failures
#[test]
fn test_graceful_degradation_retry_logic() {
    let mut attempt_count = 0;
    let mut degradation = GracefulDegradation::new(3);

    let result: Result<i32, &str> = degradation.attempt(|| {
        attempt_count += 1;
        if attempt_count < 2 { Err("simulated transient failure") } else { Ok(42) }
    });

    assert_eq!(result, Ok(42), "Should succeed after retry");
    assert!(attempt_count >= 2, "Should have retried at least once");
}

/// Test that timeout profiles provide appropriate durations
#[test]
fn test_timeout_profiles_are_appropriate() {
    let standard = TimeoutProfile::Standard.timeout();
    let initialization = TimeoutProfile::Initialization.timeout();
    let performance = TimeoutProfile::Performance.timeout();
    let stress = TimeoutProfile::Stress.timeout();
    let quick = TimeoutProfile::Quick.timeout();
    let cross_file = TimeoutProfile::CrossFile.timeout();

    eprintln!("╔════════════════════════════════════════════════════════════════════╗");
    eprintln!("║ Timeout Profile Analysis                                          ║");
    eprintln!("╠════════════════════════════════════════════════════════════════════╣");
    eprintln!(
        "║ Standard:       {:>7.2}s                                       ║",
        standard.as_secs_f64()
    );
    eprintln!(
        "║ Initialization: {:>7.2}s                                       ║",
        initialization.as_secs_f64()
    );
    eprintln!(
        "║ Performance:    {:>7.2}s                                       ║",
        performance.as_secs_f64()
    );
    eprintln!(
        "║ Stress:         {:>7.2}s                                       ║",
        stress.as_secs_f64()
    );
    eprintln!(
        "║ Quick:          {:>7.2}s                                       ║",
        quick.as_secs_f64()
    );
    eprintln!(
        "║ CrossFile:      {:>7.2}s                                       ║",
        cross_file.as_secs_f64()
    );
    eprintln!("╚════════════════════════════════════════════════════════════════════╝");

    // Verify ordering
    assert!(quick <= performance, "Quick should be <= performance");
    assert!(performance <= standard, "Performance should be <= standard");
    assert!(standard <= initialization, "Standard should be <= initialization");
    assert!(initialization <= stress, "Initialization should be <= stress");

    // All timeouts should be reasonable
    assert!(
        standard >= std::time::Duration::from_secs(1),
        "Standard timeout should be at least 1s"
    );
    assert!(stress <= std::time::Duration::from_secs(120), "Stress timeout should be at most 120s");
}

/// Test that CI environment detection works
#[test]
fn test_ci_environment_detection() {
    let is_ci = is_ci_environment();
    let env = TestEnvironment::validate().unwrap();

    eprintln!("CI detected: {}", is_ci);
    eprintln!("CI from environment: {}", env.is_ci);

    // Both methods should agree
    assert_eq!(is_ci, env.is_ci, "CI detection should be consistent");
}

/// Test that adaptive timeouts scale appropriately
#[test]
fn test_adaptive_timeout_scaling() {
    let timeout = get_adaptive_timeout();
    let env = TestEnvironment::validate().unwrap();

    eprintln!("Adaptive timeout: {:?}", timeout);
    eprintln!("Environment: {}", env.summary());

    // Timeout should be reasonable
    assert!(timeout >= std::time::Duration::from_secs(2), "Timeout should be at least 2s");
    assert!(timeout <= std::time::Duration::from_secs(60), "Timeout should be at most 60s");

    // In CI, timeout should be longer
    if env.is_ci {
        assert!(timeout >= std::time::Duration::from_secs(5), "CI timeout should be at least 5s");
    }
}

/// Test error formatting with context
#[test]
fn test_error_formatting() {
    let error =
        TestError::new("LSP server initialization", "Timeout waiting for initialize response");

    let formatted = error.format();
    eprintln!("{}", formatted);

    // Verify error contains key information
    assert!(formatted.contains("TEST FAILURE"));
    assert!(formatted.contains("Context:"));
    assert!(formatted.contains("Error:"));
    assert!(formatted.contains("Environment:"));
    assert!(formatted.contains("Suggestions:"));
}

/// Integration test: Full LSP lifecycle with infrastructure support
#[test]
fn test_lsp_lifecycle_with_infrastructure_support() -> Result<(), String> {
    // Validate environment first
    let env = TestEnvironment::validate()?;
    eprintln!("Running in environment: {}", env.summary());

    let _monitor = ResourceMonitor::start("lsp_lifecycle_test");

    // Start server with graceful degradation
    let mut server = {
        let mut degradation = GracefulDegradation::new(2);
        degradation.attempt(|| Ok::<_, String>(start_lsp_server()))?
    };

    // Initialize with adaptive timeout
    let init_timeout = TimeoutProfile::Initialization.timeout();
    eprintln!("Using initialization timeout: {:?}", init_timeout);

    let _init_response = initialize_lsp(&mut server);

    // Wait for server to settle
    drain_until_quiet(
        &mut server,
        std::time::Duration::from_millis(100),
        std::time::Duration::from_secs(1),
    );

    // Perform health check
    HealthCheck::new(&mut server)
        .with_timeout(TimeoutProfile::Standard.timeout())
        .verify()
        .map_err(|e| {
            let error = TestError::new("LSP lifecycle health check", e);
            eprintln!("{}", error);
            "Health check failed".to_string()
        })?;

    shutdown_and_exit(&mut server);

    Ok(())
}

/// Test that server startup is deterministic and reliable
#[test]
fn test_server_startup_reliability() -> Result<(), String> {
    let env = TestEnvironment::validate()?;
    let _monitor = ResourceMonitor::start("server_startup_reliability");

    // Try starting server multiple times to verify consistency
    for i in 0..3 {
        eprintln!("Server startup attempt {}/3", i + 1);

        let mut server = start_lsp_server();
        let _init_response = initialize_lsp(&mut server);

        // Verify server is responsive
        HealthCheck::new(&mut server)
            .with_timeout(TimeoutProfile::Standard.timeout())
            .verify()
            .map_err(|e| {
                let error = TestError::new(format!("Startup attempt {}", i + 1), e);
                eprintln!("{}", error);
                format!("Server startup {} failed", i + 1)
            })?;

        shutdown_and_exit(&mut server);

        // Brief delay between iterations in constrained environments
        if env.is_constrained() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    Ok(())
}

/// Test stability helpers work correctly
#[test]
fn test_stability_helpers() {
    use common::test_reliability::stability::{retry_with_backoff, wait_for_condition};

    // Test wait_for_condition
    let start = std::time::Instant::now();
    let mut counter = 0;

    let result = wait_for_condition(
        || {
            counter += 1;
            counter >= 3
        },
        std::time::Duration::from_secs(2),
        std::time::Duration::from_millis(50),
    );

    assert!(result.is_ok(), "Condition should be met");
    assert!(counter >= 3, "Counter should have been incremented");
    assert!(start.elapsed() < std::time::Duration::from_secs(2), "Should complete quickly");

    // Test retry_with_backoff
    let mut attempt = 0;
    let result: Result<i32, &str> = retry_with_backoff(
        || {
            attempt += 1;
            if attempt < 2 { Err("not ready") } else { Ok(42) }
        },
        3,
        std::time::Duration::from_millis(10),
    );

    assert_eq!(result, Ok(42), "Should succeed after retry");
}

/// Benchmark infrastructure overhead
#[test]
fn test_infrastructure_overhead_is_minimal() {
    let iterations = 100;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        let _env = TestEnvironment::validate().unwrap();
        let _timeout = get_adaptive_timeout();
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;

    eprintln!("Infrastructure overhead: {:?} total, {:?} per iteration", elapsed, per_iteration);

    // Overhead should be negligible (< 1ms per iteration)
    assert!(
        per_iteration < std::time::Duration::from_millis(1),
        "Infrastructure overhead should be minimal"
    );
}
