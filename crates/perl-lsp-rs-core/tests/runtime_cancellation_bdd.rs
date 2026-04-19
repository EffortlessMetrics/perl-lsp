use perl_lsp_rs_core::runtime::cancellation::{
    CancellationRegistry, GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken,
    RequestCleanupGuard,
};
use serde_json::json;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn given_an_active_request_when_cancelled_then_global_registry_cleans_and_marks_cancelled()
-> Result<(), Box<dyn std::error::Error>> {
    let _lock = TEST_LOCK.lock().map_err(|e| format!("lock error: {e}"))?;
    let request_id = json!(9001);
    let token = PerlLspCancellationToken::new(request_id.clone(), "given/request".to_string());

    GLOBAL_CANCELLATION_REGISTRY.remove_request(&request_id);
    let count_before = GLOBAL_CANCELLATION_REGISTRY.active_count();
    assert!(!GLOBAL_CANCELLATION_REGISTRY.is_cancelled(&request_id));

    GLOBAL_CANCELLATION_REGISTRY.register_token(token)?;
    assert_eq!(GLOBAL_CANCELLATION_REGISTRY.active_count(), count_before + 1);
    assert!(!GLOBAL_CANCELLATION_REGISTRY.is_cancelled(&request_id));

    let snapshot =
        GLOBAL_CANCELLATION_REGISTRY.get_token(&request_id).ok_or("token should be retrievable")?;
    assert!(!snapshot.is_cancelled());

    let cancel_context = GLOBAL_CANCELLATION_REGISTRY.cancel_request(&request_id)?;
    assert!(cancel_context.is_none());
    assert!(GLOBAL_CANCELLATION_REGISTRY.is_cancelled(&request_id));

    GLOBAL_CANCELLATION_REGISTRY.remove_request(&request_id);
    assert_eq!(GLOBAL_CANCELLATION_REGISTRY.active_count(), count_before);
    Ok(())
}

#[test]
fn given_cleanup_guard_when_dropped_then_request_is_removed()
-> Result<(), Box<dyn std::error::Error>> {
    let _lock = TEST_LOCK.lock().map_err(|e| format!("lock error: {e}"))?;
    let request_id = json!(9002);
    let token = PerlLspCancellationToken::new(request_id.clone(), "guard-scope".to_string());

    GLOBAL_CANCELLATION_REGISTRY.remove_request(&request_id);
    let count_before = GLOBAL_CANCELLATION_REGISTRY.active_count();
    GLOBAL_CANCELLATION_REGISTRY.register_token(token)?;
    assert_eq!(GLOBAL_CANCELLATION_REGISTRY.active_count(), count_before + 1);

    {
        let _guard = RequestCleanupGuard::new(Some(request_id.clone()));
    }

    assert_eq!(GLOBAL_CANCELLATION_REGISTRY.active_count(), count_before);
    Ok(())
}

#[test]
fn when_multiple_requests_registered_then_counts_match_active_registry_size() {
    let registry = CancellationRegistry::new();

    for i in 0..32u64 {
        let token =
            PerlLspCancellationToken::new(json!(format!("id-{i}")), format!("provider-{i}"));
        let _ = registry.register_token(token);
    }

    assert_eq!(registry.active_count(), 32);

    for i in 0..32u64 {
        let _ = registry.cancel_request(&json!(format!("id-{i}")));
        registry.remove_request(&json!(format!("id-{i}")));
    }

    assert_eq!(registry.active_count(), 0);
}
