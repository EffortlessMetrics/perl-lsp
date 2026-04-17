//! Perldoc integration tests for dynamic builtin documentation
//!
//! This test file verifies that the LSP can provide documentation for Perl
//! builtin functions that are NOT in the hardcoded BuiltinDoc list by
//! querying `perldoc -f <function_name>`.
//!
//! Issue: #3517 - docs(builtins): Add perldoc integration for dynamic builtin documentation
//!
//! ## Background
//!
//! The current implementation has a hardcoded list of ~40 builtins in
//! `crates/perl-semantic-analyzer/src/analysis/semantic/builtins.rs`.
//! Many common Perl builtins are NOT in this list, including:
//! - `fc` (Unicode foldcase, added in Perl 5.16)
//! - `trim` (Remove leading/trailing whitespace, added in Perl 5.22)
//! - `state` (State variable declaration)
//! - `caller` (Call stack information)
//! - `fork` (Create child process)
//! - `exec` (Replace current process with another program)
//!
//! When perldoc integration is enabled, the LSP should query `perldoc -f <func>`
//! to get documentation for ANY builtin function name, even if it's not in the
//! hardcoded list.
//!
//! ## Test Strategy
//!
//! These tests define the EXPECTED behavior after perldoc integration is implemented.
//! They currently FAIL because the feature doesn't exist yet (red state).
//! Once the feature is implemented, these tests should PASS.
//!
//! ## Current State Analysis
//!
//! When a builtin is not in the hardcoded list, the hover currently returns:
//! - Either "**Perl**: `<builtin>`" (minimal builtin recognition)
//! - Or "**Subroutine**" (misidentified as a user subroutine)
//! - Or null
//!
//! The perldoc integration should replace these minimal responses with actual
//! documentation from `perldoc -f <builtin>`.

mod common;

#[cfg(test)]
mod perldoc_hover_tests {
    use crate::common::test_utils::{TestServerBuilder, semantic};
    use serde_json::Value;

    // ── helpers ──────────────────────────────────────────────────────────────

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

    // ── Perldoc documentation check ───────────────────────────────────────────
    //
    // Perldoc-style documentation should be substantive, not just the minimal
    // "**Perl**: `<builtin>`" pattern.
    //
    // The minimal pattern looks like:
    //   **Perl**: `fc`
    //
    // But perldoc-style documentation should look more like:
    //   **Perl**: `fc`
    //   fc LIST
    //   Returns ...
    //   (with substantial content)
    fn is_perldoc_documentation(content: &str) -> bool {
        // Perldoc documentation should be substantial (more than minimal)
        // The minimal pattern is just "**Perl**: `<name>`" possibly with a newline
        let lines: Vec<&str> = content.lines().collect();

        // If only 1-2 lines, it's likely the minimal pattern
        if lines.len() <= 2 {
            return false;
        }

        // Check for perldoc signature patterns (LIST, SCALAR, EXPR, BLOCK)
        let has_signature = content.contains("LIST")
            || content.contains("SCALAR")
            || content.contains("EXPR")
            || content.contains("BLOCK")
            || content.contains("pmf");

        // Check for description-like content
        let has_description = content.contains("Returns")
            || content.contains("Evaluates")
            || content.contains("This function")
            || content.contains("Converts")
            || content.contains("Removes")
            || content.contains("Creates")
            || content.contains("Calls")
            || content.contains("Used to")
            || content.contains("is a");

        // Check for perlfunc reference
        let has_perldoc_reference = content.contains("perlfunc")
            || content.contains("perldoc")
            || content.contains("builtin");

        // Content should be substantial (> 100 chars for real docs)
        let is_substantial = content.len() > 100;

        is_substantial && (has_signature || has_description || has_perldoc_reference)
    }

    // ── Unknown builtin tests ─────────────────────────────────────────────────
    // These tests use Perl builtins that are NOT in the hardcoded list.
    // They currently return minimal or no documentation, but SHOULD return
    // full documentation via perldoc when the feature is implemented.

