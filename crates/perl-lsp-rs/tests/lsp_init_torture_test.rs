//! Init Torture Test
//!
//! This test rapidly cycles through LSP initialization sequences to detect
//! race conditions that cause BrokenPipe errors. It validates that the
//! `initialize_ready()` pattern provides deterministic, reliable initialization.
//!
//! Run with:
//! ```bash
//! RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_init_torture_test -- --test-threads=1
//! ```

// Integration tests print diagnostic output for CI troubleshooting; this is
// not the LSP server's stdio transport, so print_stdout/print_stderr don't
// apply the way they do to production code.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod support;

use serde_json::json;
use support::lsp_harness::{LspHarness, TestContext};

/// Number of iterations for torture tests
const TORTURE_ITERATIONS: usize = 50;

/// Reduced iterations for CI environments
const CI_TORTURE_ITERATIONS: usize = 20;

fn is_coverage_instrumented() -> bool {
    std::env::var_os("LLVM_PROFILE_FILE").is_some()
        || std::env::var_os("CARGO_LLVM_COV").is_some()
        || std::env::var_os("CARGO_LLVM_COV_TARGET_DIR").is_some()
}

fn get_iterations() -> usize {
    if std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || is_coverage_instrumented()
    {
        CI_TORTURE_ITERATIONS
    } else {
        TORTURE_ITERATIONS
    }
}

/// Test: Rapid init/shutdown cycles with LspHarness
///
/// This catches the exact class of race that produced BrokenPipe errors.
#[test]
fn torture_init_shutdown_cycles() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::test_workspace::TempWorkspace::new()?;
    let iterations = get_iterations();
    let mut success_count = 0;
    let mut failure_count = 0;

    for i in 0..iterations {
        let result = std::panic::catch_unwind(|| -> Result<(), Box<dyn std::error::Error>> {
            let mut harness = LspHarness::new_raw();

            // Initialize with barrier
            harness.initialize_ready(&workspace.root_uri, None)?;

            // Open a tiny document
            harness.open("file:///test.pl", "my $x = 1;")?;

            // Request hover
            let result = harness.request(
                "textDocument/hover",
                json!({
                    "textDocument": { "uri": "file:///test.pl" },
                    "position": { "line": 0, "character": 4 }
                }),
            );

            // Verify we got a response (success or null is fine, error is not)
            result?;

            // Graceful shutdown (happens automatically on drop, but we can be explicit)
            harness.shutdown_gracefully();
            Ok(())
        });

        match result {
            Ok(Ok(())) => success_count += 1,
            Ok(Err(error)) => {
                failure_count += 1;
                eprintln!("Init torture test failed on iteration {}: {error}", i + 1);
            }
            Err(_) => {
                failure_count += 1;
                eprintln!("Init torture test failed on iteration {} with panic", i + 1);
            }
        }
    }

    // All iterations should succeed
    assert_eq!(
        failure_count, 0,
        "Init torture test had {} failures out of {} iterations. \
         This indicates a race condition in the initialization sequence.",
        failure_count, iterations
    );

    eprintln!("Init torture test passed: {}/{} iterations successful", success_count, iterations);
    Ok(())
}

/// Test: Rapid init/shutdown cycles with TestContext wrapper
///
/// Validates that the TestContext compatibility wrapper also prevents races.
#[test]
fn torture_test_context_init_cycles() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::test_workspace::TempWorkspace::new()?;
    let iterations = get_iterations();
    let mut success_count = 0;

    for _ in 0..iterations {
        let mut ctx = TestContext::new();
        let _ = ctx.initialize_with(&workspace.root_uri, None);

        ctx.open_document("file:///test.pl", "my $x = 1;");

        ctx.send_request(
            "textDocument/hover",
            Some(json!({
                "textDocument": { "uri": "file:///test.pl" },
                "position": { "line": 0, "character": 4 }
            })),
        )
        .ok_or("TestContext hover request failed")?;
        success_count += 1;
    }

    eprintln!(
        "TestContext torture test passed: {}/{} iterations successful",
        success_count, iterations
    );
    Ok(())
}

/// Test: Multiple documents open/close in rapid succession
#[test]
fn torture_document_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::test_workspace::TempWorkspace::new()?;
    let iterations = get_iterations();

    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(&workspace.root_uri, None)?;

    for i in 0..iterations {
        let uri = format!("file:///test_{}.pl", i);
        let content = format!("my $var{} = {};", i, i);

        // Open document
        harness.open(&uri, &content)?;

        // Barrier to ensure processing
        if i % 10 == 0 {
            harness.barrier();
        }

        // Close document
        harness.close(&uri)?;
    }

    // Final barrier
    harness.barrier();

    eprintln!("Document lifecycle torture test passed: {} open/close cycles", iterations);
    Ok(())
}

/// Test: Rapid document updates
#[test]
fn torture_document_updates() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::test_workspace::TempWorkspace::new()?;
    let iterations = get_iterations();

    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(&workspace.root_uri, None)?;

    let uri = "file:///test.pl";
    harness.open(uri, "my $x = 0;")?;

    for i in 1..=iterations {
        let content = format!("my $x = {};", i);
        harness.change_full(uri, i as i32, &content)?;
    }

    // Final barrier
    harness.barrier();

    eprintln!("Document update torture test passed: {} updates", iterations);
    Ok(())
}
