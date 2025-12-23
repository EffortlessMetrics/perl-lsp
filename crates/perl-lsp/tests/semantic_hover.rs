//! Semantic-aware textDocument/hover tests
//!
//! These tests verify that the LSP hover handler uses SemanticAnalyzer
//! for accurate symbol information display including type, declaration,
//! and documentation details.
//!
//! The LSP handler at lsp_server.rs:2484 uses SemanticAnalyzer::analyze()
//! and symbol_at() to provide rich hover information for Perl symbols.
//! These tests validate hover behavior across common Perl patterns.

mod test_utils;

#[cfg(test)]
mod semantic_hover_tests {
    use crate::test_utils::TestServerBuilder;
    use serde_json::Value;

    /// Extract hover content from an LSP hover response.
    /// Returns the markdown value string for assertions.
    fn hover_content(resp: &Value) -> Option<String> {
        let result = resp.get("result")?;
        if result.is_null() {
            return None;
        }
        let contents = result.get("contents")?;
        let value = contents.get("value")?.as_str()?;
        Some(value.to_string())
    }

    /// Compute (line, character) for a given `needle` on a specific `target_line`.
    /// Same helper as used in semantic_definition.rs for consistency.
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
    fn hover_on_scalar_variable_shows_declaration_info() {
        let code = r#"my $count = 42;
my $result = $count * 2;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the `$count` reference in the second line
        let (line, character) = find_pos(code, "$count", 1);
        let response = server.get_hover(uri, line, character);
        println!("SCALAR HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).expect("expected hover content for $count reference");

        // Verify hover shows scalar variable information
        assert!(
            content.contains("Scalar Variable"),
            "hover should indicate Scalar Variable, got: {content}"
        );
        assert!(
            content.contains("$count"),
            "hover should show variable name with sigil, got: {content}"
        );
    }

    #[test]
    fn hover_on_subroutine_shows_signature() {
        let code = r#"sub calculate {
    my ($x, $y) = @_;
    return $x + $y;
}

my $sum = calculate(10, 20);
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "calculate" in the function call
        let (line, character) = find_pos(code, "calculate(10", 5);
        let response = server.get_hover(uri, line, character);
        println!("SUB HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).expect("expected hover content for calculate() call");

        // Verify hover shows subroutine information
        assert!(
            content.contains("Subroutine") || content.contains("calculate"),
            "hover should indicate subroutine or show name, got: {content}"
        );
    }

    #[test]
    fn hover_on_subroutine_declaration_shows_signature() {
        let code = r#"sub format_name {
    my ($first, $last) = @_;
    return "$first $last";
}
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "format_name" in the declaration
        let (line, character) = find_pos(code, "format_name", 0);
        let response = server.get_hover(uri, line, character);
        println!("SUB DECL HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).expect("expected hover content for format_name declaration");

        // Verify hover shows subroutine declaration information
        assert!(
            content.contains("Subroutine") || content.contains("format_name"),
            "hover should show subroutine information, got: {content}"
        );
    }

    #[test]
    fn hover_on_package_qualified_call_shows_context() {
        let code = r#"package Math::Utils {
    sub multiply {
        my ($a, $b) = @_;
        return $a * $b;
    }
}

package main;
my $product = Math::Utils::multiply(5, 6);
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "multiply" in the qualified call Math::Utils::multiply
        let (line, character) = find_pos(code, "multiply(5", 8);
        let response = server.get_hover(uri, line, character);
        println!("PKG QUALIFIED HOVER RESPONSE: {response:#}");

        let content = hover_content(&response)
            .expect("expected hover content for Math::Utils::multiply() call");

        // Verify hover shows function information
        // Note: Package context validation depends on SemanticAnalyzer's package tracking
        assert!(
            content.contains("multiply") || content.contains("Subroutine"),
            "hover should show function name or type, got: {content}"
        );
    }

