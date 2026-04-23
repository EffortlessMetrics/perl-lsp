//! UX-focused LSP test harness helpers.
//!
//! This module wraps [`LspHarness`] with a scenario-oriented API so tests can
//! express Given/When/Then behavior without repeating boilerplate setup.

use serde_json::{Value, json};

use super::lsp_harness::{LspHarness, TempWorkspace};

/// Scenario-oriented facade around [`LspHarness`] for UX integration tests.
pub struct LspUxHarness {
    harness: LspHarness,
    workspace: TempWorkspace,
}

impl LspUxHarness {
    /// GIVEN a workspace with files on disk and opened in the server.
    pub fn given_workspace(files: &[(&str, &str)]) -> Result<Self, String> {
        let workspace = TempWorkspace::new()?;

        for (path, content) in files {
            workspace.write(path, content)?;
        }

        let mut harness = LspHarness::new_raw();
        harness.initialize_ready(&workspace.root_uri, None)?;

        for (path, content) in files {
            harness.open(&workspace.uri(path), content)?;
        }

        harness.barrier();

        Ok(Self { harness, workspace })
    }

    /// WHEN the user invokes workspace/symbol.
    pub fn when_workspace_symbol(&mut self, query: &str) -> Result<Value, String> {
        self.harness.request("workspace/symbol", json!({ "query": query }))
    }

    /// WHEN the user invokes go-to-definition at a position in a workspace file.
    pub fn when_go_to_definition(
        &mut self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        self.harness.request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": self.workspace.uri(file_path) },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// THEN helper: get file URI for workspace relative path.
    pub fn uri_for(&self, file_path: &str) -> String {
        self.workspace.uri(file_path)
    }
}

/// Find the first position of `needle` in `source` as LSP (line, character).
pub fn find_position(source: &str, needle: &str) -> Result<(u32, u32), String> {
    for (line_idx, line) in source.lines().enumerate() {
        if let Some(char_idx) = line.find(needle) {
            return Ok((line_idx as u32, char_idx as u32));
        }
    }

    Err(format!("could not find '{needle}' in source"))
}

/// Assert response contains a definition location in `expected_uri`.
pub fn assert_has_location_in_uri(response: &Value, expected_uri: &str) -> Result<(), String> {
    let locations =
        response.as_array().ok_or_else(|| format!("expected location array, got: {response}"))?;

    if locations.iter().any(|location| {
        location.get("uri").and_then(Value::as_str).is_some_and(|uri| uri == expected_uri)
    }) {
        Ok(())
    } else {
        Err(format!("expected at least one location in '{expected_uri}', got: {response}"))
    }
}

/// Assert response contains a symbol result located in `expected_uri`.
pub fn assert_symbol_results_include_uri(
    response: &Value,
    expected_uri: &str,
) -> Result<(), String> {
    let symbols =
        response.as_array().ok_or_else(|| format!("expected symbol array, got: {response}"))?;

    if symbols.iter().any(|symbol| {
        symbol
            .pointer("/location/uri")
            .and_then(Value::as_str)
            .is_some_and(|uri| uri == expected_uri)
    }) {
        Ok(())
    } else {
        Err(format!("expected symbol results to include '{expected_uri}', got: {response}"))
    }
}
