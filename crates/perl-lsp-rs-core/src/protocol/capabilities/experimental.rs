use super::BuildFlags;
use lsp_types::ServerCapabilities;

pub(super) fn apply_experimental_features(caps: &mut ServerCapabilities, build: &BuildFlags) {
    // Type hierarchy via experimental: lsp-types 0.97 lacks a `type_hierarchy_provider`
    // field on `ServerCapabilities`. We advertise it via `experimental` so that
    // `capabilities_for()` users and `feature_ids_from_caps` can detect the capability.
    // The `handle_initialize` response also injects it at the top-level for clients.
    if build.type_hierarchy {
        insert_experimental_capability(caps, "typeHierarchyProvider", serde_json::json!(true));
    }
}

fn insert_experimental_capability(
    caps: &mut ServerCapabilities,
    key: &'static str,
    value: serde_json::Value,
) {
    let mut experimental = caps.experimental.take().unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = experimental.as_object_mut() {
        obj.insert(key.to_string(), value);
    }
    caps.experimental = Some(experimental);
}
