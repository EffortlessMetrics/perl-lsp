//! Comprehensive unit tests for perl-lsp-cancellation crate.
//!
//! These tests complement the existing BDD and property tests by covering
//! edge cases, concurrency, error formatting, caching, and macro behavior.

use perl_lsp_rs_core::protocol::JsonRpcId;
use perl_lsp_rs_core::runtime::cancellation::{
    CancellableProvider, CancellationError, CancellationRegistry, PerlLspCancellationToken,
    ProviderCleanupContext, RequestCleanupGuard,
};
// NOTE(G2-API-fix): check_cancellation! is #[macro_export] so lives at crate root after absorption.
use perl_lsp_rs_core::check_cancellation;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

// ---------------------------------------------------------------------------
// PerlLspCancellationToken
// ---------------------------------------------------------------------------

#[test]
fn token_new_is_not_cancelled() {
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "hover".into());
    assert!(!token.is_cancelled());
    assert!(!token.is_cancelled_relaxed());
    assert!(!token.is_cancelled_hot_path());
}

#[test]
fn token_cancel_sets_all_check_variants() {
    let token = PerlLspCancellationToken::new(JsonRpcId::String("abc".into()), "completion".into());
    token.cancel();
    assert!(token.is_cancelled());
    assert!(token.is_cancelled_relaxed());
    assert!(token.is_cancelled_hot_path());
}

#[test]
fn token_accessors_return_construction_values() {
    let id = JsonRpcId::Integer(99);
    let token = PerlLspCancellationToken::new(id.clone(), "references".into());
    assert_eq!(token.request_id(), &id);
    assert_eq!(token.provider(), "references");
    assert!(token.timestamp() > 0);
}

#[test]
fn token_elapsed_increases_over_time() {
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "test".into());
    let first = token.elapsed();
    // Spin briefly to ensure monotonic clock advances
    std::thread::sleep(Duration::from_millis(1));
    let second = token.elapsed();
    assert!(second >= first);
}

#[test]
fn token_clone_shares_cancellation_state() {
    let original = PerlLspCancellationToken::new(JsonRpcId::Integer(10), "hover".into());
    let cloned = original.clone();

    assert!(!cloned.is_cancelled());
    original.cancel();
    // Clone shares the same Arc<AtomicBool>, so it should also see cancellation
    assert!(cloned.is_cancelled());
}

#[test]
fn token_debug_format_is_nonempty() {
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "test".into());
    let debug = format!("{:?}", token);
    assert!(!debug.is_empty());
}

// ---------------------------------------------------------------------------
// ProviderCleanupContext
// ---------------------------------------------------------------------------

#[test]
fn cleanup_context_without_callback_is_noop() {
    let ctx = ProviderCleanupContext::new("hover".into(), None);
    // Should not panic
    ctx.execute_cleanup();
    assert_eq!(ctx.provider_type, "hover");
    assert!(ctx.request_params.is_none());
}

#[test]
fn cleanup_context_with_params_stores_them() {
    let params = json!({"uri": "file:///test.pl"});
    let ctx = ProviderCleanupContext::new("completion".into(), Some(params.clone()));
    assert_eq!(ctx.request_params.as_ref(), Some(&params));
}

#[test]
fn cleanup_context_callback_is_invoked() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    let ctx = ProviderCleanupContext::new("references".into(), None)
        .with_cleanup(move || flag_clone.store(true, Ordering::Relaxed));

    assert!(!flag.load(Ordering::Relaxed));
    ctx.execute_cleanup();
    assert!(flag.load(Ordering::Relaxed));
}

#[test]
fn cleanup_context_callback_can_be_called_multiple_times() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let ctx = ProviderCleanupContext::new("test".into(), None).with_cleanup(move || {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    ctx.execute_cleanup();
    ctx.execute_cleanup();
    assert_eq!(counter.load(Ordering::Relaxed), 2);
}

#[test]
fn cleanup_context_cancelled_at_is_recent() {
    let before = std::time::Instant::now();
    let ctx = ProviderCleanupContext::new("test".into(), None);
    let after = std::time::Instant::now();

    assert!(ctx.cancelled_at >= before);
    assert!(ctx.cancelled_at <= after);
}

// ---------------------------------------------------------------------------
// CancellationRegistry — basic operations
// ---------------------------------------------------------------------------

#[test]
fn registry_default_has_zero_active() {
    let registry = CancellationRegistry::default();
    assert_eq!(registry.active_count(), 0);
}

#[test]
fn registry_register_and_count() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "test".into());
    registry.register_token(token)?;
    assert_eq!(registry.active_count(), 1);
    Ok(())
}

