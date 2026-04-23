use perl_lsp_rs_core::runtime::cancellation::{CancellationRegistry, PerlLspCancellationToken};
use proptest::collection::vec;
use proptest::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
enum RegistryOp {
    Register(u16),
    Cancel(u16),
    Remove(u16),
    GetToken(u16),
    CheckCancelled(u16),
}

fn registry_op_strategy() -> impl Strategy<Value = RegistryOp> {
    prop_oneof![
        (0u16..128).prop_map(RegistryOp::Register),
        (0u16..128).prop_map(RegistryOp::Cancel),
        (0u16..128).prop_map(RegistryOp::Remove),
        (0u16..128).prop_map(RegistryOp::GetToken),
        (0u16..128).prop_map(RegistryOp::CheckCancelled),
    ]
}

proptest! {
    #[test]
    fn prop_registry_active_count_matches_unique_registered_ids(ids in vec(0u64..1000, 1..64)) {
        let registry = CancellationRegistry::new();

        for id in &ids {
            let request_id = Value::from(*id);
            let token = PerlLspCancellationToken::new(request_id, "prop".to_string());
            let _ = registry.register_token(token);
        }

        let unique_count = ids.iter().collect::<HashSet<_>>().len() as u64;
        prop_assert_eq!(registry.active_count(), unique_count as usize);

        for id in &ids {
            let request_id = Value::from(*id);
            let _ = registry.cancel_request(&request_id);
            registry.remove_request(&request_id);
        }

        prop_assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn prop_registry_model_matches_runtime_state(ops in vec(registry_op_strategy(), 1..256)) {
        let registry = CancellationRegistry::new();
        let mut model: HashMap<u16, bool> = HashMap::new();

        for op in ops {
            match op {
                RegistryOp::Register(id) => {
                    let request_id = Value::from(u64::from(id));
                    let token = PerlLspCancellationToken::new(request_id, "model".to_string());
                    let _ = registry.register_token(token);
                    model.insert(id, false);
                }
                RegistryOp::Cancel(id) => {
                    let request_id = Value::from(u64::from(id));
                    let _ = registry.cancel_request(&request_id);
                    if let Some(cancelled) = model.get_mut(&id) {
                        *cancelled = true;
                    }
                }
                RegistryOp::Remove(id) => {
                    let request_id = Value::from(u64::from(id));
                    registry.remove_request(&request_id);
                    model.remove(&id);
                }
                RegistryOp::GetToken(id) => {
                    let request_id = Value::from(u64::from(id));
                    let token_exists = registry.get_token(&request_id).is_some();
                    prop_assert_eq!(token_exists, model.contains_key(&id));
                }
                RegistryOp::CheckCancelled(id) => {
                    let request_id = Value::from(u64::from(id));
                    let is_cancelled = registry.is_cancelled(&request_id);
                    prop_assert_eq!(is_cancelled, *model.get(&id).unwrap_or(&false));
                }
            }

            prop_assert_eq!(registry.active_count(), model.len());
        }
    }

    #[test]
    fn prop_removed_requests_never_leak_from_cache(ids in vec(0u16..128, 1..96)) {
        let registry = CancellationRegistry::new();

        for id in &ids {
            let request_id = Value::from(u64::from(*id));
            let token = PerlLspCancellationToken::new(request_id.clone(), "cache".to_string());
            let _ = registry.register_token(token);

            // Prime the fast-path cache before removal.
            let _ = registry.get_token(&request_id);
            registry.remove_request(&request_id);

            prop_assert!(registry.get_token(&request_id).is_none());
            prop_assert!(!registry.is_cancelled(&request_id));
        }
    }
}
