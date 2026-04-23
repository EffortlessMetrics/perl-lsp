//! BDD helpers for UX-focused LSP integration tests.

use serde_json::Value;
use std::collections::BTreeSet;

#[allow(dead_code)]
pub struct UxScenario {
    name: &'static str,
}

impl UxScenario {
    #[allow(dead_code)]
    pub fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    #[allow(dead_code)]
    pub fn given(&self, message: &str) {
        eprintln!("[{}] Given {message}", self.name);
    }

    #[allow(dead_code)]
    pub fn when(&self, message: &str) {
        eprintln!("[{}] When {message}", self.name);
    }

    #[allow(dead_code)]
    pub fn then(&self, message: &str) {
        eprintln!("[{}] Then {message}", self.name);
    }
}

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
