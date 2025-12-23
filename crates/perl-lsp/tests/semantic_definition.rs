//! Semantic-aware textDocument/definition tests
//!
//! These tests verify that the LSP definition handler uses SemanticAnalyzer
//! for precise symbol resolution rather than heuristic-based approaches.
//!
//! The LSP handler at lsp_server.rs:3463 already uses SemanticAnalyzer::find_definition().
//! These tests validate that it works correctly for common Perl patterns.

mod test_utils;

#[cfg(test)]
mod semantic_definition_tests {
    use crate::test_utils::TestServerBuilder;
    use serde_json::Value;

    /// Extract the first definition location from an LSP response.
    /// Returns (uri, line, character) for easier assertions.
    fn first_location(resp: &Value) -> Option<(String, u32, u32)> {
        let arr = resp.get("result")?.as_array()?;
        let first = arr.first()?;
        let uri = first.get("uri")?.as_str()?.to_string();
        let range = first.get("range")?;
        let start = &range["start"];
        let line = start.get("line")?.as_u64()? as u32;
        let character = start.get("character")?.as_u64()? as u32;
        Some((uri, line, character))
    }

    /// Compute (line, character) for a given `needle` on a specific `target_line`.
    fn find_pos(code: &str, needle: &str, target_line: usize) -> (u32, u32) {
        let line = code
            .lines()
            .nth(target_line)
            .unwrap_or_else(|| panic!("no line {} in test code", target_line));
        let col = line
            .find(needle)
            .unwrap_or_else(|| panic!("could not find `{needle}` on line {target_line}"));
        (target_line as u32, col as u32)
    }

    #[test]
    fn definition_finds_scalar_variable_declaration() {
        let code = "my $x = 1;\n$x + 2;\n";
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on the `$x` reference in the second line
        let (line, character) = find_pos(code, "$x", 1);
        let response = server.get_definition(uri, line, character);
        println!("SCALAR DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) =
            first_location(&response).expect("no definition found for $x reference");

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "definition for $x should be on line 0");
    }

    #[test]
    fn definition_finds_subroutine_declaration() {
        let code = "sub foo { 1 }\nmy $x = foo();\n";
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on "foo" in the call
        let (line, character) = find_pos(code, "foo()", 1);
        let response = server.get_definition(uri, line, character);
        println!("SUB DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) =
            first_location(&response).expect("no definition found for foo() call");

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 0, "definition for foo should be on line 0");
    }

    #[test]
    fn definition_resolves_scoped_variables() {
        let code = r#"my $outer = 1;
sub foo {
    my $inner = 2;
    return $inner + $outer;
}
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on `$inner` in the return expression
        let (line, character) = find_pos(code, "$inner", 3);
        let response = server.get_definition(uri, line, character);
        println!("SCOPED DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) =
            first_location(&response).expect("no definition found for $inner reference");

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 2, "definition for $inner should be on line 2");
    }

    #[test]
    fn definition_handles_package_qualified_calls() {
        let code = r#"package Foo {
    sub bar { 42 }
}

package main;
Foo::bar();
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Position on "bar" in Foo::bar()
        let (line, character) = find_pos(code, "bar()", 5);
        let response = server.get_definition(uri, line, character);
        println!("PKG DEF RESPONSE: {response:#}");

        let (def_uri, def_line, _def_char) =
            first_location(&response).expect("no definition found for Foo::bar() call");

        assert_eq!(def_uri, uri, "definition should be in same file");
        assert_eq!(def_line, 1, "definition for bar should be on line 1");
    }
}
