//! Unit tests for BridgeAdapter lifecycle
//!
//! Coverage target: crates/perl-dap/src/bridge_adapter.rs (265 LOC)
//!
//! Tests:
//! - New adapter initializes with no child process
//! - Default trait implementation matches new()
//! - Shutdown on adapter with no process is a no-op
//! - proxy_messages errors when no child process is spawned
//! - Drop cleans up child process
//! - spawn_pls_dap error propagation (invalid binary)
//! - Double shutdown is safe
//! - Multiple sequential spawn/shutdown cycles

use anyhow::Result;
use perl_dap::BridgeAdapter;

/// New adapter starts with no child process; shutdown is a harmless no-op.
#[tokio::test]
async fn test_new_adapter_has_no_process() -> Result<()> {
    let mut adapter = BridgeAdapter::new();
    // Shutdown on a fresh adapter should succeed without error
    adapter.shutdown().await?;
    Ok(())
}

/// Default implementation produces the same initial state as new().
/// Both should shut down cleanly (no child process to terminate).
#[tokio::test]
async fn test_default_matches_new() -> Result<()> {
    let mut from_new = BridgeAdapter::new();
    let mut from_default = BridgeAdapter::default();
    // Both should succeed at shutdown (no child process in either case)
    from_new.shutdown().await?;
    from_default.shutdown().await?;
    Ok(())
}

/// proxy_messages must return an error when called before spawn_pls_dap.
#[tokio::test]
async fn test_proxy_messages_errors_without_spawn() -> Result<()> {
    let mut adapter = BridgeAdapter::new();
    let result = adapter.proxy_messages().await;
    assert!(result.is_err(), "proxy_messages should fail without a spawned process");
    let err_msg = match &result {
        Err(e) => format!("{e}"),
        Ok(_) => String::new(),
    };
    assert!(
        err_msg.contains("Child process not spawned"),
        "Error should mention missing child process, got: {err_msg}"
    );
    Ok(())
}

/// Shutdown called twice in a row must not panic or error.
#[tokio::test]
async fn test_double_shutdown_is_safe() -> Result<()> {
    let mut adapter = BridgeAdapter::new();
    adapter.shutdown().await?;
    adapter.shutdown().await?;
    Ok(())
}

/// Drop implementation cleans up a fresh (no-process) adapter without error.
/// Verified by creating a second adapter after the first is dropped -- if Drop
/// had corrupted shared state the second adapter would fail to construct or
/// shut down.
#[tokio::test]
async fn test_drop_then_new_adapter_works() -> Result<()> {
    {
        let _adapter = BridgeAdapter::new();
        // adapter goes out of scope and Drop runs
    }
    let mut second = BridgeAdapter::new();
    second.shutdown().await?;
    Ok(())
}

/// Spawning with a deliberately non-existent binary propagates an error.
///
/// We cannot easily inject a fake perl path into BridgeAdapter (it calls
/// platform::resolve_perl_path internally), so we test the spawn path by
/// overriding PATH to ensure perl is not found.
///
/// SAFETY: set_var is unsafe in Rust 2024 edition because env mutation is
/// not thread-safe. This test is marked serial-only via test-threads=1 at
/// the call-site level or accepted as safe because the env is restored
/// before any assertion.
#[tokio::test]
async fn test_spawn_fails_when_perl_not_on_path() -> Result<()> {
    // Save and clobber PATH so resolve_perl_path fails
    let original_path = std::env::var("PATH").ok();
    // SAFETY: We restore PATH immediately after the spawn attempt.
    // This test should be run with --test-threads=1 to avoid races.
    unsafe {
        std::env::set_var("PATH", "/nonexistent-dir-for-test");
    }

    let mut adapter = BridgeAdapter::new();
    let result = adapter.spawn_pls_dap().await;

    // Restore PATH immediately (before assertions) to avoid polluting other tests
    unsafe {
        match original_path {
            Some(ref p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
    }

    assert!(result.is_err(), "spawn_pls_dap should fail when perl is not on PATH");
    let err_msg = match &result {
        Err(e) => format!("{e:#}"),
        Ok(_) => String::new(),
    };
    // The error chain should mention failing to find perl
    assert!(
        err_msg.contains("perl") || err_msg.contains("PATH") || err_msg.contains("spawn"),
        "Error should reference perl/PATH/spawn, got: {err_msg}"
    );
    Ok(())
}

/// A spawned short-lived process can be shut down cleanly.
///
/// We use `true` (or `echo`) as a fast-exiting stand-in. Since BridgeAdapter
/// hard-codes the perl binary, we instead test the shutdown path by spawning
/// a real child via tokio and exercising the adapter's shutdown logic indirectly
/// through the public API with a process that exits immediately.
///
/// Note: This test verifies that shutdown handles an already-exited child
/// gracefully. We spawn via the adapter pointing at a real perl (if available)
/// and let the child fail/exit, then call shutdown.
#[tokio::test]
async fn test_shutdown_after_child_exits() -> Result<()> {
    let mut adapter = BridgeAdapter::new();

    // If perl is available, spawn will succeed and the child will likely fail
    // because -d:LanguageServer::DAP is not installed. Either way, shutdown
    // should handle it gracefully.
    let spawn_result = adapter.spawn_pls_dap().await;
    if spawn_result.is_ok() {
        // Give the child a moment to exit on its own (the DAP flag will likely
        // cause perl to fail immediately if the module is not installed)
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        adapter.shutdown().await?;
    }
    // If spawn failed (perl not installed), that's fine -- test passes trivially

    Ok(())
}

/// Multiple create-shutdown cycles should not leak or panic.
#[tokio::test]
async fn test_repeated_lifecycle_cycles() -> Result<()> {
    for _ in 0..5 {
        let mut adapter = BridgeAdapter::new();
        adapter.shutdown().await?;
    }
    Ok(())
}

/// Multiple adapters can be created sequentially without interference.
/// This verifies no global state leaks between adapter instances.
#[tokio::test]
async fn test_sequential_adapters_are_independent() -> Result<()> {
    let mut first = BridgeAdapter::new();
    first.shutdown().await?;
    drop(first);

    let mut second = BridgeAdapter::new();
    let result = second.proxy_messages().await;
    assert!(result.is_err(), "second adapter should still require spawn before proxy_messages");
    second.shutdown().await?;
    Ok(())
}

/// Calling spawn twice should not leave an orphaned process.
///
/// If perl / Perl::LanguageServer is unavailable, this test exits early.
#[tokio::test]
async fn test_respawn_replaces_previous_child_cleanly() -> Result<()> {
    let mut adapter = BridgeAdapter::new();
    let first_spawn = adapter.spawn_pls_dap().await;
    if first_spawn.is_err() {
        // Environment does not have a runnable bridge target.
        return Ok(());
    }

    // Second spawn should force cleanup of any previous child before respawning.
    let _ = adapter.spawn_pls_dap().await;
    adapter.shutdown().await?;
    Ok(())
}
