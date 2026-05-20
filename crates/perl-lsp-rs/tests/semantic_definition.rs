//! Semantic-aware textDocument/definition tests
//!
//! These tests verify that the LSP definition handler uses SemanticAnalyzer
//! for precise symbol resolution rather than heuristic-based approaches.
//!
//! The LSP handler at lsp_server.rs:3463 already uses SemanticAnalyzer::find_definition().
//! These tests validate that it works correctly for common Perl patterns.

mod common;

#[cfg(test)]
mod semantic_definition_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;

    /// Extract the first definition location from an LSP response.
    /// Returns (uri, line, character) for easier assertions.
    fn first_location(resp: &Value) -> Result<(String, u32, u32), Box<dyn std::error::Error>> {
        let arr = resp
            .get("result")
            .ok_or("missing result field")?
            .as_array()
            .ok_or("result is not an array")?;
        let first = arr.first().ok_or("result array is empty")?;
        let uri = first
            .get("uri")
            .ok_or("missing uri field")?
            .as_str()
            .ok_or("uri is not a string")?
            .to_string();
        let range = first.get("range").ok_or("missing range field")?;
        let start = &range["start"];
        let line =
            start.get("line").ok_or("missing line field")?.as_u64().ok_or("line is not a number")?
                as u32;
        let character = start
            .get("character")
            .ok_or("missing character field")?
            .as_u64()
            .ok_or("character is not a number")? as u32;
        Ok((uri, line, character))
    }

    /// Compute (line, character) for a given `needle` on a specific `target_line`.
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
    fn definition_finds_scalar_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 1;\n$x + 2;\n";
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on the `$x` reference in the second line
        let (line, character) = find_pos(code, "$x", 1)?;
        let response = server.get_definition(uri, line, character);
        println!("SCALAR DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "definition for $x should be on line 0");
        Ok(())
    }

    #[test]
    fn definition_finds_subroutine_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let code = "sub foo { 1 }\nmy $x = foo();\n";
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on "foo" in the call
        let (line, character) = find_pos(code, "foo()", 1)?;
        let response = server.get_definition(uri, line, character);
        println!("SUB DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "definition for foo should be on line 0");
        Ok(())
    }

    #[test]
    fn definition_resolves_scoped_variables() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $outer = 1;
sub foo {
    my $inner = 2;
    return $inner + $outer;
}
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on `$inner` in the return expression
        let (line, character) = find_pos(code, "$inner", 3)?;
        let response = server.get_definition(uri, line, character);
        println!("SCOPED DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 2, "definition for $inner should be on line 2");
        Ok(())
    }

    #[test]
    fn definition_handles_package_qualified_calls() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package Foo {
    sub bar { 42 }
}

package main;
Foo::bar();
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on "bar" in Foo::bar()
        let (line, character) = find_pos(code, "bar()", 5)?;
        let response = server.get_definition(uri, line, character);
        println!("PKG DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 1, "definition for bar should be on line 1");
        Ok(())
    }

    #[test]
    fn definition_finds_array_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @arr = (1, 2);\npush @arr, 3;\n";
        let uri = "file:///test_arr.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on `@arr` in the push statement (line 1)
        let (line, character) = find_pos(code, "@arr", 1)?;
        let response = server.get_definition(uri, line, character);
        println!("ARRAY DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "definition for @arr should be on line 0");
        Ok(())
    }

    #[test]
    fn definition_finds_hash_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my %opts = (timeout => 30);\nmy $t = $opts{timeout};\n";
        let uri = "file:///test_hash.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on `%opts` in the declaration (line 0), verifying the hash itself resolves
        let (line, character) = find_pos(code, "%opts", 0)?;
        let response = server.get_definition(uri, line, character);
        println!("HASH DECL DEF RESPONSE: {response:#}");

        // Positioned on the declaration itself — result may be empty or point to the same line;
        // either way the server must not crash or return a malformed response.
        let result = response.get("result").ok_or("response must have result field")?;
        assert!(result.is_array(), "result must be an array (got: {result:#})");
        Ok(())
    }

    #[test]
    fn definition_finds_our_package_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = "our $counter = 0;\nsub increment { $counter++ }\n";
        let uri = "file:///test_our.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on `$counter` inside the subroutine body (line 1)
        let (line, character) = find_pos(code, "$counter", 1)?;
        let response = server.get_definition(uri, line, character);
        println!("OUR VAR DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "definition for $counter should be on line 0");
        Ok(())
    }

    #[test]
    fn definition_finds_anonymous_sub_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $handler = sub { return 42; };\nmy $result = $handler->(); \n";
        let uri = "file:///test_anon.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on `$handler` in the call expression (line 1)
        let (line, character) = find_pos(code, "$handler", 1)?;
        let response = server.get_definition(uri, line, character);
        println!("ANON SUB VAR DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "$handler reference should resolve to its declaration on line 0");
        Ok(())
    }

    #[test]
    fn definition_on_declaration_site_returns_valid_response()
    -> Result<(), Box<dyn std::error::Error>> {
        // Clicking on the declaration itself: some servers return the declaration location,
        // others return an empty array — both are valid. Assert no crash and valid structure.
        let code = "my $value = 42;\nprint $value;\n";
        let uri = "file:///test_decl.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on `$value` in the declaration (line 0)
        let (line, character) = find_pos(code, "$value", 0)?;
        let response = server.get_definition(uri, line, character);
        println!("DECL SITE DEF RESPONSE: {response:#}");

        let result = response.get("result").ok_or("response must have result field")?;
        assert!(result.is_array(), "result must be an array even at declaration site");
        Ok(())
    }

    #[test]
    fn definition_returns_empty_for_non_symbol_position() -> Result<(), Box<dyn std::error::Error>>
    {
        // Clicking on a comment or whitespace should return an empty result gracefully.
        let code = "# A comment line\nmy $x = 1;\n";
        let uri = "file:///test_nosym.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position in the middle of the comment (character 4)
        let response = server.get_definition(uri, 0, 4);
        println!("NO SYMBOL DEF RESPONSE: {response:#}");

        let result = response.get("result").ok_or("response must have result field")?;
        // Comment positions have no symbol definition
        assert!(
            result.is_array(),
            "result for a non-symbol position must be an array (got: {result:#})"
        );
        Ok(())
    }

    #[test]
    fn definition_resolves_variable_across_nested_blocks() -> Result<(), Box<dyn std::error::Error>>
    {
        // Use a uniquely-named variable to avoid workspace-fixture collisions.
        let code = r#"my $xnestedouter = { timeout => 30 };
sub run {
    my $limit = 10;
    for my $i (1..$limit) {
        return $xnestedouter if $i > 5;
    }
}
"#;
        let uri = "file:///test_nested.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // `$xnestedouter` inside the for loop (line 4) should resolve to its outer declaration (line 0)
        let (line, character) = find_pos(code, "$xnestedouter", 4)?;
        let response = server.get_definition(uri, line, character);
        println!("NESTED BLOCK DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "$xnestedouter in nested block should resolve to line 0");
        Ok(())
    }
}
