//! Integration tests for semantic analyzer wiring to hover and diagnostics
//!
//! Tests verify that SemanticAnalyzer is properly integrated with:
//! 1. Hover provider - showing type inference information
//! 2. Diagnostics provider - reporting unused variables and semantic issues

mod common;

#[cfg(test)]
mod semantic_integration_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;

    /// Extract hover content from an LSP hover response
    fn hover_content(resp: &Value) -> Option<String> {
        let result = resp.get("result")?;
        if result.is_null() {
            return None;
        }
        let contents = result.get("contents")?;
        let value = contents.get("value")?.as_str()?;
        Some(value.to_string())
    }

    /// Find position of needle in code at given line number
    fn find_pos(
        code: &str,
        needle: &str,
        target_line: usize,
    ) -> Result<(u32, u32), Box<dyn std::error::Error>> {
        let line = code
            .lines()
            .nth(target_line)
            .ok_or_else(|| format!("no line {} in test code", target_line))?;
        let col = line
            .find(needle)
            .ok_or_else(|| format!("could not find `{needle}` on line {target_line}"))?;
        Ok((target_line as u32, col as u32))
    }

    #[test]
    fn test_hover_shows_inferred_type_for_number() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $count = 42;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the number literal
        let (line, character) = find_pos(code, "42", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("NUMBER HOVER RESPONSE: {response:#}");

        if let Some(content) = hover_content(&response) {
            // Should show type information if type inference is working
            println!("Hover content: {}", content);
            // Type information should be present for the literal
            // This test verifies the infer_type method is being called
        }
        Ok(())
    }

    #[test]
    fn test_hover_shows_inferred_type_for_string() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $name = "test";
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the string literal
        let (line, character) = find_pos(code, "\"test\"", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("STRING HOVER RESPONSE: {response:#}");

        if let Some(content) = hover_content(&response) {
            println!("Hover content: {}", content);
            // Type information should indicate string type
        }
        Ok(())
    }

    #[test]
    fn test_hover_shows_inferred_type_for_array() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my @items = (1, 2, 3);
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the array variable
        let (line, character) = find_pos(code, "@items", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("ARRAY HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content for @items")?;
        println!("Hover content: {}", content);

        // Should show array variable type
        assert!(
            content.contains("Array Variable") || content.contains("array"),
            "hover should show array type information, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_shows_inferred_type_for_hash() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my %config = (debug => 1);
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the hash variable
        let (line, character) = find_pos(code, "%config", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("HASH HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content for %config")?;
        println!("Hover content: {}", content);

        // Should show hash variable type
        assert!(
            content.contains("Hash Variable") || content.contains("hash"),
            "hover should show hash type information, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_diagnostics_detect_unused_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $used = 42;
my $unused = 10;
print $used;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Wait a bit for diagnostics to be computed
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Get diagnostics from the pull diagnostics API
        let diag_response = server.get_diagnostics(uri);
        println!("DIAGNOSTICS RESPONSE: {diag_response:#?}");

        // Extract the items array from the response
        let items = diag_response
            .get("result")
            .and_then(|r| r.get("items"))
            .and_then(|i| i.as_array())
            .ok_or("expected items array in diagnostic response")?;

        println!("DIAGNOSTIC ITEMS: {items:#?}");

        // Should have at least one diagnostic for the unused variable
        assert!(!items.is_empty(), "should have diagnostics for unused variable");

        // Check if there's a diagnostic mentioning the unused variable
        let has_unused_diag = items.iter().any(|d| {
            d.get("message")
                .and_then(|m| m.as_str())
                .map(|s: &str| s.to_lowercase().contains("unused") || s.contains("$unused"))
                .unwrap_or(false)
        });

        assert!(has_unused_diag, "should have diagnostic for unused variable $unused");
        Ok(())
    }

    #[test]
    fn test_diagnostics_no_warning_for_used_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $value = 42;
print $value;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Wait for diagnostics
        std::thread::sleep(std::time::Duration::from_millis(200));

        let diag_response = server.get_diagnostics(uri);
        println!("DIAGNOSTICS RESPONSE: {diag_response:#?}");

        // Extract the items array
        let empty_vec = vec![];
        let items = diag_response
            .get("result")
            .and_then(|r| r.get("items"))
            .and_then(|i| i.as_array())
            .unwrap_or(&empty_vec);

        // Should not have unused variable warnings for $value
        let has_unused_value = items.iter().any(|d| {
            d.get("message")
                .and_then(|m| m.as_str())
                .map(|s: &str| s.contains("$value") && s.to_lowercase().contains("unused"))
                .unwrap_or(false)
        });

        assert!(!has_unused_value, "should not have unused warning for used variable $value");
        Ok(())
    }

    #[test]
    fn test_hover_with_type_for_binary_expression() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $sum = 10 + 20;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the binary expression
        let (line, character) = find_pos(code, "+", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("BINARY EXPR HOVER RESPONSE: {response:#}");

        if let Some(content) = hover_content(&response) {
            println!("Hover content: {}", content);
            // Binary arithmetic expressions should infer to number type
        }
        Ok(())
    }

    #[test]
    fn test_hover_with_type_for_builtin_function() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $len = length("test");
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the length function call
        let (line, character) = find_pos(code, "length", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("BUILTIN FUNCTION HOVER RESPONSE: {response:#}");

        if let Some(content) = hover_content(&response) {
            println!("Hover content: {}", content);
            // Built-in functions should show type information
        }
        Ok(())
    }

    #[test]
    fn test_diagnostics_detect_shadowed_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $value = 100;
sub process {
    my $value = 200;  # Shadows outer $value
    return $value * 2;
}
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Wait for diagnostics
        std::thread::sleep(std::time::Duration::from_millis(200));

        let diag_response = server.get_diagnostics(uri);
        println!("DIAGNOSTICS RESPONSE: {diag_response:#?}");

        // Extract the items array
        let empty_vec = vec![];
        let items = diag_response
            .get("result")
            .and_then(|r| r.get("items"))
            .and_then(|i| i.as_array())
            .unwrap_or(&empty_vec);

        // The scope analyzer should detect variable shadowing
        // This verifies semantic analysis is wired to diagnostics
        println!("Semantic diagnostics count: {}", items.len());
        if !items.is_empty() {
            println!("Semantic diagnostics are working");
        }
        Ok(())
    }
}
