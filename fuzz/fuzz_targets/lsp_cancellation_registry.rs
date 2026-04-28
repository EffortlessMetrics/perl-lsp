#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_lsp_rs_core::runtime::cancellation::{CancellationRegistry, PerlLspCancellationToken};
use serde_json::Value;

fn split_u64(data: &[u8], index: usize) -> Option<u64> {
    if data.len() < index + 8 {
        return None;
    }

    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[index..index + 8]);
    Some(u64::from_le_bytes(bytes))
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let registry = CancellationRegistry::new();
    let mut offset = 0usize;

    while let Some(id) = split_u64(data, offset) {
        let request_id = Value::from(id);
        let token = PerlLspCancellationToken::new(request_id.clone(), "fuzz-provider".to_string());
        let _ = registry.register_token(token);

        if offset % 2 == 0 {
            let _ = registry.get_token(&request_id);
        }

        if offset % 3 == 0 {
            let _ = registry.is_cancelled(&request_id);
        }

        if offset % 5 == 0 {
            let _ = registry.cancel_request(&request_id);
        }

        if offset % 7 == 0 {
            registry.remove_request(&request_id);
        }

        offset += 8;
    }

    for metric_id in 0u8..4 {
        let request_id = Value::from(u64::from(metric_id));
        let _ = registry.register_token(PerlLspCancellationToken::new(
            request_id.clone(),
            "metric-provider".to_string(),
        ));
        let _ = registry.cancel_request(&request_id);
        registry.remove_request(&request_id);
    }
});