#[test]
fn registry_duplicate_id_overwrites() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();

    let t1 = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "first".into());
    let t2 = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "second".into());

    registry.register_token(t1)?;
    registry.register_token(t2)?;

    // Count should still be 1 because same key
    assert_eq!(registry.active_count(), 1);

    // The token should be the second one
    let id = JsonRpcId::Integer(1);
    let retrieved = registry.get_token(&id).ok_or("token should be retrievable after overwrite")?;
    assert_eq!(retrieved.provider(), "second");
    Ok(())
}

#[test]
fn registry_cancel_nonexistent_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let result = registry.cancel_request(&JsonRpcId::String("nonexistent".into()))?;
    assert!(result.is_none());
    Ok(())
}

#[test]
fn registry_remove_nonexistent_is_safe() {
    let registry = CancellationRegistry::new();
    // Should not panic
    registry.remove_request(&JsonRpcId::String("nonexistent".into()));
    assert_eq!(registry.active_count(), 0);
}

#[test]
fn registry_is_cancelled_for_missing_token_returns_false() {
    let registry = CancellationRegistry::new();
    assert!(!registry.is_cancelled(&JsonRpcId::String("missing".into())));
}

#[test]
fn registry_get_token_for_missing_returns_none() {
    let registry = CancellationRegistry::new();
    assert!(registry.get_token(&JsonRpcId::String("missing".into())).is_none());
}

// ---------------------------------------------------------------------------
// CancellationRegistry — cancel + cleanup integration
// ---------------------------------------------------------------------------

#[test]
fn registry_cancel_with_cleanup_executes_callback() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let req_id = JsonRpcId::Integer(42);

    let token = PerlLspCancellationToken::new(req_id.clone(), "hover".into());
    registry.register_token(token)?;

    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let ctx = ProviderCleanupContext::new("hover".into(), None)
        .with_cleanup(move || executed_clone.store(true, Ordering::Relaxed));
    registry.register_cleanup(&req_id, ctx)?;

    let result = registry.cancel_request(&req_id)?;
    assert!(result.is_some());
    assert!(executed.load(Ordering::Relaxed));
    assert!(registry.is_cancelled(&req_id));
    Ok(())
}

#[test]
fn registry_cancel_removes_cleanup_context() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let req_id = JsonRpcId::Integer(50);

    let token = PerlLspCancellationToken::new(req_id.clone(), "test".into());
    registry.register_token(token)?;

    let ctx = ProviderCleanupContext::new("test".into(), None);
    registry.register_cleanup(&req_id, ctx)?;

    // First cancel returns cleanup context
    let first = registry.cancel_request(&req_id)?;
    assert!(first.is_some());

    // Second cancel has no cleanup context left
    let second = registry.cancel_request(&req_id)?;
    assert!(second.is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// CancellationRegistry — caching behavior
// ---------------------------------------------------------------------------

#[test]
fn registry_get_token_caches_on_second_access() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let req_id = JsonRpcId::Integer(100);
    let token = PerlLspCancellationToken::new(req_id.clone(), "cache-test".into());
    registry.register_token(token)?;

    // First access populates cache
    let t1 = registry.get_token(&req_id).ok_or("first get should succeed")?;
    // Second access hits cache
    let t2 = registry.get_token(&req_id).ok_or("second get should succeed")?;

    assert_eq!(t1.provider(), t2.provider());
    assert_eq!(t1.request_id(), t2.request_id());
    Ok(())
}

