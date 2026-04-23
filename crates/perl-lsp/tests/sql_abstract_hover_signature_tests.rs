//! Tests for SQL::Abstract hover and signature help
//!
//! These tests verify the SQL::Abstract support follows the DBI pattern:
//! - AC2: Hover documentation for SQL::Abstract methods
//! - AC3: Signature help for SQL::Abstract methods
//! - Guard pattern prevents false positives (no `use SQL::Abstract` = no hover/signature help)

mod common;

// =============================================================================
// Hover Tests
// =============================================================================

#[cfg(test)]
mod hover_tests {
    use crate::common::test_utils::{TestServerBuilder, assertions, semantic};
    use serde_json::Value;

    /// Extract hover markdown content from a full JSON-RPC response.
    fn hover_content(resp: &Value) -> Option<String> {
        semantic::hover_content(resp)
    }

    /// Shorthand: open a document and return a hover response at the given
    /// needle position on the target line.
    fn hover_at(
        code: &str,
        uri: &str,
        needle: &str,
        target_line: usize,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);
        let (line, character) = semantic::find_pos(code, needle, target_line);
        Ok(server.get_hover(uri, line, character))
    }

    // ---------------------------------------------------------------------------
    // AC2: Hover Documentation
    // ---------------------------------------------------------------------------

    /// AC2: Given a Perl file with `use SQL::Abstract` and `$sql->select(...)` on a line
    /// When the user hovers over `select`
    /// Then perl-lsp displays hover documentation showing the method signature and description
    #[test]
    fn test_hover_sql_abstract_select_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->select('users', ['name'], { active => 1 });\n";
        // Hover on "select" on line 2
        let resp = hover_at(code, "file:///sql_abstract_hover.pl", "select", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for SQL::Abstract select")?;

        // Must NOT return the generic token fallback
        assert!(
            !content.starts_with("**Perl**: `select`"),
            "hover on SQL::Abstract select must NOT return the generic fallback card, got: {content}"
        );

        // Must show SQL::Abstract-specific documentation
        assert!(
            content.contains("SQL::Abstract")
                || content.contains("SELECT")
                || content.contains("table"),
            "hover on SQL::Abstract select must contain SQL::Abstract-related documentation, got: {content}"
        );

        Ok(())
    }

    /// AC2: Hover on `$sql->insert()` should return SQL::Abstract documentation
    #[test]
    fn test_hover_sql_abstract_insert_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->insert('users', { name => 'Alice' });\n";
        // Hover on "insert" on line 2
        let resp = hover_at(code, "file:///sql_abstract_insert_hover.pl", "insert", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for SQL::Abstract insert")?;

        assert!(
            !content.starts_with("**Perl**: `insert`"),
            "hover on SQL::Abstract insert must NOT return the generic fallback, got: {content}"
        );

        assert!(
            content.contains("SQL::Abstract")
                || content.contains("INSERT")
                || content.contains("table"),
            "hover on SQL::Abstract insert must contain SQL::Abstract-related documentation, got: {content}"
        );

        Ok(())
    }

    /// AC2: Hover on `$sql->update()` should return SQL::Abstract documentation
    #[test]
    fn test_hover_sql_abstract_update_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->update('users', { name => 'Bob' }, { id => 1 });\n";
        // Hover on "update" on line 2
        let resp = hover_at(code, "file:///sql_abstract_update_hover.pl", "update", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for SQL::Abstract update")?;

        assert!(
            !content.starts_with("**Perl**: `update`"),
            "hover on SQL::Abstract update must NOT return the generic fallback, got: {content}"
        );

        assert!(
            content.contains("SQL::Abstract")
                || content.contains("UPDATE")
                || content.contains("table"),
            "hover on SQL::Abstract update must contain SQL::Abstract-related documentation, got: {content}"
        );

        Ok(())
    }

    /// AC2: Hover on `$sql->delete()` should return SQL::Abstract documentation
    #[test]
    fn test_hover_sql_abstract_delete_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->delete('users', { id => 1 });\n";
        // Hover on "delete" on line 2
        let resp = hover_at(code, "file:///sql_abstract_delete_hover.pl", "delete", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for SQL::Abstract delete")?;

        assert!(
            !content.starts_with("**Perl**: `delete`"),
            "hover on SQL::Abstract delete must NOT return the generic fallback, got: {content}"
        );

        assert!(
            content.contains("SQL::Abstract")
                || content.contains("DELETE")
                || content.contains("table"),
            "hover on SQL::Abstract delete must contain SQL::Abstract-related documentation, got: {content}"
        );

        Ok(())
    }

    /// AC2: Hover on `$sql->where()` should return SQL::Abstract documentation
    #[test]
    fn test_hover_sql_abstract_where_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\nmy ($stmt, @bind) = $sql->where({ id => 1, active => 1 });\n";
        // Hover on "where" on line 2
        let resp = hover_at(code, "file:///sql_abstract_where_hover.pl", "where", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for SQL::Abstract where")?;

        assert!(
            !content.starts_with("**Perl**: `where`"),
            "hover on SQL::Abstract where must NOT return the generic fallback, got: {content}"
        );

        assert!(
            content.contains("SQL::Abstract")
                || content.contains("WHERE")
                || content.contains("clause"),
            "hover on SQL::Abstract where must contain SQL::Abstract-related documentation, got: {content}"
        );

        Ok(())
    }

    /// AC2: Hover on `$sql->generate()` should return SQL::Abstract documentation
    #[test]
    fn test_hover_sql_abstract_generate_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\nmy ($stmt, @bind) = $sql->generate('SELECT * FROM users WHERE id = ?', 42);\n";
        // Hover on "generate" on line 2
        let resp = hover_at(code, "file:///sql_abstract_generate_hover.pl", "generate", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for SQL::Abstract generate")?;

        assert!(
            !content.starts_with("**Perl**: `generate`"),
            "hover on SQL::Abstract generate must NOT return the generic fallback, got: {content}"
        );

        assert!(
            content.contains("SQL::Abstract") || content.contains("SQL"),
            "hover on SQL::Abstract generate must contain SQL::Abstract-related documentation, got: {content}"
        );

        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Guard Pattern Tests - Hover should NOT show SQL::Abstract docs without guard
    // ---------------------------------------------------------------------------

    /// Guard pattern: Without `use SQL::Abstract`, hover should NOT show SQL::Abstract documentation
    #[test]
    fn test_hover_without_use_sql_abstract_no_false_positive()
    -> Result<(), Box<dyn std::error::Error>> {
        // No `use SQL::Abstract` — this is some hypothetical object
        let code = "use SomeFramework;\nmy $obj = SomeFramework->new;\n$obj->select('data');\n";
        // Hover on "select" on line 2
        let resp = hover_at(code, "file:///no_sql_abstract_hover.pl", "select", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for select")?;

        // Must NOT show SQL::Abstract documentation
        assert!(
            !content.contains("**SQL::Abstract**"),
            "hover on select without use SQL::Abstract must NOT return SQL::Abstract docs, got: {content}"
        );

        Ok(())
    }
}

// =============================================================================
// Signature Help Tests
// =============================================================================

#[cfg(test)]
mod signature_help_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;

    /// Helper to get signature help at a specific position
    fn signature_help_at(code: &str, uri: &str, line: usize, character: usize) -> Value {
        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);
        server.get_signature_help(uri, line, character)
    }

    // ---------------------------------------------------------------------------
    // AC3: Signature Help
    // ---------------------------------------------------------------------------

    /// AC3: Given a Perl file with `use SQL::Abstract` and `$sql->select(` with cursor inside the parentheses
    /// When the user is typing arguments
    /// Then perl-lsp shows signature help with parameter hints for `$table, $fields?, $where?, $order?`
    #[test]
    fn test_signature_help_sql_abstract_select_returns_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        // The cursor is inside the select() call — after the opening paren
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->select(";
        let resp = signature_help_at(code, "file:///sql_abstract_sig.pl", 2, 14);

        // Should have signature information
        assert!(
            resp.get("signatures").is_some(),
            "signature help should have signatures field, got: {resp:#}"
        );

        let signatures = resp["signatures"].as_array().ok_or("signatures should be an array")?;
        assert!(!signatures.is_empty(), "signatures array should not be empty");

        let first_sig = &signatures[0];
        let label = first_sig["label"].as_str().ok_or("label should be a string")?;

        // Should contain "select" in the signature
        assert!(label.contains("select"), "signature label should contain 'select', got: {label}");

        // Should contain table parameter hint
        assert!(
            label.contains("table") || label.contains("$table"),
            "signature should mention table parameter, got: {label}"
        );

        Ok(())
    }

    /// AC3: Signature help for `$sql->insert()` should show table and values parameters
    #[test]
    fn test_signature_help_sql_abstract_insert_returns_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->insert(";
        let resp = signature_help_at(code, "file:///sql_abstract_insert_sig.pl", 2, 14);

        let signatures = resp["signatures"].as_array().ok_or("signatures should be an array")?;
        assert!(!signatures.is_empty(), "signatures array should not be empty");

        let first_sig = &signatures[0];
        let label = first_sig["label"].as_str().ok_or("label should be a string")?;

        assert!(label.contains("insert"), "signature label should contain 'insert', got: {label}");

        Ok(())
    }

    /// AC3: Signature help for `$sql->update()` should show table, set, and where parameters
    #[test]
    fn test_signature_help_sql_abstract_update_returns_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->update(";
        let resp = signature_help_at(code, "file:///sql_abstract_update_sig.pl", 2, 14);

        let signatures = resp["signatures"].as_array().ok_or("signatures should be an array")?;
        assert!(!signatures.is_empty(), "signatures array should not be empty");

        let first_sig = &signatures[0];
        let label = first_sig["label"].as_str().ok_or("label should be a string")?;

        assert!(label.contains("update"), "signature label should contain 'update', got: {label}");

        Ok(())
    }

    /// AC3: Signature help for `$sql->delete()` should show table and where parameters
    #[test]
    fn test_signature_help_sql_abstract_delete_returns_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\n$sql->delete(";
        let resp = signature_help_at(code, "file:///sql_abstract_delete_sig.pl", 2, 14);

        let signatures = resp["signatures"].as_array().ok_or("signatures should be an array")?;
        assert!(!signatures.is_empty(), "signatures array should not be empty");

        let first_sig = &signatures[0];
        let label = first_sig["label"].as_str().ok_or("label should be a string")?;

        assert!(label.contains("delete"), "signature label should contain 'delete', got: {label}");

        Ok(())
    }

    /// AC3: Signature help for `$sql->where()` should show where parameter
    #[test]
    fn test_signature_help_sql_abstract_where_returns_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        let code =
            "use SQL::Abstract;\nmy $sql = SQL::Abstract->new();\nmy ($stmt, @bind) = $sql->where(";
        let resp = signature_help_at(code, "file:///sql_abstract_where_sig.pl", 2, 32);

        let signatures = resp["signatures"].as_array().ok_or("signatures should be an array")?;
        assert!(!signatures.is_empty(), "signatures array should not be empty");

        let first_sig = &signatures[0];
        let label = first_sig["label"].as_str().ok_or("label should be a string")?;

        assert!(label.contains("where"), "signature label should contain 'where', got: {label}");

        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Guard Pattern Tests - Signature help should NOT show SQL::Abstract without guard
    // ---------------------------------------------------------------------------

    /// Guard pattern: Without `use SQL::Abstract`, signature help should NOT show SQL::Abstract signatures
    #[test]
    fn test_signature_help_without_use_sql_abstract_no_false_positive()
    -> Result<(), Box<dyn std::error::Error>> {
        // No `use SQL::Abstract`
        let code = "use SomeFramework;\nmy $obj = SomeFramework->new;\n$obj->select(";
        let resp = signature_help_at(code, "file:///no_sql_abstract_sig.pl", 2, 14);

        let signatures = resp["signatures"].as_array().ok_or("signatures should be an array")?;
        assert!(!signatures.is_empty(), "signatures array should not be empty");

        let first_sig = &signatures[0];
        let label = first_sig["label"].as_str().ok_or("label should be a string")?;

        // Must NOT show SQL::Abstract signature
        assert!(
            !label.contains("SQL::Abstract"),
            "signature help without use SQL::Abstract must NOT show SQL::Abstract docs, got: {label}"
        );

        Ok(())
    }
}
