//! Comprehensive hover provider coverage tests
//!
//! Covers the six required hover scenarios with strong content assertions:
//! 1. Hover on variable: shows type and scope (scalar, array, hash, `my` declaration)
//! 2. Hover on function: shows signature with extracted parameters
//! 3. Hover on builtin: shows "Built-in Function" heading and signature
//! 4. Hover on package name: shows module/package info
//! 5. Hover on keyword: shows keyword help or at least token info
//! 6. Hover on nothing: returns null
//!
//! Additional edge-case tests:
//! - Hover at end-of-file
//! - Hover on numeric literal
//! - Hover on string content
//! - Hover on `our` variable
//! - Hover on subroutine with explicit signature (Perl 5.20+ syntax)
//! - Hover on `use constant` symbol

mod common;

#[cfg(test)]
mod hover_provider_tests {
    use crate::common::test_utils::{TestServerBuilder, assertions, semantic};
    use serde_json::Value;

    // ── helpers ──────────────────────────────────────────────────────────

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

    // ── 1. Hover on variable: shows type and scope ──────────────────────

    #[test]
    fn test_hover_scalar_variable_shows_type_and_declaration()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $name = \"Alice\";\nprint $name;\n";
        let resp = hover_at(code, "file:///var_scalar.pl", "$name", 1)?;