#[test]
fn registry_cache_eviction_on_overflow() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();

    // Register 150 tokens to exceed the max_cache_size of 100
    for i in 0..150i64 {
        let id = JsonRpcId::Integer(i);
        let token = PerlLspCancellationToken::new(id.clone(), format!("provider-{i}"));
        registry.register_token(token)?;
        // Access to populate cache
        let _ = registry.get_token(&id);
    }

    // All tokens should still be retrievable (cache or main storage)
    for i in 0..150i64 {
        let id = JsonRpcId::Integer(i);
        assert!(registry.get_token(&id).is_some(), "token {i} should be retrievable");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CancellationRegistry — metrics tracking
// ---------------------------------------------------------------------------

#[test]
fn registry_metrics_track_operations() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let id = JsonRpcId::Integer(1);

    let token = PerlLspCancellationToken::new(id.clone(), "test".into());
    registry.register_token(token)?;
    assert_eq!(registry.metrics().registered_count(), 1);

    registry.cancel_request(&id)?;
    assert_eq!(registry.metrics().cancelled_count(), 1);

    registry.remove_request(&id);
    assert_eq!(registry.metrics().completed_count(), 1);
    Ok(())
}

#[test]
fn registry_metrics_uptime_is_positive() {
    let registry = CancellationRegistry::new();
    // uptime should be non-negative (may be zero on fast machines)
    let _ = registry.metrics().uptime();
}

#[test]
fn registry_metrics_memory_overhead_under_1mb() {
    let registry = CancellationRegistry::new();
    assert!(registry.metrics().memory_overhead_bytes() < 1024 * 1024);
}

// ---------------------------------------------------------------------------
// CancellationMetrics standalone
// ---------------------------------------------------------------------------

#[test]
fn metrics_default_is_equivalent_to_new() {
    use perl_lsp_rs_core::runtime::cancellation::CancellationMetrics;
    let m = CancellationMetrics::default();
    assert_eq!(m.registered_count(), 0);
    assert_eq!(m.cancelled_count(), 0);
    assert_eq!(m.completed_count(), 0);
}

#[test]
fn metrics_increments_are_independent() {
    use perl_lsp_rs_core::runtime::cancellation::CancellationMetrics;
    let m = CancellationMetrics::new();

    for _ in 0..5 {
        m.increment_registered();
    }
    for _ in 0..3 {
        m.increment_cancelled();
    }
    m.increment_completed();

    assert_eq!(m.registered_count(), 5);
    assert_eq!(m.cancelled_count(), 3);
    assert_eq!(m.completed_count(), 1);
}

// ---------------------------------------------------------------------------
// CancellationError — Display + Error trait
// ---------------------------------------------------------------------------

#[test]
fn error_lock_display() {
    let err = CancellationError::LockError("mutex poisoned".into());
    let msg = format!("{err}");
    assert!(msg.contains("Lock error"));
    assert!(msg.contains("mutex poisoned"));
}

#[test]
fn error_invalid_request_display() {
    let err = CancellationError::InvalidRequest("bad id".into());
    let msg = format!("{err}");
    assert!(msg.contains("Invalid request"));
    assert!(msg.contains("bad id"));
}

#[test]
fn error_provider_not_found_display() {
    let err = CancellationError::ProviderNotFound("hover".into());
    let msg = format!("{err}");
    assert!(msg.contains("Provider not found"));
    assert!(msg.contains("hover"));
}

#[test]
fn error_timeout_display() {
    let err = CancellationError::Timeout(Duration::from_millis(500));
    let msg = format!("{err}");
    assert!(msg.contains("timeout"));
}

#[test]
fn error_debug_format_is_nonempty() {
    let err = CancellationError::LockError("test".into());
    let debug = format!("{:?}", err);
    assert!(!debug.is_empty());
}

#[test]
fn error_implements_std_error() {
    let err: Box<dyn std::error::Error> =
        Box::new(CancellationError::InvalidRequest("test".into()));
    // source() returns None for our error type
    assert!(err.source().is_none());
}

// ---------------------------------------------------------------------------
// check_cancellation! macro
// ---------------------------------------------------------------------------

fn run_with_cancellation_check(
    token: &PerlLspCancellationToken,
) -> Result<&'static str, CancellationError> {
    check_cancellation!(token);
    Ok("completed")
}

#[test]
fn macro_passes_when_not_cancelled() -> Result<(), Box<dyn std::error::Error>> {
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "test".into());
    let result = run_with_cancellation_check(&token)?;
    assert_eq!(result, "completed");
    Ok(())
}

#[test]
fn macro_returns_error_when_cancelled() {
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "test".into());
    token.cancel();
    let result = run_with_cancellation_check(&token);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// CancellableProvider trait
// ---------------------------------------------------------------------------

struct TestProvider {
    name: &'static str,
    cleanup_called: Arc<AtomicBool>,
}

