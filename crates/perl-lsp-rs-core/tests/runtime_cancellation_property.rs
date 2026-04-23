use perl_lsp_rs_core::runtime::cancellation::CancellationRegistry;
use proptest::collection::vec;
use proptest::prelude::*;
use serde_json::Value;

proptest! {
    #[test]
    fn prop_registry_active_count_matches_unique_registered_ids(ids in vec(0u64..1000, 1..64)) {
        let registry = CancellationRegistry::new();

    for id in &ids {
            let request_id = Value::from(*id);
            let token = perl_lsp_rs_core::runtime::cancellation::PerlLspCancellationToken::new(
                request_id,
                "prop".to_string(),
            );
            let _ = registry.register_token(token);
        }

        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len() as u64;
        prop_assert_eq!(registry.active_count(), unique_count as usize);

    for id in &ids {
        let request_id = Value::from(*id);
        let _ = registry.cancel_request(&request_id);
        registry.remove_request(&request_id);
    }

        prop_assert_eq!(registry.active_count(), 0);
    }
}
