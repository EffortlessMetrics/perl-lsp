//! Semantic-aware textDocument/hover tests
//!
//! These tests verify that the LSP hover handler uses SemanticAnalyzer
//! for accurate symbol information display including type, declaration,
//! and documentation details.
//!
//! The LSP handler at lsp_server.rs:2484 uses SemanticAnalyzer::analyze()
//! and symbol_at() to provide rich hover information for Perl symbols.
//! These tests validate hover behavior across common Perl patterns.

mod common;

#[cfg(test)]
mod semantic_hover_tests {
    use crate::common::test_utils::TestServerBuilder;
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
    fn hover_on_scalar_variable_shows_declaration_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $count = 42;
my $result = $count * 2;
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the `$count` reference in the second line
        let (line, character) = find_pos(code, "$count", 1)?;
        let response = server.get_hover(uri, line, character);
        println!("SCALAR HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).ok_or("expected hover content for $count reference")?;

        // Verify hover shows scalar variable information
        assert!(
            content.contains("Scalar Variable"),
            "hover should indicate Scalar Variable, got: {content}"
        );
        assert!(
            content.contains("$count"),
            "hover should show variable name with sigil, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_subroutine_shows_signature() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"sub calculate {
    my ($x, $y) = @_;
    return $x + $y;
}

my $sum = calculate(10, 20);
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "calculate" in the function call
        let (line, character) = find_pos(code, "calculate(10", 5)?;
        let response = server.get_hover(uri, line, character);
        println!("SUB HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).ok_or("expected hover content for calculate() call")?;

        // Verify hover shows subroutine information
        assert!(
            content.contains("Subroutine") || content.contains("calculate"),
            "hover should indicate subroutine or show name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_subroutine_declaration_shows_signature() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"sub format_name {
    my ($first, $last) = @_;
    return "$first $last";
}
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "format_name" in the declaration
        let (line, character) = find_pos(code, "format_name", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("SUB DECL HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).ok_or("expected hover content for format_name declaration")?;

        // Verify hover shows subroutine declaration information
        assert!(
            content.contains("Subroutine") || content.contains("format_name"),
            "hover should show subroutine information, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_package_qualified_call_shows_context() -> Result<(), Box<dyn std::error::Error>> {
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

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "multiply" in the qualified call Math::Utils::multiply
        let (line, character) = find_pos(code, "multiply(5", 8)?;
        let response = server.get_hover(uri, line, character);
        println!("PKG QUALIFIED HOVER RESPONSE: {response:#}");

        let content = hover_content(&response)
            .ok_or("expected hover content for Math::Utils::multiply() call")?;

        // Verify hover shows function information
        // Note: Package context validation depends on SemanticAnalyzer's package tracking
        assert!(
            content.contains("multiply") || content.contains("Subroutine"),
            "hover should show function name or type, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_array_variable_shows_type() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my @numbers = (1, 2, 3, 4, 5);
my $first = $numbers[0];
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "@numbers" in the declaration
        let (line, character) = find_pos(code, "@numbers", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("ARRAY HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content for @numbers")?;

        // Verify hover shows array variable information
        assert!(
            content.contains("Array Variable") || content.contains("@numbers"),
            "hover should show array type or name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_hash_variable_shows_type() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my %config = (debug => 1, verbose => 0);
my $debug_mode = $config{debug};
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "%config" in the declaration
        let (line, character) = find_pos(code, "%config", 0)?;
        let response = server.get_hover(uri, line, character);
        println!("HASH HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content for %config")?;

        // Verify hover shows hash variable information
        assert!(
            content.contains("Hash Variable") || content.contains("%config"),
            "hover should show hash type or name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_lexical_scoped_variable() -> Result<(), Box<dyn std::error::Error>> {
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

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "$inner_var" in the return statement
        let (line, character) = find_pos(code, "$inner_var", 5)?;
        let response = server.get_hover(uri, line, character);
        println!("LEXICAL SCOPED HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content for $inner_var")?;

        // Verify hover shows variable information with proper scoping
        assert!(
            content.contains("Scalar Variable") || content.contains("$inner_var"),
            "hover should show variable information, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_builtin_function_shows_perl_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my @items = (1, 2, 3);
my @doubled = map { $_ * 2 } @items;
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "map" builtin function
        let (line, character) = find_pos(code, "map", 1)?;
        let response = server.get_hover(uri, line, character);
        println!("BUILTIN HOVER RESPONSE: {response:#}");

        // Hover should return information even if it's just the token
        // Built-in documentation would be a future enhancement
        let content = hover_content(&response);

        // Either we get semantic info or at least the token
        assert!(
            content.is_some(),
            "hover should provide some information for builtin function"
        );

        if let Some(c) = content {
            assert!(
                c.contains("map") || c.contains("Perl"),
                "hover should reference the function or Perl, got: {c}"
            );
        }
        Ok(())
    }

    #[test]
    fn hover_on_undefined_symbol_returns_minimal_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $defined = 42;
my $result = $undefined + $defined;
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "$undefined" which is not declared
        let (line, character) = find_pos(code, "$undefined", 1)?;
        let response = server.get_hover(uri, line, character);
        println!("UNDEFINED HOVER RESPONSE: {response:#}");

        // Should return hover info showing the token even if not in symbol table
        let content = hover_content(&response);

        // Either we get minimal info or null (both acceptable for undefined symbols)
        assert!(
            content.is_none() || content.as_ref().is_some_and(|c| c.contains("$undefined")),
            "hover should handle undefined symbols gracefully"
        );
        Ok(())
    }

    #[test]
    fn hover_on_package_declaration_shows_package_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package MyApp::Utils;

use strict;
use warnings;

sub helper {
    return 1;
}

1;
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "MyApp::Utils" in package declaration
        let (line, character) = find_pos(code, "MyApp", 0)?;
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
        Ok(())
    }

    #[test]
    fn hover_on_method_call_with_arrow_operator() -> Result<(), Box<dyn std::error::Error>> {
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

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "log_message" in the method call
        let (line, character) = find_pos(code, "log_message(\"test\")", 13)?;
        let response = server.get_hover(uri, line, character);
        println!("METHOD CALL HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content for method call")?;

        // Verify hover shows method information
        assert!(
            content.contains("log_message") || content.contains("Subroutine"),
            "hover should show method name or type, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_respects_variable_shadowing() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $value = 100;

sub process {
    my $value = 200;  # Shadows outer $value
    return $value * 2;
}

my $result = $value + process();
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on the inner "$value" (line 4)
        let (line, character) = find_pos(code, "$value", 4)?;
        let response = server.get_hover(uri, line, character);
        println!("SHADOWED HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).ok_or("expected hover content for shadowed variable")?;

        // Verify hover shows variable information
        // Semantic analyzer should resolve to the inner scope
        assert!(
            content.contains("Scalar Variable") || content.contains("$value"),
            "hover should show variable information, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn hover_on_empty_space_returns_null() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $var = 42;

# Comment line
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on empty space (line 1, character 0)
        let response = server.get_hover(uri, 1, 0);
        println!("EMPTY SPACE HOVER RESPONSE: {response:#}");

        // Should return null result for empty space
        let result = response
            .get("result")
            .ok_or("expected result field in hover response")?;
        assert!(
            result.is_null(),
            "hover on empty space should return null result"
        );
        Ok(())
    }

    #[test]
    fn hover_on_constant_shows_constant_type() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"use constant PI => 3.14159;
use constant MAX_SIZE => 1000;

my $circumference = 2 * PI * $radius;
"#;
        let uri = "file:///test.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover on "PI" constant usage
        let (line, character) = find_pos(code, "PI", 3)?;
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
        Ok(())
    }
}

#[cfg(test)]
mod tied_variable_hover_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;

    fn hover_content(resp: &Value) -> Option<String> {
        let result = resp.get("result")?;
        if result.is_null() {
            return None;
        }
        let contents = result.get("contents")?;
        let value = contents.get("value")?.as_str()?;
        Some(value.to_string())
    }

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

    /// Hover on a tied scalar variable at usage site should mention the tied class.
    #[test]
    fn test_hover_on_tied_variable_shows_class() -> Result<(), Box<dyn std::error::Error>> {
        let code = "tie my $counter, 'Tie::Counter';\nmy $x = $counter;\n";
        let uri = "file:///test_tied.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover over $counter on line 1 (the usage site)
        let (line, character) = find_pos(code, "$counter", 1)?;
        let response = server.get_hover(uri, line, character);
        println!("TIED SCALAR HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content for tied $counter")?;

        assert!(
            content.contains("Tie::Counter"),
            "hover should mention tied class 'Tie::Counter', got: {content}"
        );
        Ok(())
    }

    /// Hover on a tied variable when the class is given as a runtime variable should
    /// not panic and should gracefully degrade (no class shown is acceptable).
    #[test]
    fn test_hover_on_tied_variable_unknown_class_does_not_panic()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $cls = 'Tie::Counter';\ntie my $x, $cls;\nmy $y = $x;\n";
        let uri = "file:///test_tied_dynamic.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Hover over $x on line 2 (the usage site) — must not panic
        let (line, character) = find_pos(code, "$x", 2)?;
        let response = server.get_hover(uri, line, character);
        println!("TIED DYNAMIC CLASS HOVER RESPONSE: {response:#}");

        // Must not panic; result may be null or generic — both acceptable
        let _ = hover_content(&response);
        Ok(())
    }
}

#[cfg(test)]
mod method_modifier_hover_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;

    fn hover_content(resp: &Value) -> Option<String> {
        let result = resp.get("result")?;
        if result.is_null() {
            return None;
        }
        let contents = result.get("contents")?;
        let value = contents.get("value")?.as_str()?;
        Some(value.to_string())
    }

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

    /// Hovering over the method name inside `before 'save' => sub { ... }`
    /// should show "Method Modifier" rather than the generic "Subroutine" label.
    #[test]
    fn hover_on_before_modifier_shows_method_modifier_kind()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = include_str!("fixtures/frameworks/moo_method_modifiers.pl");
        let uri = "file:///moo_modifiers.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Line 8 (0-indexed) is: before 'save' => sub {
        let (line, col) = find_pos(code, "save", 8)?;
        let response = server.get_hover(uri, line, col);
        println!("BEFORE MODIFIER HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).ok_or("expected hover content on before modifier")?;

        assert!(
            content.to_lowercase().contains("modifier") || content.contains("before"),
            "hover should mention modifier kind, got: {content}"
        );
        assert!(
            !content.contains("**Subroutine**"),
            "hover should not show generic Subroutine label for a modifier, got: {content}"
        );
        Ok(())
    }

    /// Hovering over the method name inside `around 'save' => sub { ... }`
    /// should mention "around" and show `$orig` usage semantics.
    /// The improved hover card (after fix) will show "Method Modifier (`around`)"
    /// and explain that around receives `$orig` as the first arg.
    #[test]
    fn hover_on_around_modifier_mentions_orig() -> Result<(), Box<dyn std::error::Error>> {
        let code = include_str!("fixtures/frameworks/moo_method_modifiers.pl");
        let uri = "file:///moo_modifiers_around.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Line 18 (0-indexed) is: around 'save' => sub {
        let (line, col) = find_pos(code, "save", 18)?;
        let response = server.get_hover(uri, line, col);
        println!("AROUND MODIFIER HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).ok_or("expected hover content on around modifier")?;

        // After fix: hover should mention $orig (the key semantic of around modifiers).
        // Before fix: shows generic "**Subroutine**" with no $orig guidance.
        assert!(
            content.contains("orig"),
            "around modifier hover should explain $orig usage, got: {content}"
        );
        assert!(
            !content.contains("**Subroutine**"),
            "hover should not show generic Subroutine label for around modifier, got: {content}"
        );
        Ok(())
    }

    /// Hovering over the method name inside `after 'save' => sub { ... }`
    /// should name the "after" modifier and not show the generic Subroutine label.
    #[test]
    fn hover_on_after_modifier_is_distinct_from_subroutine()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = include_str!("fixtures/frameworks/moo_method_modifiers.pl");
        let uri = "file:///moo_modifiers_after.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Line 13 (0-indexed) is: after 'save' => sub {
        let (line, col) = find_pos(code, "save", 13)?;
        let response = server.get_hover(uri, line, col);
        println!("AFTER MODIFIER HOVER RESPONSE: {response:#}");

        let content = hover_content(&response).ok_or("expected hover content on after modifier")?;

        assert!(
            content.contains("after") || content.contains("modifier"),
            "after modifier hover should name the modifier, got: {content}"
        );
        assert!(
            !content.contains("**Subroutine**"),
            "hover should not show generic Subroutine label for a modifier, got: {content}"
        );
        Ok(())
    }

    /// Hovering over the method name inside `override 'render' => sub { ... }`
    /// should identify the override modifier and avoid the generic Subroutine label.
    #[test]
    fn hover_on_override_modifier_mentions_override() -> Result<(), Box<dyn std::error::Error>> {
        let code =
            "package MyApp::User;\nuse Moose;\nsub render { }\noverride 'render' => sub { };\n";
        let uri = "file:///moo_modifiers_override.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        let (line, col) = find_pos(code, "render", 3)?;
        let response = server.get_hover(uri, line, col);

        let content =
            hover_content(&response).ok_or("expected hover content on override modifier")?;

        assert!(
            content.contains("override") || content.contains("modifier"),
            "override modifier hover should name the modifier, got: {content}"
        );
        assert!(
            !content.contains("**Subroutine**"),
            "hover should not show generic Subroutine label for an override modifier, got: {content}"
        );
        Ok(())
    }

    /// Hovering over the method name inside `augment 'render' => sub { ... }`
    /// should identify the augment modifier and avoid the generic Subroutine label.
    #[test]
    fn hover_on_augment_modifier_mentions_augment() -> Result<(), Box<dyn std::error::Error>> {
        let code =
            "package MyApp::User;\nuse Moose;\nsub render { }\naugment 'render' => sub { };\n";
        let uri = "file:///moo_modifiers_augment.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        let (line, col) = find_pos(code, "render", 3)?;
        let response = server.get_hover(uri, line, col);

        let content =
            hover_content(&response).ok_or("expected hover content on augment modifier")?;

        assert!(
            content.contains("augment") || content.contains("modifier"),
            "augment modifier hover should name the modifier, got: {content}"
        );
        assert!(
            !content.contains("**Subroutine**"),
            "hover should not show generic Subroutine label for an augment modifier, got: {content}"
        );
        Ok(())
    }

    /// `use Mouse;` packages should get modifier symbols just like `use Moo;`.
    /// This verifies that Mouse is recognized in symbol.rs framework detection.
    /// Without the fix, Mouse is not in `update_framework_context`, so `is_moo`
    /// is never set and the modifier symbol is never added — hover falls through
    /// to the generic token fallback showing "**Perl**: `save`" with no modifier info.
    #[test]
    fn mouse_modifier_emits_symbol_with_modifier_attribute()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "package MyApp::User;\nuse Mouse;\nbefore 'save' => sub { };\n";
        let uri = "file:///mouse_modifier.pl";

        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        // Line 2 (0-indexed) is: before 'save' => sub { };
        let (line, col) = find_pos(code, "save", 2)?;
        let response = server.get_hover(uri, line, col);
        println!("MOUSE MODIFIER HOVER RESPONSE: {response:#}");

        let content =
            hover_content(&response).ok_or("expected hover content on Mouse before modifier")?;

        // Must be a semantic hover (not just token fallback "**Perl**: `save`").
        // The semantic hover will have Declaration/Attributes fields from the modifier symbol.
        assert!(
            content.contains("before") || content.contains("modifier"),
            "Mouse before modifier should produce semantic hover mentioning the modifier, got: {content}"
        );
        assert!(
            !content.contains("**Perl**"),
            "hover should be semantic, not the generic token fallback, got: {content}"
        );
        Ok(())
    }
}

#[cfg(test)]
mod module_hover_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;
    use std::fs;

    fn hover_content(resp: &Value) -> Option<String> {
        let result = resp.get("result")?;
        if result.is_null() {
            return None;
        }
        let contents = result.get("contents")?;
        let value = contents.get("value")?.as_str()?;
        Some(value.to_string())
    }

    #[test]
    fn hover_on_use_statement_module_found() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_dir = workspace.join("lib").join("My");
        fs::create_dir_all(&module_dir)?;
        fs::write(module_dir.join("Module.pm"), "package My::Module; 1;")?;

        let workspace_path = workspace.to_str().ok_or("non-UTF-8 workspace path")?;
        let server = TestServerBuilder::new()
            .with_workspace(workspace_path)
            .build();

        let code = "use My::Module;\nmy $x = 1;\n";
        let uri = "file:///test.pl";
        server.open_document(uri, code);

        // Hover on "My::Module" (line 0, on the module name)
        let response = server.get_hover(uri, 0, 5);
        let content = hover_content(&response).ok_or("expected hover content for use statement")?;

        assert!(
            content.contains("My::Module"),
            "hover should show module name, got: {content}"
        );
        assert!(
            content.contains("Module.pm") && content.contains("Go to module"),
            "hover should show resolved path for found module, got: {content}"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn hover_on_use_statement_honors_absolute_perl5lib_outside_workspace()
    -> Result<(), Box<dyn std::error::Error>> {
        let original_perl5lib = std::env::var_os("PERL5LIB");

        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let external_lib = temp.path().join("external").join("lib");
        let module_dir = external_lib.join("External");
        fs::create_dir_all(&module_dir)?;
        fs::write(module_dir.join("Tool.pm"), "package External::Tool; 1;")?;

        unsafe {
            std::env::set_var("PERL5LIB", &external_lib);
        }

        let workspace_path = workspace.to_str().ok_or("non-UTF-8 workspace path")?;
        let server = TestServerBuilder::new()
            .with_workspace(workspace_path)
            .build();

        let code = "use External::Tool;\nmy $x = 1;\n";
        let uri = "file:///test.pl";
        server.open_document(uri, code);

        let response = server.get_hover(uri, 0, 5);
        let content = hover_content(&response).ok_or("expected hover content for use statement")?;

        assert!(
            content.contains("External::Tool"),
            "hover should show module name, got: {content}"
        );
        assert!(
            content.contains("Tool.pm") && content.contains("Go to module"),
            "hover should show resolved path for PERL5LIB module, got: {content}"
        );

        match original_perl5lib {
            Some(value) => unsafe { std::env::set_var("PERL5LIB", value) },
            None => unsafe { std::env::remove_var("PERL5LIB") },
        }
        Ok(())
    }

    #[test]
    fn hover_on_use_statement_module_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let server = TestServerBuilder::new().build();

        let code = "use Nonexistent::Module;\nmy $x = 1;\n";
        let uri = "file:///test.pl";
        server.open_document(uri, code);

        // Hover on "Nonexistent::Module"
        let response = server.get_hover(uri, 0, 5);
        let content = hover_content(&response).ok_or("expected hover content for use statement")?;

        assert!(
            content.contains("Nonexistent::Module"),
            "hover should show module name, got: {content}"
        );
        assert!(
            content.contains("Not found"),
            "hover should indicate module not found, got: {content}"
        );
        assert!(
            content.contains("Search paths"),
            "hover should show search paths, got: {content}"
        );
        Ok(())
    }
}