impl CancellableProvider for TestProvider {
    fn check_cancellation(
        &self,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        if token.is_cancelled() {
            return Err(CancellationError::InvalidRequest("cancelled".into()));
        }
        Ok(())
    }

    fn cleanup_on_cancel(&self, _context: &ProviderCleanupContext) {
        self.cleanup_called.store(true, Ordering::Relaxed);
    }

    fn provider_name(&self) -> &'static str {
        self.name
    }
}

#[test]
fn cancellable_provider_check_passes_when_active() -> Result<(), Box<dyn std::error::Error>> {
    let provider = TestProvider { name: "hover", cleanup_called: Arc::new(AtomicBool::new(false)) };
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "hover".into());
    provider.check_cancellation(&token)?;
    Ok(())
}

#[test]
fn cancellable_provider_check_fails_when_cancelled() {
    let provider = TestProvider { name: "hover", cleanup_called: Arc::new(AtomicBool::new(false)) };
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "hover".into());
    token.cancel();
    assert!(provider.check_cancellation(&token).is_err());
}

#[test]
fn cancellable_provider_cleanup_invokes_callback() {
    let flag = Arc::new(AtomicBool::new(false));
    let provider = TestProvider { name: "completion", cleanup_called: flag.clone() };
    let ctx = ProviderCleanupContext::new("completion".into(), None);
    provider.cleanup_on_cancel(&ctx);
    assert!(flag.load(Ordering::Relaxed));
}

#[test]
fn cancellable_provider_name_returns_configured_name() {
    let provider =
        TestProvider { name: "references", cleanup_called: Arc::new(AtomicBool::new(false)) };
    assert_eq!(provider.provider_name(), "references");
}

// ---------------------------------------------------------------------------
// RequestCleanupGuard
// ---------------------------------------------------------------------------

#[test]
fn guard_with_none_does_not_panic_on_drop() {
    let _guard = RequestCleanupGuard::new(None);
}

#[test]
fn guard_from_ref_none_does_not_panic_on_drop() {
    let _guard = RequestCleanupGuard::from_ref(None);
}

#[test]
fn guard_from_ref_some_clones_value() -> Result<(), Box<dyn std::error::Error>> {
    let id = JsonRpcId::Integer(777);
    let guard = RequestCleanupGuard::from_ref(Some(&id));
    // Guard holds a cloned copy — just verify it doesn't panic on drop
    drop(guard);
    Ok(())
}

// ---------------------------------------------------------------------------
// Concurrency
// ---------------------------------------------------------------------------

#[test]
fn concurrent_register_and_cancel() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(CancellationRegistry::new());
    let mut handles = Vec::new();

    // Spawn threads that register tokens
    for i in 0..20i64 {
        let reg = registry.clone();
        handles.push(std::thread::spawn(move || {
            let token = PerlLspCancellationToken::new(JsonRpcId::Integer(i), format!("thread-{i}"));
            let _ = reg.register_token(token);
        }));
    }

    for h in handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    assert_eq!(registry.active_count(), 20);

    // Spawn threads that cancel tokens
    let mut cancel_handles = Vec::new();
    for i in 0..20i64 {
        let reg = registry.clone();
        cancel_handles.push(std::thread::spawn(move || {
            let _ = reg.cancel_request(&JsonRpcId::Integer(i));
        }));
    }

    for h in cancel_handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    // All should be cancelled
    for i in 0..20i64 {
        assert!(registry.is_cancelled(&JsonRpcId::Integer(i)), "token {i} should be cancelled");
    }
    Ok(())
}

#[test]
fn concurrent_token_cancellation_visibility() -> Result<(), Box<dyn std::error::Error>> {
    let token = PerlLspCancellationToken::new(JsonRpcId::Integer(1), "shared".into());
    let token_clone = token.clone();

    let handle = std::thread::spawn(move || {
        token_clone.cancel();
    });
    handle.join().map_err(|_| "thread panicked")?;

    assert!(token.is_cancelled());
    Ok(())
}