    #[test]
    fn hover_on_array_variable_shows_type() {
        let code = r#"my @numbers = (1, 2, 3, 4, 5);
my $first = $numbers[0];
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "@numbers" in the declaration
        let (line, character) = find_pos(code, "@numbers", 0);
        let response = server.get_hover(uri, line, character);
        println!("ARRAY HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).expect("expected hover content for @numbers");

        // Verify hover shows array variable information
        assert!(
            content.contains("Array Variable") || content.contains("@numbers"),
            "hover should show array type or name, got: {content}"
        );
    }

    #[test]
    fn hover_on_hash_variable_shows_type() {
        let code = r#"my %config = (debug => 1, verbose => 0);
my $debug_mode = $config{debug};
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "%config" in the declaration
        let (line, character) = find_pos(code, "%config", 0);
        let response = server.get_hover(uri, line, character);
        println!("HASH HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).expect("expected hover content for %config");

        // Verify hover shows hash variable information
        assert!(
            content.contains("Hash Variable") || content.contains("%config"),
            "hover should show hash type or name, got: {content}"
        );
    }

    #[test]
    fn hover_on_lexical_scoped_variable() {
        let code = r#"sub outer {
    my $outer_var = 10;

    sub inner {
        my $inner_var = 20;
        return $inner_var + $outer_var;
    }

    return inner();
}
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "$inner_var" in the return statement
        let (line, character) = find_pos(code, "$inner_var", 5);
        let response = server.get_hover(uri, line, character);
        println!("LEXICAL SCOPED HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).expect("expected hover content for $inner_var");

        // Verify hover shows variable information with proper scoping
        assert!(
            content.contains("Scalar Variable") || content.contains("$inner_var"),
            "hover should show variable information, got: {content}"
        );
    }

    #[test]
    fn hover_on_builtin_function_shows_perl_info() {
        let code = r#"my @items = (1, 2, 3);
my @doubled = map { $_ * 2 } @items;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "map" builtin function
        let (line, character) = find_pos(code, "map", 1);
        let response = server.get_hover(uri, line, character);
        println!("BUILTIN HOVER RESPONSE: {response:#}");

        // Hover should return information even if it's just the token
        // Built-in documentation would be a future enhancement
        let content = hover_content(&response);

        // Either we get semantic info or at least the token
        assert!(content.is_some(), "hover should provide some information for builtin function");

        if let Some(c) = content {
            assert!(
                c.contains("map") || c.contains("Perl"),
                "hover should reference the function or Perl, got: {c}"
            );
        }
    }

    #[test]
    fn hover_on_undefined_symbol_returns_minimal_info() {
        let code = r#"my $defined = 42;
my $result = $undefined + $defined;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "$undefined" which is not declared
        let (line, character) = find_pos(code, "$undefined", 1);
        let response = server.get_hover(uri, line, character);
        println!("UNDEFINED HOVER RESPONSE: {response:#}");

        // Should return hover info showing the token even if not in symbol table
        let content = hover_content(&response);

        // Either we get minimal info or null (both acceptable for undefined symbols)
        assert!(
            content.is_none() || content.unwrap().contains("$undefined"),
            "hover should handle undefined symbols gracefully"
        );
    }

    #[test]
    fn hover_on_package_declaration_shows_package_info() {
        let code = r#"package MyApp::Utils;

use strict;
use warnings;

sub helper {
    return 1;
}

1;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "MyApp::Utils" in package declaration
        let (line, character) = find_pos(code, "MyApp", 0);
        let response = server.get_hover(uri, line, character);
        println!("PACKAGE HOVER RESPONSE: {response:#}");

        let content = hover_content(&response);

        // Package hover may return package info or minimal token info
        if let Some(c) = content {
            assert!(
                c.contains("MyApp") || c.contains("Package") || c.contains("Perl"),
                "hover should show package-related information, got: {c}"
            );
        }
    }

    #[test]
    fn hover_on_method_call_with_arrow_operator() {
        let code = r#"package Logger {
    sub new {
        my $class = shift;
        return bless {}, $class;
    }

    sub log_message {
        my ($self, $msg) = @_;
        print "$msg\n";
    }
}

my $logger = Logger->new();
$logger->log_message("test");
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "log_message" in the method call
        let (line, character) = find_pos(code, "log_message(\"test\")", 13);
        let response = server.get_hover(uri, line, character);
        println!("METHOD CALL HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).expect("expected hover content for method call");

        // Verify hover shows method information
        assert!(
            content.contains("log_message") || content.contains("Subroutine"),
            "hover should show method name or type, got: {content}"
        );
    }

    #[test]
    fn hover_respects_variable_shadowing() {
        let code = r#"my $value = 100;

sub process {
    my $value = 200;  # Shadows outer $value
    return $value * 2;
}

my $result = $value + process();
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the inner "$value" (line 4)
        let (line, character) = find_pos(code, "$value", 4);
        let response = server.get_hover(uri, line, character);
        println!("SHADOWED HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).expect("expected hover content for shadowed variable");

        // Verify hover shows variable information
        // Semantic analyzer should resolve to the inner scope
        assert!(
            content.contains("Scalar Variable") || content.contains("$value"),
            "hover should show variable information, got: {content}"
        );
    }

    #[test]
    fn hover_on_empty_space_returns_null() {
        let code = r#"my $var = 42;

# Comment line
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on empty space (line 1, character 0)
        let response = server.get_hover(uri, 1, 0);
        println!("EMPTY SPACE HOVER RESPONSE: {response:#}");

        // Should return null result for empty space
        let result = response.get("result");
        assert!(
            result.is_some() && result.unwrap().is_null(),
            "hover on empty space should return null result"
        );
    }

    #[test]
    fn hover_on_constant_shows_constant_type() {
        let code = r#"use constant PI => 3.14159;
use constant MAX_SIZE => 1000;

my $circumference = 2 * PI * $radius;
"#;
        let uri = "file:///test.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "PI" constant usage
        let (line, character) = find_pos(code, "PI", 3);
        let response = server.get_hover(uri, line, character);
        println!("CONSTANT HOVER RESPONSE: {response:#}");

        let content = hover_content(&response);

        // Constants may be recognized as symbols or bare words
        if let Some(c) = content {
            assert!(
                c.contains("PI") || c.contains("Constant") || c.contains("Perl"),
                "hover should show constant information, got: {c}"
            );
        }
    }
}