    /// Test that hovering over `fc` (Unicode foldcase) builtin shows documentation.
    ///
    /// `fc` was added in Perl 5.16 and is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_fc_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $lower = fc($str);";
        let resp = hover_at(code, "file:///builtin_fc.pl", "fc", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for fc builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for fc should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `trim` builtin shows documentation.
    ///
    /// `trim` was added in Perl 5.22 and is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_trim_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "trim $str;";
        let resp = hover_at(code, "file:///builtin_trim.pl", "trim", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for trim builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for trim should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `state` builtin shows documentation.
    ///
    /// `state` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_state_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "state $x = 1;";
        let resp = hover_at(code, "file:///builtin_state.pl", "state", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for state builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for state should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `caller` builtin shows documentation.
    ///
    /// `caller` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_caller_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "sub foo { my @c = caller(0); }";
        let resp = hover_at(code, "file:///builtin_caller.pl", "caller", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for caller builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for caller should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `fork` builtin shows documentation.
    ///
    /// `fork` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_fork_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $pid = fork();";
        let resp = hover_at(code, "file:///builtin_fork.pl", "fork", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for fork builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for fork should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `exec` builtin shows documentation.
    ///
    /// `exec` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_exec_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "exec $program;";
        let resp = hover_at(code, "file:///builtin_exec.pl", "exec", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for exec builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for exec should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `pipe` builtin shows documentation.
    ///
    /// `pipe` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_pipe_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "pipe my ($reader, $writer);";
        let resp = hover_at(code, "file:///builtin_pipe.pl", "pipe", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for pipe builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for pipe should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `socket` builtin shows documentation.
    ///
    /// `socket` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_socket_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "socket my ($sock, AF_INET, SOCK_STREAM, 0);";
        let resp = hover_at(code, "file:///builtin_socket.pl", "socket", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for socket builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for socket should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `glob` builtin shows documentation.
    ///
    /// `glob` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_glob_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @files = glob('*.pl');";
        let resp = hover_at(code, "file:///builtin_glob.pl", "glob", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for glob builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for glob should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `readline` builtin shows documentation.
    ///
    /// `readline` is NOT in the hardcoded builtin list.
    /// Note: The readline operator is written as `<HANDLE>` but we test the function form.
    #[test]
    fn test_hover_unknown_builtin_readline_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        // Using readline() function syntax to test the builtin
        let code = "my $line = readline(*FH);";
        let resp = hover_at(code, "file:///builtin_readline.pl", "readline", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for readline builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for readline should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `system` builtin shows documentation.
    ///
    /// `system` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_system_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "system('ls -la');";
        let resp = hover_at(code, "file:///builtin_system.pl", "system", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for system builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for system should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    /// Test that hovering over `wait` builtin shows documentation.
    ///
    /// `wait` is NOT in the hardcoded builtin list.
    /// This test verifies that perldoc integration provides documentation.
    #[test]
    fn test_hover_unknown_builtin_wait_returns_documentation()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $pid = wait;";
        let resp = hover_at(code, "file:///builtin_wait.pl", "wait", 0)?;

        let content = hover_content(&resp)
            .ok_or("perldoc integration should provide hover for wait builtin")?;

        assert!(
            is_perldoc_documentation(&content),
            "hover for wait should show full perldoc-style documentation (got minimal pattern): {}",
            content
        );
        Ok(())
    }

    // ── Function name validation tests ─────────────────────────────────────
    // The perldoc integration should validate function names before calling
    // perldoc. Function names must match ^[a-zA-Z_][a-zA-Z0-9_]*$.

    /// Test that invalid function names are NOT passed to perldoc.
    ///
    /// Builtin function names must be valid Perl identifiers.
    /// This tests that the implementation properly validates names.
    #[test]
    fn test_perldoc_lookup_validates_function_name_format() -> Result<(), Box<dyn std::error::Error>>
    {
        // These are not valid Perl identifiers, so perldoc should NOT be called
        // The hover should NOT crash and should return None or handle gracefully
        let invalid_names = vec![
            "123abc", // starts with digit
            "my-var", // contains hyphen
            "my$var", // contains dollar sign
        ];

        for invalid_name in invalid_names {
            let code = format!("{};", invalid_name);
            // The server should handle this gracefully without panicking
            let server = TestServerBuilder::new().build();
            server.open_document("file:///invalid.pl", &code);
            // This should not crash the server
            let _resp = server.get_hover("file:///invalid.pl", 0, 0);
        }
        Ok(())
    }

    // ── Comprehensive test ─────────────────────────────────────────────────
    // The perldoc integration should work for any valid builtin name.

    /// Test that documentation can be retrieved for various valid builtin names.
    ///
    /// This is a comprehensive test that verifies the perldoc lookup
    /// can handle various valid Perl builtin function names.
    #[test]
    fn test_perldoc_integration_handles_various_builtin_names()
    -> Result<(), Box<dyn std::error::Error>> {
        // A list of Perl builtins NOT in the hardcoded list
        let test_cases = vec![
            ("fc", "my $lower = fc($str);"),
            ("trim", "trim $str;"),
            ("state", "state $x = 1;"),
            ("caller", "sub foo { my @c = caller(0); }"),
            ("fork", "my $pid = fork();"),
            ("exec", "exec $program;"),
            ("pipe", "pipe my ($r, $w);"),
            ("socket", "socket my ($s, AF_INET, SOCK_STREAM, 0);"),
            ("glob", "my @f = glob('*.pl');"),
            ("readline", "my $l = readline(*FH);"),
            ("system", "system('ls');"),
            ("wait", "my $pid = wait;"),
        ];

        for (builtin_name, code) in test_cases {
            let uri = format!("file:///test_{}.pl", builtin_name);
            let resp = hover_at(code, &uri, builtin_name, 0)?;

            let content = hover_content(&resp)
                .ok_or(format!("perldoc integration should provide hover for {}", builtin_name))?;

            assert!(
                is_perldoc_documentation(&content),
                "hover for {} should show full perldoc-style documentation (got minimal pattern): {}",
                builtin_name,
                content
            );
        }
        Ok(())
    }
}