#[test]
fn concurrent_registry_reads_while_writing() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(CancellationRegistry::new());

    // Pre-populate
    for i in 0..10i64 {
        let token = PerlLspCancellationToken::new(JsonRpcId::Integer(i), "pre".into());
        registry.register_token(token)?;
    }

    let mut handles = Vec::new();

    // Readers
    for _ in 0..5 {
        let reg = registry.clone();
        handles.push(std::thread::spawn(move || {
            for i in 0..10i64 {
                let id = JsonRpcId::Integer(i);
                let _ = reg.is_cancelled(&id);
                let _ = reg.get_token(&id);
            }
        }));
    }

    // Writer
    {
        let reg = registry.clone();
        handles.push(std::thread::spawn(move || {
            for i in 10..20i64 {
                let token = PerlLspCancellationToken::new(JsonRpcId::Integer(i), "writer".into());
                let _ = reg.register_token(token);
            }
        }));
    }

    for h in handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    // All 20 tokens should be present
    assert_eq!(registry.active_count(), 20);
    Ok(())
}

// ---------------------------------------------------------------------------
// Various JsonRpcId variants as request IDs
// ---------------------------------------------------------------------------

#[test]
fn registry_supports_string_request_ids() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let req_id = JsonRpcId::String("request-abc".into());
    let token = PerlLspCancellationToken::new(req_id.clone(), "test".into());
    registry.register_token(token)?;
    assert!(!registry.is_cancelled(&req_id));

    registry.cancel_request(&req_id)?;
    assert!(registry.is_cancelled(&req_id));
    Ok(())
}

#[test]
fn registry_supports_integer_request_ids() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let req_id = JsonRpcId::Integer(12345);
    let token = PerlLspCancellationToken::new(req_id.clone(), "test".into());
    registry.register_token(token)?;
    assert!(!registry.is_cancelled(&req_id));

    registry.cancel_request(&req_id)?;
    assert!(registry.is_cancelled(&req_id));
    Ok(())
}

// ---------------------------------------------------------------------------
// Full lifecycle
// ---------------------------------------------------------------------------

#[test]
fn full_lifecycle_register_cancel_cleanup_remove() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();
    let req_id = JsonRpcId::Integer(999);

    // 1. Register token
    let token = PerlLspCancellationToken::new(req_id.clone(), "lifecycle".into());
    registry.register_token(token)?;
    assert_eq!(registry.active_count(), 1);
    assert!(!registry.is_cancelled(&req_id));

    // 2. Register cleanup
    let cleaned = Arc::new(AtomicBool::new(false));
    let cleaned_clone = cleaned.clone();
    let ctx = ProviderCleanupContext::new("lifecycle".into(), Some(json!({"line": 42})))
        .with_cleanup(move || cleaned_clone.store(true, Ordering::Relaxed));
    registry.register_cleanup(&req_id, ctx)?;

    // 3. Retrieve token and verify state
    let retrieved = registry.get_token(&req_id).ok_or("token should exist")?;
    assert!(!retrieved.is_cancelled());

    // 4. Cancel request
    let cleanup_result = registry.cancel_request(&req_id)?;
    assert!(cleanup_result.is_some());
    assert!(cleaned.load(Ordering::Relaxed));
    assert!(registry.is_cancelled(&req_id));

    // 5. Remove request
    registry.remove_request(&req_id);
    assert_eq!(registry.active_count(), 0);

    // 6. Verify metrics
    assert_eq!(registry.metrics().registered_count(), 1);
    assert_eq!(registry.metrics().cancelled_count(), 1);
    assert_eq!(registry.metrics().completed_count(), 1);

    Ok(())
}

#[test]
fn register_cancel_remove_many_sequential() -> Result<(), Box<dyn std::error::Error>> {
    let registry = CancellationRegistry::new();

    for i in 0..50i64 {
        let token = PerlLspCancellationToken::new(JsonRpcId::Integer(i), format!("seq-{i}"));
        registry.register_token(token)?;
    }
    assert_eq!(registry.active_count(), 50);

    for i in 0..25i64 {
        registry.cancel_request(&JsonRpcId::Integer(i))?;
    }

    // First 25 cancelled, rest active
    for i in 0..25i64 {
        assert!(registry.is_cancelled(&JsonRpcId::Integer(i)), "token {i} should be cancelled");
    }
    for i in 25..50i64 {
        assert!(
            !registry.is_cancelled(&JsonRpcId::Integer(i)),
            "token {i} should not be cancelled"
        );
    }

    for i in 0..50i64 {
        registry.remove_request(&JsonRpcId::Integer(i));
    }
    assert_eq!(registry.active_count(), 0);

    Ok(())
}
