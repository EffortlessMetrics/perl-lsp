//! End-to-end completion tests for indirect-object method calls (#1758).
//!
//! These drive the real `textDocument/completion` request path through the LSP
//! server, so the indirect-call dispatch seams (`is_indirect_method_word`,
//! `indirect_word_end`, `parse_indirect_receiver`,
//! `complete_indirect_method_context`) are observed through a production caller
//! — not just a unit test of the helpers.

mod common;

#[cfg(test)]
mod indirect_completion_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;

    fn completion_items(response: &Value) -> Option<&Vec<Value>> {
        response["result"]["items"].as_array().or_else(|| response["result"].as_array())
    }

    /// `new Child` (indirect-object constructor syntax) should offer the
    /// receiver class's methods via the LSP completion request, exercising the
    /// indirect-call dispatch path end to end.
    #[test]
    fn indirect_bareword_receiver_completes_methods_e2e() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = r#"package Child;
sub run { }
sub speak { }

package main;
new Child
"#;
        let uri = "file:///indirect_completion.pl";
        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Cursor right after the method word `new` on the `new Child` line.
        let completion_line = code
            .lines()
            .position(|line| line.contains("new Child"))
            .ok_or("completion line not found")? as u32;
        let completion_char = "new".len() as u32;

        let response = server.get_completion(uri, completion_line, completion_char);
        let items = completion_items(&response).ok_or("missing completion items")?;
        assert!(
            items.iter().any(|item| item["label"] == "run"),
            "indirect `new Child` should offer Child method `run`; got: {response:#}"
        );

        server.shutdown();
        Ok(())
    }

    /// A non-indirect statement (`print $fh`) routed through the same production
    /// completion request must NOT surface receiver-class methods — guarding the
    /// builtin/keyword exclusion through the real caller.
    #[test]
    fn indirect_builtin_does_not_complete_methods_e2e() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package Child;
sub run { }

package main;
print Child
"#;
        let uri = "file:///indirect_print.pl";
        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        let completion_line = code
            .lines()
            .position(|line| line.contains("print Child"))
            .ok_or("completion line not found")? as u32;
        let completion_char = "print".len() as u32;

        let response = server.get_completion(uri, completion_line, completion_char);
        let items = completion_items(&response).ok_or("missing completion items")?;
        assert!(
            !items.iter().any(|item| item["label"] == "run"),
            "`print Child` is a builtin call and must not offer method `run`; got: {response:#}"
        );

        server.shutdown();
        Ok(())
    }
}
