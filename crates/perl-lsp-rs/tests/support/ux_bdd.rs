//! BDD helpers for UX-focused LSP integration tests.

use serde_json::Value;
use std::collections::BTreeSet;

#[allow(unused_imports)]
pub use perl_tdd_support::BddScenario as UxScenario;

#[allow(dead_code)]
pub fn completion_labels(response: &Value) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    let items = response.get("items").and_then(Value::as_array).or_else(|| response.as_array());

    if let Some(items) = items {
        for item in items {
            if let Some(label) = item.get("label").and_then(Value::as_str) {
                labels.insert(label.to_string());
            }
        }
    }

    labels
}

#[allow(dead_code)]
pub fn completion_contains_label(response: &Value, label: &str) -> bool {
    completion_labels(response).contains(label)
}
