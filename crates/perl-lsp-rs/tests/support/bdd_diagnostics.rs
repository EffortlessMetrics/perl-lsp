use serde_json::{Value, json};

use super::lsp_harness::LspHarness;

#[allow(unused_imports)]
pub use perl_tdd_support::BddScenario;

// Scenario-style diagnostic flow helpers are intentionally reusable across
// future LSP diagnostic tests.
#[allow(dead_code)]
pub struct DocumentDiagnosticFlow<'a> {
    harness: &'a mut LspHarness,
    uri: String,
}

// Diagnostic flow helpers are fixture API for BDD-style integration tests.
#[allow(dead_code)]
impl<'a> DocumentDiagnosticFlow<'a> {
    pub fn new(harness: &'a mut LspHarness, uri: impl Into<String>) -> Self {
        Self { harness, uri: uri.into() }
    }

    pub fn request(&mut self, previous_result_id: Option<&str>) -> Result<Value, String> {
        let mut params = json!({
            "textDocument": { "uri": self.uri }
        });

        if let Some(previous_result_id) = previous_result_id {
            params["previousResultId"] = Value::String(previous_result_id.to_string());
        }

        self.harness.request("textDocument/diagnostic", params)
    }

    pub fn result_id(report: &Value) -> Result<String, String> {
        report
            .get("resultId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| format!("diagnostic report missing resultId: {report:?}"))
    }

    pub fn kind(report: &Value) -> Option<&str> {
        report.get("kind").and_then(Value::as_str)
    }
}