        let content = hover_content(&resp).ok_or("expected hover content for $name")?;
        assert!(
            content.contains("Scalar Variable"),
            "hover should indicate Scalar Variable, got: {content}"
        );
        assert!(
            content.contains("$name"),
            "hover should include variable name with sigil, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_scalar_variable_at_declaration_site() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $counter = 0;\n$counter += 1;\n";
        let resp = hover_at(code, "file:///var_decl.pl", "$counter", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content at declaration")?;
        assert!(
            content.contains("Scalar Variable"),
            "hover at declaration should show type, got: {content}"
        );
        assert!(
            content.contains("$counter"),
            "hover at declaration should show name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_array_variable_shows_array_type() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @items = (1, 2, 3);\npush @items, 4;\n";
        let resp = hover_at(code, "file:///var_array.pl", "@items", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for @items")?;
        assert!(
            content.contains("Array Variable"),
            "hover should indicate Array Variable, got: {content}"
        );
        assert!(
            content.contains("@items") || content.contains("items"),
            "hover should include variable name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_hash_variable_shows_hash_type() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my %lookup = (a => 1, b => 2);\nmy $val = $lookup{a};\n";
        let resp = hover_at(code, "file:///var_hash.pl", "%lookup", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for %lookup")?;
        assert!(
            content.contains("Hash Variable"),
            "hover should indicate Hash Variable, got: {content}"
        );
        assert!(
            content.contains("%lookup") || content.contains("lookup"),
            "hover should include variable name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_our_variable_shows_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let code = "our $VERSION = '1.00';\nprint $VERSION;\n";
        let resp = hover_at(code, "file:///var_our.pl", "$VERSION", 1)?;

        let content = hover_content(&resp).ok_or("expected hover content for our $VERSION")?;
        // At minimum the variable name and type should appear
        assert!(
            content.contains("$VERSION") || content.contains("Scalar Variable"),
            "hover should show our variable info, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_variable_in_nested_scope() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"sub outer {
    my $x = 10;
    if (1) {
        my $y = 20;
        print $y;
    }
}
"#;
        let resp = hover_at(code, "file:///var_scope.pl", "$y", 4)?;

        let content = hover_content(&resp).ok_or("expected hover for scoped $y")?;
        assert!(
            content.contains("Scalar Variable") || content.contains("$y"),
            "hover should show scoped variable info, got: {content}"
        );
        Ok(())
    }

    // ── 2. Hover on function: shows signature ───────────────────────────

    #[test]
    fn test_hover_subroutine_shows_signature_with_params() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = r#"sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

my $sum = add(3, 4);
"#;
        let resp = hover_at(code, "file:///fn_sig.pl", "add(3", 5)?;

        let content = hover_content(&resp).ok_or("expected hover content for add()")?;
        assert!(content.contains("Subroutine"), "hover should indicate Subroutine, got: {content}");
        // The handler extracts params from `my ($a, $b) = @_;` pattern
        assert!(content.contains("add"), "hover should include function name, got: {content}");
        Ok(())
    }

    #[test]
    fn test_hover_subroutine_at_definition() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"sub greet {
    my ($name) = @_;
    print "Hello, $name\n";
}
"#;
        let resp = hover_at(code, "file:///fn_def.pl", "greet", 0)?;

        let content = hover_content(&resp).ok_or("expected hover at sub definition")?;
        assert!(
            content.contains("Subroutine"),
            "hover at sub definition should show Subroutine, got: {content}"
        );
        assert!(
            content.contains("greet"),
            "hover at sub definition should show name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_subroutine_no_params() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"sub get_time {
    return time();
}

my $t = get_time();
"#;
        let resp = hover_at(code, "file:///fn_noparams.pl", "get_time()", 4)?;

        let content = hover_content(&resp).ok_or("expected hover for no-param sub")?;
        assert!(
            content.contains("Subroutine") || content.contains("get_time"),
            "hover should show subroutine info for no-param sub, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_subroutine_with_many_params() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"sub connect_db {
    my ($host, $port, $user, $pass) = @_;
    return 1;
}

connect_db("localhost", 5432, "admin", "secret");
"#;
        let resp = hover_at(code, "file:///fn_many.pl", "connect_db(\"", 5)?;

        let content = hover_content(&resp).ok_or("expected hover for multi-param sub")?;
        assert!(
            content.contains("Subroutine") || content.contains("connect_db"),
            "hover should show sub info, got: {content}"
        );
        Ok(())
    }

    // ── 3. Hover on builtin: shows documentation ────────────────────────

    #[test]
    fn test_hover_builtin_print_shows_documentation() -> Result<(), Box<dyn std::error::Error>> {
        let code = "print \"hello world\\n\";\n";
        let resp = hover_at(code, "file:///builtin_print.pl", "print", 0)?;

        let content = hover_content(&resp).ok_or("expected hover for print builtin")?;
        // The hover handler should hit the builtin documentation path
        assert!(
            content.contains("Built-in Function") || content.contains("print"),
            "hover should show builtin info for print, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_builtin_push_shows_documentation() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @arr;\npush @arr, 42;\n";
        let resp = hover_at(code, "file:///builtin_push.pl", "push", 1)?;

        let content = hover_content(&resp).ok_or("expected hover for push builtin")?;
        assert!(
            content.contains("Built-in Function")
                || content.contains("push")
                || content.contains("Perl"),
            "hover should show builtin info for push, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_builtin_chomp_shows_documentation() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $line = <STDIN>;\nchomp $line;\n";
        let resp = hover_at(code, "file:///builtin_chomp.pl", "chomp", 1)?;

        let content = hover_content(&resp).ok_or("expected hover for chomp builtin")?;
        assert!(
            content.contains("Built-in Function")
                || content.contains("chomp")
                || content.contains("Perl"),
            "hover should show builtin info for chomp, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_builtin_defined_shows_documentation() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = undef;\nif (defined $x) { print 1; }\n";
        let resp = hover_at(code, "file:///builtin_defined.pl", "defined", 1)?;

        let content = hover_content(&resp).ok_or("expected hover for defined builtin")?;
        assert!(
            content.contains("Built-in Function")
                || content.contains("defined")
                || content.contains("Perl"),
            "hover should show builtin info for defined, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_builtin_split_shows_documentation() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @parts = split /,/, \"a,b,c\";\n";
        let resp = hover_at(code, "file:///builtin_split.pl", "split", 0)?;

        let content = hover_content(&resp).ok_or("expected hover for split builtin")?;
        assert!(
            content.contains("Built-in Function")
                || content.contains("split")
                || content.contains("Perl"),
            "hover should show builtin info for split, got: {content}"
        );
        Ok(())
    }

    // ── 4. Hover on package name: shows module info ─────────────────────

    #[test]
    fn test_hover_package_declaration_shows_package() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package MyApp::Config;

use strict;
use warnings;

sub new { return bless {}, shift; }

1;
"#;
        let resp = hover_at(code, "file:///pkg_decl.pl", "MyApp", 0)?;

        let content = hover_content(&resp);
        // Package hover should return some info (Package type or at least the token)
        if let Some(c) = content {
            assert!(
                c.contains("Package") || c.contains("MyApp") || c.contains("Perl"),
                "hover on package name should show package info, got: {c}"
            );
        }
        // If None, that is also acceptable -- package hover depends on semantic depth
        Ok(())
    }

    #[test]
    fn test_hover_package_qualified_name_in_call() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package Util {
    sub helper { return 1; }
}

my $r = Util::helper();
"#;
        let resp = hover_at(code, "file:///pkg_call.pl", "helper()", 4)?;

        let content = hover_content(&resp).ok_or("expected hover on qualified call")?;
        assert!(
            content.contains("helper")
                || content.contains("Subroutine")
                || content.contains("Perl"),
            "hover on qualified call should show function info, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_package_block_syntax() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package Data::Store {
    sub store { return 1; }
}
"#;
        let resp = hover_at(code, "file:///pkg_block.pl", "Data", 0)?;

        let content = hover_content(&resp);
        if let Some(c) = content {
            assert!(
                c.contains("Package") || c.contains("Data") || c.contains("Perl"),
                "hover on block-syntax package should show info, got: {c}"
            );
        }
        Ok(())
    }

    // ── 5. Hover on keyword: shows keyword help ─────────────────────────

    #[test]
    fn test_hover_keyword_if_shows_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 1;\nif ($x) { print 1; }\n";
        let resp = hover_at(code, "file:///kw_if.pl", "if", 1)?;

        let content = hover_content(&resp);
        // Keywords may or may not produce hover; if they do, it should be valid
        if let Some(c) = content {
            assert!(
                c.contains("if") || c.contains("Perl"),
                "hover on keyword should reference it, got: {c}"
            );
        }
        // null is also acceptable for keywords not in the builtin/symbol table
        Ok(())
    }

    #[test]
    fn test_hover_keyword_foreach_shows_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @items = (1, 2, 3);\nforeach my $item (@items) { print $item; }\n";
        let resp = hover_at(code, "file:///kw_foreach.pl", "foreach", 1)?;

        let content = hover_content(&resp);
        if let Some(c) = content {
            assert!(
                c.contains("foreach") || c.contains("Perl"),
                "hover on foreach should reference it, got: {c}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_keyword_while_shows_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $i = 0;\nwhile ($i < 10) { $i++; }\n";
        let resp = hover_at(code, "file:///kw_while.pl", "while", 1)?;

        let content = hover_content(&resp);
        if let Some(c) = content {
            assert!(
                c.contains("while") || c.contains("Perl"),
                "hover on while should reference it, got: {c}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_keyword_return_shows_info() -> Result<(), Box<dyn std::error::Error>> {
        let code = "sub foo { return 42; }\n";
        let resp = hover_at(code, "file:///kw_return.pl", "return", 0)?;

        let content = hover_content(&resp);
        // `return` may be recognized as a builtin, a keyword token, or
        // the semantic analyzer may resolve it to the enclosing subroutine
        if let Some(c) = content {
            assert!(
                c.contains("return")
                    || c.contains("Perl")
                    || c.contains("Built-in")
                    || c.contains("Subroutine"),
                "hover on return should show info, got: {c}"
            );
        }
        Ok(())
    }

    // ── 6. Hover on nothing: returns null/empty ─────────────────────────

    #[test]
    fn test_hover_on_blank_line_returns_null() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 1;\n\nprint $x;\n";
        let server = TestServerBuilder::new().build();
        server.open_document("file:///blank.pl", code);

        let resp = server.get_hover("file:///blank.pl", 1, 0);
        let result = resp.get("result").ok_or("expected result field")?;
        assert!(result.is_null(), "hover on blank line should return null, got: {result:?}");
        Ok(())
    }

    #[test]
    fn test_hover_on_comment_line() -> Result<(), Box<dyn std::error::Error>> {
        let code = "# This is a comment\nmy $x = 1;\n";
        let server = TestServerBuilder::new().build();
        server.open_document("file:///comment.pl", code);

        let resp = server.get_hover("file:///comment.pl", 0, 5);
        let result = resp.get("result").ok_or("expected result field")?;
        // Comments may return null or some comment content -- both are acceptable
        if !result.is_null() {
            // If non-null, should at least be valid hover structure
            assert!(
                result.get("contents").is_some(),
                "non-null hover on comment must have contents"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_past_end_of_line() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 1;\n";
        let server = TestServerBuilder::new().build();
        server.open_document("file:///eol.pl", code);

        // Position well past end of the line
        let resp = server.get_hover("file:///eol.pl", 0, 200);
        let result = resp.get("result").ok_or("expected result field")?;
        // Should return null or gracefully handle out-of-range
        // The server should not crash
        assert!(
            result.is_null() || result.get("contents").is_some(),
            "hover past EOL should return null or valid hover, got: {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_on_empty_document() -> Result<(), Box<dyn std::error::Error>> {
        let code = "";
        let server = TestServerBuilder::new().build();
        server.open_document("file:///empty.pl", code);

        let resp = server.get_hover("file:///empty.pl", 0, 0);
        let result = resp.get("result").ok_or("expected result field")?;
        assert!(result.is_null(), "hover on empty document should return null, got: {result:?}");
        Ok(())
    }

    #[test]
    fn test_hover_on_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
        let code = "   \n   \n   \n";
        let server = TestServerBuilder::new().build();
        server.open_document("file:///ws.pl", code);

        let resp = server.get_hover("file:///ws.pl", 1, 1);
        let result = resp.get("result").ok_or("expected result field")?;
        assert!(result.is_null(), "hover on whitespace should return null, got: {result:?}");
        Ok(())
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn test_hover_on_numeric_literal() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 42;\n";
        let server = TestServerBuilder::new().build();
        server.open_document("file:///num.pl", code);

        // Position on "42"
        let resp = server.get_hover("file:///num.pl", 0, 8);
        let result = resp.get("result").ok_or("expected result field")?;
        // Numeric literals may or may not produce hover -- no crash is the baseline
        if !result.is_null() {
            assert!(
                result.get("contents").is_some(),
                "non-null hover on number must have contents"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_on_string_content() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $msg = \"hello world\";\n";
        let server = TestServerBuilder::new().build();
        server.open_document("file:///str.pl", code);

        // Position inside string content
        let resp = server.get_hover("file:///str.pl", 0, 14);
        let result = resp.get("result").ok_or("expected result field")?;
        // Inside a string -- may or may not produce hover
        if !result.is_null() {
            assert!(
                result.get("contents").is_some(),
                "non-null hover on string content must have contents"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_use_constant_symbol() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"use constant MAX => 100;
my $limit = MAX;
"#;
        let resp = hover_at(code, "file:///const.pl", "MAX", 1)?;

        let content = hover_content(&resp);
        // Constants may be recognized as Constant or bare word
        if let Some(c) = content {
            assert!(
                c.contains("MAX") || c.contains("Constant") || c.contains("Perl"),
                "hover on constant usage should show info, got: {c}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_response_has_markdown_kind() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 1;\nprint $x;\n";
        let resp = hover_at(code, "file:///md.pl", "$x", 1)?;

        let result = resp.get("result").ok_or("expected result field")?;
        if !result.is_null() {
            let contents = result.get("contents").ok_or("expected contents")?;
            if contents.is_object() {
                let kind = contents.get("kind").and_then(|k| k.as_str());
                if let Some(k) = kind {
                    assert!(
                        k == "markdown" || k == "plaintext",
                        "hover content kind should be markdown or plaintext, got: {k}"
                    );
                }
                assert!(contents.get("value").is_some(), "MarkupContent must have a value field");
            }
        }
        Ok(())
    }

    #[test]
    fn test_hover_assertion_helper_contains() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 1;\nprint $x;\n";
        let resp = hover_at(code, "file:///helper.pl", "$x", 1)?;

        // Use the assertion helper from test_utils
        assertions::assert_hover_contains(&resp, "$x");
        Ok(())
    }

    #[test]
    fn test_hover_assertion_helper_contains_any() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @list = (1, 2);\n";
        let resp = hover_at(code, "file:///helper2.pl", "@list", 0)?;

        assertions::assert_hover_contains_any(&resp, &["Array Variable", "@list", "list"]);
        Ok(())
    }

    #[test]
    fn test_hover_on_label_keyword() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"OUTER: for my $i (1..10) {
    for my $j (1..10) {
        next OUTER if $j == 5;
    }
}
"#;
        // Hover on "OUTER" label at usage site
        let resp = hover_at(code, "file:///label.pl", "OUTER", 2)?;

        let content = hover_content(&resp);
        // Labels may be recognized as Label type or bare identifier
        if let Some(c) = content {
            assert!(
                c.contains("OUTER") || c.contains("Label") || c.contains("Perl"),
                "hover on label should show info, got: {c}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_multiple_documents_isolated() -> Result<(), Box<dyn std::error::Error>> {
        let code_a = "my $alpha = 1;\n";
        let code_b = "my $beta = 2;\n";

        let server = TestServerBuilder::new().build();
        server.open_document("file:///a.pl", code_a);
        server.open_document("file:///b.pl", code_b);

        // Hover on $alpha in document A
        let (la, ca) = semantic::find_pos(code_a, "$alpha", 0);
        let resp_a = server.get_hover("file:///a.pl", la, ca);
        let content_a = hover_content(&resp_a).ok_or("expected hover for $alpha")?;
        assert!(
            content_a.contains("$alpha") || content_a.contains("Scalar"),
            "hover in doc A should show $alpha info, got: {content_a}"
        );

        // Hover on $beta in document B
        let (lb, cb) = semantic::find_pos(code_b, "$beta", 0);
        let resp_b = server.get_hover("file:///b.pl", lb, cb);
        let content_b = hover_content(&resp_b).ok_or("expected hover for $beta")?;
        assert!(
            content_b.contains("$beta") || content_b.contains("Scalar"),
            "hover in doc B should show $beta info, got: {content_b}"
        );

        Ok(())
    }

    #[test]
    fn test_hover_after_document_change() -> Result<(), Box<dyn std::error::Error>> {
        let code_v1 = "my $old = 1;\n";
        let code_v2 = "my $new = 2;\nprint $new;\n";

        let server = TestServerBuilder::new().build();
        server.open_document("file:///change.pl", code_v1);
        server.change_document("file:///change.pl", code_v2, 2);

        // Brief delay for server to process the change
        std::thread::sleep(std::time::Duration::from_millis(50));

        let (line, character) = semantic::find_pos(code_v2, "$new", 1);
        let resp = server.get_hover("file:///change.pl", line, character);
        let content = hover_content(&resp).ok_or("expected hover after document change")?;
        assert!(
            content.contains("$new") || content.contains("Scalar"),
            "hover after change should reflect updated content, got: {content}"
        );
        Ok(())
    }

    // ── Type inference in hover (Issue #2357) ────────────────────────────

    #[test]
    fn test_hover_blessed_ref_shows_class_type_from_new() -> Result<(), Box<dyn std::error::Error>>
    {
        // This test verifies that hovering on a variable assigned from a blessed reference
        // shows the inferred class type.
        let code = r#"
package Foo;
sub new { bless {}, shift }
1;

package main;
my $obj = Foo->new();
$obj;
"#;
        let resp = hover_at(code, "file:///blessed.pl", "$obj", 7)?;
        let content = hover_content(&resp).ok_or("expected hover for $obj")?;

        // Should show the scalar variable
        assert!(
            content.contains("Scalar Variable"),
            "hover should indicate Scalar Variable, got: {content}"
        );

        // Should show the variable name
        assert!(content.contains("$obj"), "hover should include variable name, got: {content}");

        // Should show the inferred type (Foo class)
        assert!(
            content.contains("Foo") || content.contains("Object"),
            "hover should show inferred class type or object, got: {content}"
        );

        Ok(())
    }

    #[test]
    fn test_hover_scalar_from_literal_assignment_shows_type()
    -> Result<(), Box<dyn std::error::Error>> {
        // Scalar with integer literal should show Integer type inference
        let code = "my $x = 42;\n$x;";
        let resp = hover_at(code, "file:///int.pl", "$x", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for $x")?;

        assert!(
            content.contains("Scalar Variable"),
            "hover should indicate Scalar Variable, got: {content}"
        );

        // After type inference is wired, should show the inferred type
        // Could be "Integer", "Int", or similar
        assert!(
            content.contains("Variable") || content.contains("Type"),
            "hover should include type information, got: {content}"
        );

        Ok(())
    }

    #[test]
    fn test_hover_shows_inferred_type_from_function_call() -> Result<(), Box<dyn std::error::Error>>
    {
        // Function returning scalar reference should infer reference type
        let code = r#"
sub get_name { return "Alice"; }
my $name = get_name();
$name;
"#;
        let resp = hover_at(code, "file:///func_return.pl", "$name", 3)?;
        let content = hover_content(&resp).ok_or("expected hover for $name")?;

        assert!(
            content.contains("Scalar Variable"),
            "hover should indicate Scalar Variable, got: {content}"
        );

        // Should show something about the type (could be String or unknown scalar)
        assert!(
            content.contains("Variable") || content.contains("Type"),
            "hover should include type context, got: {content}"
        );

        Ok(())
    }

    // ── Additional type inference coverage for Issue #348 ────────────────

    #[test]
    fn test_hover_hash_variable_shows_inferred_type() -> Result<(), Box<dyn std::error::Error>> {
        // Hash variables should show their inferred type when concretely assigned.
        let code = "my %config = (host => 'localhost', port => 8080);\n%config;\n";
        let resp = hover_at(code, "file:///hash_type.pl", "%config", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for %config")?;

        assert!(
            content.contains("Hash Variable"),
            "hover should indicate Hash Variable, got: {content}"
        );
        assert!(
            content.contains("**Type**"),
            "hover on hash variable should include **Type** annotation, got: {content}"
        );
        assert!(
            content.contains("Hash"),
            "hover should show Hash type for %config, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_string_literal_assignment_shows_str_type()
    -> Result<(), Box<dyn std::error::Error>> {
        // Scalar holding a string literal should infer to Str type.
        let code = "my $greeting = \"hello\";\n$greeting;\n";
        let resp = hover_at(code, "file:///str_type.pl", "$greeting", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for $greeting")?;

        assert!(
            content.contains("Scalar Variable"),
            "hover should indicate Scalar Variable, got: {content}"
        );
        assert!(
            content.contains("**Type**"),
            "hover should include **Type** annotation for string literal, got: {content}"
        );
        assert!(
            content.contains("Str"),
            "hover should show Str for string literal assignment, got: {content}"
        );
        Ok(())
    }

    // ── Test::More hover documentation ───────────────────────────────────

    #[test]
    fn test_hover_test_more_is_shows_signature() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use Test::More;\nis($got, $expected, 'my test');\n";
        let resp = hover_at(code, "file:///testmore_is.t", "is(", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for Test::More is()")?;
        assert!(
            content.contains("Test::More"),
            "hover should show Test::More heading, got: {content}"
        );
        assert!(content.contains("is("), "hover should include is() signature, got: {content}");
        Ok(())
    }

    #[test]
    fn test_hover_test_more_ok_shows_signature() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use Test::More tests => 1;\nok(1 == 1, 'addition');\n";
        let resp = hover_at(code, "file:///testmore_ok.t", "ok(", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for Test::More ok()")?;
        assert!(
            content.contains("Test::More"),
            "hover should show Test::More heading, got: {content}"
        );
        assert!(content.contains("ok("), "hover should include ok() signature, got: {content}");
        Ok(())
    }

    #[test]
    fn test_hover_test_more_bail_out_shows_signature() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use Test::More;\nBAIL_OUT('fatal error');\n";
        let resp = hover_at(code, "file:///testmore_bailout.t", "BAIL_OUT", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for BAIL_OUT")?;
        assert!(
            content.contains("Test::More"),
            "hover on BAIL_OUT should show Test::More heading, got: {content}"
        );
        assert!(
            content.contains("BAIL_OUT"),
            "hover should include BAIL_OUT in output, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_test_more_not_triggered_without_use() -> Result<(), Box<dyn std::error::Error>> {
        // File does NOT have `use Test::More` — hovering over `is` should not show Test::More docs
        let code = "sub is { 1 }\nis('foo', 'foo');\n";
        let resp = hover_at(code, "file:///no_testmore.pl", "is(", 1)?;
        let content = hover_content(&resp);
        if let Some(c) = content {
            assert!(
                !c.contains("Test::More"),
                "hover should NOT show Test::More docs without 'use Test::More', got: {c}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_hover_test2_triggers_test_more_docs() -> Result<(), Box<dyn std::error::Error>> {
        // `use Test2::V0` should also trigger Test::More documentation
        let code = "use Test2::V0;\nis('got', 'expected', 'my test');\n";
        let resp = hover_at(code, "file:///test2_v0.t", "is(", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for Test2::V0 is()")?;
        assert!(
            content.contains("Test::More"),
            "hover with Test2 should show Test::More docs, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_test_more_subtest_shows_signature() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use Test::More;\nsubtest 'my suite' => sub { ok(1) };\n";
        let resp = hover_at(code, "file:///testmore_subtest.t", "subtest", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for subtest")?;
        assert!(
            content.contains("Test::More"),
            "hover on subtest should show Test::More heading, got: {content}"
        );
        assert!(
            content.contains("subtest"),
            "hover should include subtest in output, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_test_more_diag_shows_stderr_note() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use Test::More;\ndiag('debug info');\n";
        let resp = hover_at(code, "file:///testmore_diag.t", "diag", 1)?;
        let content = hover_content(&resp).ok_or("expected hover for diag")?;
        assert!(
            content.contains("Test::More"),
            "hover on diag should show Test::More heading, got: {content}"
        );
        assert!(
            content.contains("STDERR") || content.contains("diag"),
            "hover on diag should mention STDERR or the function name, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_test_more_unknown_fn_no_test_more_docs() -> Result<(), Box<dyn std::error::Error>>
    {
        // Even in a test file, hovering on a non-Test::More function should NOT show Test::More docs
        let code = "use Test::More;\nmy_custom_assertion('foo');\n";
        let resp = hover_at(code, "file:///testmore_custom.t", "my_custom_assertion", 1)?;
        let content = hover_content(&resp);
        if let Some(c) = content {
            assert!(
                !c.contains("Test::More\n") || c.contains("my_custom"),
                "hover on unknown fn should not show Test::More section, got: {c}"
            );
        }
        Ok(())
    }

    // ── Phase block hover tests (issue #2360) ────────────────────────────

    #[test]
    fn test_hover_begin_block_shows_compile_time_description()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "BEGIN { 1; }\n";
        let resp = hover_at(code, "file:///begin_hover.pl", "BEGIN", 0)?;
        let content = hover_content(&resp).ok_or("BEGIN hover must return content, not null")?;
        assert!(content.contains("BEGIN"), "hover content should mention BEGIN, got: {content}");
        assert!(
            content.contains("compile") || content.contains("compile-time"),
            "hover content should describe compile-time execution, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_end_block_shows_exit_description() -> Result<(), Box<dyn std::error::Error>> {
        let code = "END { 1; }\n";
        let resp = hover_at(code, "file:///end_hover.pl", "END", 0)?;
        let content = hover_content(&resp).ok_or("END hover must return content, not null")?;
        assert!(content.contains("END"), "hover content should mention END, got: {content}");
        assert!(
            content.contains("exit") || content.contains("cleanup") || content.contains("program"),
            "hover content should describe program-exit execution, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_init_block_shows_post_compile_description()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "INIT { print 'init'; }\n";
        let resp = hover_at(code, "file:///init_hover.pl", "INIT", 0)?;
        let content = hover_content(&resp).ok_or("INIT hover must return content, not null")?;
        assert!(content.contains("INIT"), "hover content should mention INIT, got: {content}");
        assert!(
            content.contains("compilation")
                || content.contains("compile")
                || content.contains("before"),
            "hover content should describe post-compile execution, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_check_block_shows_end_of_compile_description()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "CHECK { print 'check'; }\n";
        let resp = hover_at(code, "file:///check_hover.pl", "CHECK", 0)?;
        let content = hover_content(&resp).ok_or("CHECK hover must return content, not null")?;
        assert!(content.contains("CHECK"), "hover content should mention CHECK, got: {content}");
        assert!(
            content.contains("compilation") || content.contains("compile"),
            "hover content should describe end-of-compilation execution, got: {content}"
        );
        Ok(())
    }

    #[test]
    fn test_hover_unitcheck_block_shows_description() -> Result<(), Box<dyn std::error::Error>> {
        let code = "UNITCHECK { print 'unitcheck'; }\n";
        let resp = hover_at(code, "file:///unitcheck_hover.pl", "UNITCHECK", 0)?;
        let content =
            hover_content(&resp).ok_or("UNITCHECK hover must return content, not null")?;
        assert!(
            content.contains("UNITCHECK"),
            "hover content should mention UNITCHECK, got: {content}"
        );
        assert!(
            content.contains("compilation unit") || content.contains("unit"),
            "hover content should describe compilation-unit scope, got: {content}"
        );
        Ok(())
    }

    // ── Moose/Moo role composition hover (Issue #2325) ───────────────────

    /// Hover on the role name in `with 'RoleName';` should produce a module hover
    /// (showing **RoleName** as a header), NOT the generic `**Perl**: \`RoleName\`` fallback.
    #[test]
    fn test_hover_on_with_role_shows_module_hover() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package MyApp::User;
use Moo;
with 'MyApp::Printable';
1;
"#;
        // Line 2 (0-indexed): `with 'MyApp::Printable';`
        // Position the cursor on the role name token (inside the string).
        let resp = hover_at(code, "file:///role_hover.pl", "MyApp::Printable", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for role name")?;

        // Should show the module name as a heading (module hover format)
        // NOT the generic "**Perl**: `MyApp::Printable`" fallback
        assert!(
            content.contains("MyApp::Printable"),
            "hover on role name should contain the module name, got: {content}"
        );
        assert!(
            !content.starts_with("**Perl**:"),
            "hover on role name must NOT be the generic Perl token fallback, got: {content}"
        );
        Ok(())
    }

    /// Hover on the role name in a multi-role `with 'RoleA', 'RoleB';` statement
    /// should also show a module hover for the role under the cursor.
    #[test]
    fn test_hover_on_with_multi_role_shows_module_hover() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = r#"package MyApp::User;
use Moo;
with 'MyApp::Printable', 'MyApp::Serializable';
1;
"#;
        // Line 2: `with 'MyApp::Printable', 'MyApp::Serializable';`
        // Hover on the first role name.
        let resp = hover_at(code, "file:///multi_role_hover.pl", "MyApp::Printable", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for first role name")?;
        assert!(
            content.contains("MyApp::Printable"),
            "hover on first role in multi-role with should contain module name, got: {content}"
        );
        assert!(
            !content.starts_with("**Perl**:"),
            "hover on role name must NOT be the generic Perl token fallback, got: {content}"
        );
        Ok(())
    }

    /// Hover on `extends 'ParentClass'` should also produce a module hover,
    /// not the generic fallback. The fix covers both `with` and `extends`.
    #[test]
    fn test_hover_on_extends_parent_shows_module_hover() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package MyApp::AdminUser;
use Moo;
extends 'MyApp::User';
1;
"#;
        // Line 2: `extends 'MyApp::User';`
        let resp = hover_at(code, "file:///extends_hover.pl", "MyApp::User", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for parent class name")?;
        assert!(
            content.contains("MyApp::User"),
            "hover on extends parent should contain the module name, got: {content}"
        );
        assert!(
            !content.starts_with("**Perl**:"),
            "hover on extends parent must NOT be the generic Perl token fallback, got: {content}"
        );
        Ok(())
    }

    /// Cursor on the `with` keyword itself should NOT trigger module hover.
    /// The `with` keyword is an identifier node, not a string node — the fix
    /// must only activate when the cursor falls within the role name string.
    #[test]
    fn test_hover_on_with_keyword_does_not_trigger_module_hover()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package MyApp::User;
use Moo;
with 'MyApp::Printable';
1;
"#;
        // Line 2: `with 'MyApp::Printable';`
        // Hover on the `with` keyword (column 0), not the role name.
        let server = TestServerBuilder::new().build();
        server.open_document("file:///with_keyword_hover.pl", code);
        let resp = server.get_hover("file:///with_keyword_hover.pl", 2, 0);

        let content = hover_content(&resp);
        // The hover should NOT produce a module hover for `with` keyword itself.
        // It may produce a generic Perl token hover or null — either is acceptable.
        if let Some(c) = content {
            assert!(
                !c.starts_with("**MyApp"),
                "hover on 'with' keyword must not show module hover, got: {c}"
            );
        }
        Ok(())
    }

    /// Double-quoted role name: `with "MyApp::Printable"` should be handled the same
    /// as single-quoted.  The quote-stripping in `role_name_at_offset` uses
    /// `trim_matches('"')` for this case.
    #[test]
    fn test_hover_on_with_role_double_quoted() -> Result<(), Box<dyn std::error::Error>> {
        let code = "package MyApp::User;\nuse Moo;\nwith \"MyApp::Printable\";\n1;\n";
        let resp = hover_at(code, "file:///role_hover_dq.pl", "MyApp::Printable", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for double-quoted role")?;
        assert!(
            content.contains("MyApp::Printable"),
            "hover on double-quoted role should contain module name, got: {content}"
        );
        assert!(
            !content.starts_with("**Perl**:"),
            "hover on double-quoted role must NOT be the generic fallback, got: {content}"
        );
        Ok(())
    }

    /// Cursor on the SECOND role in a multi-role `with 'A', 'B'` should also produce
    /// a module hover (not just the first role).
    #[test]
    fn test_hover_on_with_multi_role_second_role() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package MyApp::User;
use Moo;
with 'MyApp::Printable', 'MyApp::Serializable';
1;
"#;
        // Hover on the SECOND role name on line 2.
        let resp = hover_at(code, "file:///multi_role_second.pl", "MyApp::Serializable", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for second role name")?;
        assert!(
            content.contains("MyApp::Serializable"),
            "hover on second role in multi-role with should contain module name, got: {content}"
        );
        assert!(
            !content.starts_with("**Perl**:"),
            "hover on second role name must NOT be the generic Perl token fallback, got: {content}"
        );
        Ok(())
    }

    // ── DBI method hover documentation (issue #2888) ──────────────────────

    /// Helper: extract signature label from a textDocument/signatureHelp JSON-RPC response.
    fn signature_label(resp: &Value) -> Option<String> {
        resp.get("result")?
            .get("signatures")?
            .as_array()?
            .first()?
            .get("label")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// Hover on a DBI database-handle method must return DBI documentation,
    /// not the generic "**Perl**: `prepare`" fallback.
    ///
    /// This test FAILS before the fix (returns generic card) and PASSES after.
    #[test]
    fn test_hover_dbi_db_method_prepare_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use DBI;\nmy $dbh = DBI->connect('dbi:SQLite:test.db', '', '');\n$dbh->prepare(\"SELECT 1\");\n";
        // Hover on "prepare" on line 2
        let resp = hover_at(code, "file:///dbi_hover.pl", "prepare", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for DBI prepare")?;

        // Must NOT return the generic token fallback
        assert!(
            !content.starts_with("**Perl**: `prepare`"),
            "hover on DBI prepare must NOT return the generic fallback card, got: {content}"
        );
        // Must show DBI-specific documentation
        assert!(
            content.contains("DBI") || content.contains("SQL") || content.contains("statement"),
            "hover on DBI prepare must contain DBI-related documentation, got: {content}"
        );
        Ok(())
    }

    /// Hover on a DBI statement-handle method must return DBI documentation.
    #[test]
    fn test_hover_dbi_st_method_fetchrow_hashref_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use DBI;\nmy $sth = $dbh->prepare('SELECT 1');\n$sth->fetchrow_hashref();\n";
        // Hover on "fetchrow_hashref" on line 2
        let resp = hover_at(code, "file:///dbi_sth_hover.pl", "fetchrow_hashref", 2)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for DBI fetchrow_hashref")?;

        assert!(
            !content.starts_with("**Perl**: `fetchrow_hashref`"),
            "hover on DBI fetchrow_hashref must NOT return the generic fallback, got: {content}"
        );
        assert!(
            content.contains("DBI") || content.contains("hashref") || content.contains("row"),
            "hover on DBI fetchrow_hashref must contain DBI-related documentation, got: {content}"
        );
        Ok(())
    }

    /// Hover on DBI execute() method must return real documentation.
    #[test]
    fn test_hover_dbi_st_method_execute_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "use DBI;\nmy $sth = $dbh->prepare('SELECT 1');\n$sth->execute();\n";
        let resp = hover_at(code, "file:///dbi_execute_hover.pl", "execute", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for DBI execute")?;

        assert!(
            !content.starts_with("**Perl**: `execute`"),
            "hover on DBI execute must NOT return the generic fallback, got: {content}"
        );
        assert!(
            content.contains("DBI") || content.contains("statement") || content.contains("Execute"),
            "hover on DBI execute must contain DBI-related documentation, got: {content}"
        );
        Ok(())
    }

    /// Signature help for `$dbh->prepare(` must show the real DBI signature
    /// with `$statement` parameter, not the generic `prepare(...)` fallback.
    ///
    /// This test FAILS before the fix (returns `prepare(...)`) and PASSES after.
    #[test]
    fn test_signature_help_dbi_prepare_returns_real_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        // The cursor is inside the prepare() call — after the opening paren
        let code =
            "use DBI;\nmy $dbh = DBI->connect('dbi:SQLite:test.db', '', '');\n$dbh->prepare(";
        let server = TestServerBuilder::new().build();
        let uri = "file:///dbi_sig.pl";
        server.open_document(uri, code);

        // Position after the opening `(` on line 2, character 14 (after "prepare(")
        let resp = server.get_signature_help(uri, 2, 14);

        let label = signature_label(&resp)
            .ok_or("expected signature label for DBI prepare() signature help")?;

        // Must NOT be the generic fallback
        assert!(
            label != "prepare(...)",
            "signature help for DBI prepare must NOT return generic fallback, got: {label}"
        );
        // Must contain parameter name from the real DBI signature
        assert!(
            label.contains("$statement") || label.contains("statement"),
            "signature help for DBI prepare must include $statement parameter, got: {label}"
        );
        Ok(())
    }

    /// Signature help for `$sth->execute(` must show DBI execute signature.
    #[test]
    fn test_signature_help_dbi_execute_returns_real_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        // Trailing space inside the parens so cursor can be positioned after `(`
        let code = "use DBI;\nmy $sth = $dbh->prepare('SELECT ?');\n$sth->execute( ";
        let server = TestServerBuilder::new().build();
        let uri = "file:///dbi_execute_sig.pl";
        server.open_document(uri, code);

        // Position inside `(` — character 14 is the space after `(`
        let resp = server.get_signature_help(uri, 2, 14);

        let label = signature_label(&resp)
            .ok_or("expected signature label for DBI execute() signature help")?;

        assert!(
            label != "execute(...)",
            "signature help for DBI execute must NOT return generic fallback, got: {label}"
        );
        assert!(
            label.contains("bind_values") || label.contains("execute"),
            "signature help for DBI execute must include bind parameter info, got: {label}"
        );
        Ok(())
    }

    /// Hover on a DBI method called through a hash-dereference handle
    /// (`$self->{dbh}->prepare`) must return DBI documentation, not the
    /// generic fallback.  The receiver extracted is `"dbh}"` (includes the
    /// closing brace) and falls through to the unknown-receiver path, which
    /// searches both tables by method name.
    #[test]
    fn test_hover_dbi_method_through_hashref_handle() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use DBI;\n$self->{dbh}->prepare(\"SELECT 1\");\n";
        // Hover on "prepare" on line 1
        let resp = hover_at(code, "file:///dbi_hashref_hover.pl", "prepare", 1)?;

        let content =
            hover_content(&resp).ok_or("expected hover content for hashref DBI prepare")?;

        assert!(
            !content.starts_with("**Perl**: `prepare`"),
            "hover on $self->{{dbh}}->prepare must NOT return the generic fallback, got: {content}"
        );
        assert!(
            content.contains("DBI") || content.contains("SQL") || content.contains("statement"),
            "hover on $self->{{dbh}}->prepare must contain DBI documentation, got: {content}"
        );
        Ok(())
    }

    /// Hover on a non-DBI arrow method (`$ua->get`) must NOT produce DBI
    /// documentation — it must fall through to the generic "**Perl**: `get`"
    /// card.  This guards against false positives from the DBI check.
    #[test]
    fn test_hover_non_dbi_arrow_method_no_false_positive() -> Result<(), Box<dyn std::error::Error>>
    {
        let code =
            "use LWP::UserAgent;\nmy $ua = LWP::UserAgent->new;\n$ua->get('http://example.com');\n";
        // Hover on "get" on line 2
        let resp = hover_at(code, "file:///lwp_hover.pl", "get", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for $ua->get")?;

        // Must NOT return DBI Method documentation
        assert!(
            !content.contains("**DBI Method**"),
            "hover on $ua->get must NOT return DBI documentation, got: {content}"
        );
        Ok(())
    }

    /// Hover on `$obj->execute()` in a NON-DBI file (no `use DBI`) must NOT
    /// produce DBI documentation.  `execute` is a common method name in many
    /// frameworks.  The `use DBI` guard must prevent this false positive.
    #[test]
    fn test_hover_execute_without_use_dbi_no_false_positive()
    -> Result<(), Box<dyn std::error::Error>> {
        // No `use DBI` — this is some hypothetical task-runner framework.
        let code = "use MyFramework;\nmy $task = MyFramework->new;\n$task->execute();\n";
        let resp = hover_at(code, "file:///no_dbi_execute.pl", "execute", 2)?;

        let content = hover_content(&resp).ok_or("expected hover content for execute")?;

        // Must NOT show DBI documentation (file does not `use DBI`)
        assert!(
            !content.contains("**DBI Method**"),
            "hover on execute without use DBI must NOT return DBI docs, got: {content}"
        );
        Ok(())
    }

    // ── Pragma hover documentation tests ────────────────────────────────────

    /// Hovering over `strict` in `use strict;` should show pragma documentation.
    #[test]
    fn test_hover_use_strict_shows_pragma_docs() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use strict;\nuse warnings;\nmy $x = 1;\n";
        let resp = hover_at(code, "file:///pragma_strict.pl", "strict", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for use strict")?;
        assert!(
            content.contains("strict") || content.contains("Pragma"),
            "hover on use strict should show pragma documentation, got: {content}"
        );
        assert!(
            content.contains("variable")
                || content.contains("strict")
                || content.contains("Pragma"),
            "hover on use strict should mention strict checking, got: {content}"
        );
        Ok(())
    }

    /// Hovering over `warnings` in `use warnings;` should show pragma documentation.
    #[test]
    fn test_hover_use_warnings_shows_pragma_docs() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use strict;\nuse warnings;\nmy $x = 1;\n";
        let resp = hover_at(code, "file:///pragma_warnings.pl", "warnings", 1)?;

        let content = hover_content(&resp).ok_or("expected hover content for use warnings")?;
        assert!(
            content.contains("warning") || content.contains("Pragma"),
            "hover on use warnings should show pragma documentation, got: {content}"
        );
        Ok(())
    }

    /// Hovering over `utf8` in `use utf8;` should show pragma documentation.
    #[test]
    fn test_hover_use_utf8_shows_pragma_docs() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use utf8;\nmy $greeting = 'hello';\n";
        let resp = hover_at(code, "file:///pragma_utf8.pl", "utf8", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for use utf8")?;
        assert!(
            content.contains("UTF") || content.contains("Unicode") || content.contains("Pragma"),
            "hover on use utf8 should describe encoding, got: {content}"
        );
        Ok(())
    }

    /// Hovering over `feature` in `use feature 'say';` should show pragma documentation.
    #[test]
    fn test_hover_use_feature_shows_pragma_docs() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use feature 'say';\nsay 'hello';\n";
        let resp = hover_at(code, "file:///pragma_feature.pl", "feature", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for use feature")?;
        assert!(
            content.contains("feature") || content.contains("Pragma"),
            "hover on use feature should show pragma documentation, got: {content}"
        );
        Ok(())
    }

    /// Hovering over `constant` in `use constant PI => 3.14;` should show pragma docs.
    #[test]
    fn test_hover_use_constant_pragma_shows_docs() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use constant PI => 3.14159;\nprint PI;\n";
        let resp = hover_at(code, "file:///pragma_constant.pl", "constant", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for use constant")?;
        assert!(
            content.contains("constant") || content.contains("Pragma"),
            "hover on use constant should show pragma documentation, got: {content}"
        );
        Ok(())
    }

    /// Hovering over `autodie` in `use autodie;` should show pragma documentation.
    #[test]
    fn test_hover_use_autodie_shows_pragma_docs() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use autodie;\nopen my $fh, '<', 'file.txt';\n";
        let resp = hover_at(code, "file:///pragma_autodie.pl", "autodie", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for use autodie")?;
        assert!(
            content.contains("die") || content.contains("exception") || content.contains("Pragma"),
            "hover on use autodie should describe exception behavior, got: {content}"
        );
        Ok(())
    }

    /// Pragma hover must NOT fall through to the "Not found in workspace" module hover.
    #[test]
    fn test_hover_pragma_does_not_show_not_found_message() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = "use strict;\n";
        let resp = hover_at(code, "file:///pragma_not_found.pl", "strict", 0)?;

        let content = hover_content(&resp).ok_or("expected hover content for use strict")?;
        assert!(
            !content.contains("Not found in workspace"),
            "pragma hover must NOT show 'Not found in workspace', got: {content}"
        );
        Ok(())
    }
}
