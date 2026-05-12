use serde_json::{Value, json};

use super::lsp_harness::LspHarness;

#[allow(dead_code)]
pub struct BddScenario {
    name: &'static str,
}

#[allow(dead_code)]
impl BddScenario {
    pub fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {}", name);
        Self { name }
    }

    pub fn given(&self, msg: &str) {
        eprintln!("[{}] Given {}", self.name, msg);
    }

    pub fn when(&self, msg: &str) {
        eprintln!("[{}] When {}", self.name, msg);
    }

    pub fn then(&self, msg: &str) {
        eprintln!("[{}] Then {}", self.name, msg);
    }
}

#[allow(dead_code)]
pub struct DocumentDiagnosticFlow<'a> {
    harness: &'a mut LspHarness,
    uri: String,
}

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
