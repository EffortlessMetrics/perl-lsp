//! Tests for Perl 5.38+ native class hover support.
//!
//! Covers bugs 1 and 2 from issue #2344:
//! - Bug 1: find_subroutine_definition drops NodeKind::Method and NodeKind::Class
//! - Bug 2: hover param extraction misses NodeKind::Method signature
//!
//! After the fix, hovering on a native `method` name should:
//! - Show "Method" kind string (not fall through to generic "Symbol")
//! - Show "method foo($params)" format (not "sub foo($params)")
//! - Extract parameters from the method signature

mod common;

#[cfg(test)]
mod native_class_hover_tests {
    use crate::common::test_utils::{TestServerBuilder, semantic};

    // ── helpers ──────────────────────────────────────────────────────────

    fn hover_at(
        code: &str,
        uri: &str,
        needle: &str,
        target_line: usize,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);
        let (line, character) = semantic::find_pos(code, needle, target_line);
        Ok(server.get_hover(uri, line, character))
    }

    // ── Bug 1 + Bug 2: hover on native method shows "method" prefix ──────

    #[test]
    fn test_hover_on_native_method_shows_method_keyword() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = "class Foo {\n    method bar { return 1; }\n}\nmy $f = Foo->new;\n$f->bar;\n";
        // Hover on "bar" in the method declaration on line 1 (0-indexed)
        let resp = hover_at(code, "file:///native_class.pl", "bar", 1)?;

        let content = semantic::hover_content(&resp)
            .ok_or("expected hover content for native method 'bar'")?;
        assert!(
            content.contains("method"),
            "hover on native method should show 'method' keyword, got: {content}"
        );
        assert!(
            !content.contains("sub bar"),
            "hover on native method should NOT show 'sub bar', got: {content}"
        );
        assert!(content.contains("bar"), "hover should include method name, got: {content}");
        Ok(())
    }

    #[test]
    fn test_hover_on_native_method_with_signature_extracts_params()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "class Calculator {\n    method add($x, $y) { return $x + $y; }\n}\n";
        // Hover on "add" on line 1 (the declaration)
        let resp = hover_at(code, "file:///calculator.pl", "add", 1)?;

        let content =
            semantic::hover_content(&resp).ok_or("expected hover content for method 'add'")?;
        assert!(content.contains("add"), "hover should include method name 'add', got: {content}");
        // Parameters should be extracted from the method signature
        assert!(
            content.contains("$x") || content.contains("$y"),
            "hover should show parameters from method signature, got: {content}"
        );
        Ok(())
    }

    // ── Bug 1: find_subroutine_definition recurses into Class body ────────

    #[test]
    fn test_hover_finds_method_inside_class_block() -> Result<(), Box<dyn std::error::Error>> {
        // Without the Class recursion fix, find_subroutine_definition never
        // enters the Class body and returns None, so the hover display falls
        // back to generic "Symbol" instead of showing method-specific info.
        // After the fix, it should show "method greet" (not "sub greet").
        let code = "class Greeter {\n    method greet { return \"hello\"; }\n}\n";
        let resp = hover_at(code, "file:///greeter.pl", "greet", 1)?;

        let content = semantic::hover_content(&resp)
            .ok_or("expected hover content for method 'greet' inside class body")?;

        // After the fix, the display name must use "method" prefix, not "sub"
        assert!(
            content.contains("method greet"),
            "hover should show 'method greet' (not 'sub greet'), got: {content}"
        );
        Ok(())
    }
}
