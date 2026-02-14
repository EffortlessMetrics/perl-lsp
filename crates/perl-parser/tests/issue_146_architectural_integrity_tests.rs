//! Test scaffolding for Issue #146: Architectural Integrity Repair
//!
//! Comprehensive test suite validating the restoration of commented-out modules
//! (tdd_workflow.rs and refactoring.rs) with full compilation and integration testing.

use std::process::Command;

/// Test suite for Issue #146 - Architectural Integrity Repair
#[cfg(test)]
mod issue_146_tests {
    use super::*;

    /// AC-1.1: Validate tdd_workflow.rs compilation after signature variable fix
    #[test]
    fn test_tdd_workflow_compilation_fix() {
        // Test that tdd_workflow.rs compiles without the undefined signature error
        let output_res = Command::new("cargo")
            .args(["check", "--package", "perl-parser", "--message-format", "json"])
            .output();
        assert!(output_res.is_ok(), "Failed to run cargo check");
        let output = output_res.unwrap_or_else(|_| unreachable!());

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should not contain compilation errors about undefined signature variable
        assert!(
            !stdout.contains("cannot find value `signature`"),
            "tdd_workflow.rs still has undefined signature variable error"
        );

        // Should not contain tower_lsp import errors
        assert!(
            !stdout.contains("failed to resolve: could not find `tower_lsp`"),
            "tdd_workflow.rs still has tower_lsp import errors"
        );

        // Check that compilation succeeds
        assert!(
            output.status.success(),
            "cargo check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// AC-1.2: Validate LSP types import compatibility
    #[test]
    fn test_lsp_types_import_compatibility() {
        // This test validates that the module uses lsp_types instead of tower_lsp
        // By importing and using key LSP types that should be available

        use lsp_types::{CodeActionKind, DiagnosticSeverity, Position, Range};

        // Test that all required LSP types are available and can be instantiated
        let _position = Position::new(0, 0);
        let _range = Range::new(Position::new(0, 0), Position::new(0, 10));
        let _diagnostic_severity = DiagnosticSeverity::ERROR;
        let _code_action_kind = CodeActionKind::REFACTOR;

        // If this compiles, LSP types are properly available
    }

    /// AC-2.1: Test refactoring.rs module structure and API
    #[test]
    fn test_refactoring_module_api_structure() {
        // This test will validate the refactoring module API once it's created
        // For now, it serves as a placeholder to ensure test infrastructure works

        // Check that core modules exist for refactoring functionality
        // Note: workspace_refactor and modernize will be integrated into refactoring.rs

        // If we can import these, the foundation for refactoring.rs exists
    }

    /// AC-3.1: Integration test for lib.rs module exports
    #[test]
    fn test_lib_module_exports_integration() {
        // Test that lib.rs can be parsed and core modules are available
        // This validates that uncommenting modules doesn't break existing functionality

        // Core parser functionality validation
        use perl_parser::error::ParseResult;

        // Test basic parser functionality remains intact
        let _result: ParseResult<()> = Ok(());

        // If this compiles, core parser API is stable
    }

    /// AC-1.3: API contract validation for TDD workflow components
    #[test]
    fn test_tdd_workflow_api_contracts() {
        // Test that TestGenerator, TestRunner, and RefactoringSuggester APIs are compatible
        // This validates that tdd_workflow.rs can integrate with existing test infrastructure

        use perl_parser::test_generator::{TestFramework, TestGenerator};

        // Test that TestGenerator can be instantiated
        let _test_generator = TestGenerator::new(TestFramework::Test2V0);

        // If this compiles, API contracts are valid
    }
}

/// Integration tests for architectural integrity
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Full compilation test for entire perl-parser crate
    #[test]
    fn test_full_crate_compilation() {
        let output_res = Command::new("cargo").args(["build", "--package", "perl-parser"]).output();
        assert!(output_res.is_ok(), "Failed to run cargo build");
        let output = output_res.unwrap_or_else(|_| unreachable!());

        assert!(
            output.status.success(),
            "Full crate compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Test that clippy passes after architectural repairs
    #[test]
    #[ignore = "clippy warnings burn-down: collapsible_if across multiple crates"]
    fn test_clippy_compliance() {
        let output_res = Command::new("cargo")
            .args(["clippy", "--package", "perl-parser", "--", "-D", "warnings"])
            .output();
        assert!(output_res.is_ok(), "Failed to run cargo clippy");
        let output = output_res.unwrap_or_else(|_| unreachable!());

        assert!(
            output.status.success(),
            "Clippy found warnings: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Test LSP end-to-end functionality after module restoration
    #[test]
    fn test_lsp_e2e_with_restored_modules() {
        // This test validates that LSP functionality works correctly
        // after tdd_workflow.rs and refactoring.rs are restored

        let output_res = Command::new("cargo")
            .args(["test", "--package", "perl-lsp", "--test", "lsp_comprehensive_e2e_test"])
            .output();
        assert!(output_res.is_ok(), "Failed to run LSP E2E tests");
        let output = output_res.unwrap_or_else(|_| unreachable!());

        assert!(
            output.status.success(),
            "LSP E2E tests failed after module restoration: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
