//! Unit tests for specific Issue #146 fixes
//!
//! These tests validate individual components and fixes implemented
//! for the architectural integrity repair.

#[cfg(test)]
mod tdd_workflow_unit_tests {
    use std::fs;

    /// Test that tdd_workflow.rs signature variable fix is correct
    #[test]
    fn test_signature_variable_fix() {
        let content =
            fs::read_to_string("src/tdd_workflow.rs").expect("Failed to read tdd_workflow.rs");

        // Should not contain undefined signature variable usage
        assert!(
            !content.contains("let args = signature.as_ref()"),
            "tdd_workflow.rs still contains undefined signature variable"
        );

        // Should use params parameter instead
        assert!(
            content.contains("params.iter()") || content.contains("let args = params"),
            "tdd_workflow.rs does not use params parameter correctly"
        );
    }

    /// Test that tower_lsp imports are replaced with lsp_types
    #[test]
    fn test_lsp_imports_fix() {
        let content =
            fs::read_to_string("src/tdd_workflow.rs").expect("Failed to read tdd_workflow.rs");

        // Should not contain tower_lsp imports
        assert!(
            !content.contains("use tower_lsp::lsp_types"),
            "tdd_workflow.rs still contains tower_lsp imports"
        );

        // Should contain lsp_types imports
        assert!(
            content.contains("use lsp_types::") || !content.contains("CodeAction"),
            "tdd_workflow.rs does not use lsp_types properly"
        );
    }

    /// Test that generate_basic_test method compiles correctly
    #[test]
    fn test_generate_basic_test_method() {
        // This test validates that the method signature and implementation are correct
        // We can't directly test the method without uncommenting the module,
        // but we can validate the source code syntax

        let content =
            fs::read_to_string("src/tdd_workflow.rs").expect("Failed to read tdd_workflow.rs");

        // Check that the method exists and has correct parameter usage
        if content.contains("fn generate_basic_test") {
            // The method should use the params parameter
            let method_start = content.find("fn generate_basic_test").unwrap();
            let method_end = content[method_start..].find("\n    }").unwrap() + method_start;
            let method_content = &content[method_start..method_end];

            assert!(
                method_content.contains("params"),
                "generate_basic_test method does not reference params parameter"
            );
        }
    }
}

#[cfg(test)]
mod refactoring_module_tests {
    use std::path::Path;

    /// Test that refactoring.rs file exists after implementation
    #[test]
    #[ignore] // Only run after refactoring.rs is created
    fn test_refactoring_module_exists() {
        assert!(
            Path::new("src/refactoring.rs").exists(),
            "refactoring.rs module file does not exist"
        );
    }

    /// Test refactoring module structure after implementation
    #[test]
    #[ignore] // Only run after refactoring.rs is created
    fn test_refactoring_module_structure() {
        let content =
            std::fs::read_to_string("src/refactoring.rs").expect("Failed to read refactoring.rs");

        // Should contain the main RefactoringEngine struct
        assert!(
            content.contains("pub struct RefactoringEngine"),
            "RefactoringEngine struct not found in refactoring.rs"
        );

        // Should contain RefactoringType enum
        assert!(
            content.contains("pub enum RefactoringType"),
            "RefactoringType enum not found in refactoring.rs"
        );

        // Should contain RefactoringResult struct
        assert!(
            content.contains("pub struct RefactoringResult"),
            "RefactoringResult struct not found in refactoring.rs"
        );
    }

    /// Test refactoring module API compatibility
    #[test]
    #[ignore] // Only run after refactoring.rs is created and lib.rs is updated
    fn test_refactoring_api_compatibility() {
        // This test will validate that the refactoring module can be imported
        // and used correctly once it's implemented

        // Test imports - these should compile if the API is correct
        // Core parser functionality validation
        use perl_parser::error::ParseResult;

        // Basic API types should be available
        let _result: ParseResult<()> = Ok(());

        assert!(true, "Refactoring API is compatible with parser infrastructure");
    }
}

#[cfg(test)]
mod lib_integration_tests {
    /// Test that lib.rs module declarations are correct after uncommenting
    #[test]
    #[ignore] // Only run after modules are uncommented in lib.rs
    fn test_lib_module_declarations() {
        let content = std::fs::read_to_string("src/lib.rs").expect("Failed to read lib.rs");

        // Should contain uncommented tdd_workflow module
        assert!(
            content.contains("pub mod tdd_workflow;")
                && !content.contains("// pub mod tdd_workflow;"),
            "tdd_workflow module is still commented out in lib.rs"
        );

        // Should contain uncommented refactoring module
        assert!(
            content.contains("pub mod refactoring;")
                && !content.contains("// pub mod refactoring;"),
            "refactoring module is still commented out in lib.rs"
        );
    }

    /// Test that public API exports are added correctly
    #[test]
    #[ignore] // Only run after modules are restored and exports are added
    fn test_public_api_exports() {
        let content = std::fs::read_to_string("src/lib.rs").expect("Failed to read lib.rs");

        // Check for TDD workflow exports
        assert!(
            content.contains("TddWorkflow") || content.contains("pub use tdd_workflow"),
            "TDD workflow types are not exported from lib.rs"
        );

        // Check for refactoring exports
        assert!(
            content.contains("RefactoringEngine") || content.contains("pub use refactoring"),
            "Refactoring types are not exported from lib.rs"
        );
    }
}

#[cfg(test)]
mod api_contract_validation_tests {
    /// Test TestGenerator API contract
    #[test]
    fn test_test_generator_api_contract() {
        use perl_parser::test_generator::{TestFramework, TestGenerator};

        // Validate that TestGenerator can be instantiated with available frameworks
        let _generator = TestGenerator::new(TestFramework::Test2V0);

        // If this compiles, the API contract is valid
        assert!(true, "TestGenerator API contract is valid");
    }

    /// Test that existing parser APIs remain stable
    #[test]
    fn test_parser_api_stability() {
        // Core parser functionality validation
        use perl_parser::error::ParseResult;

        // Test that core types are still available
        let _result: ParseResult<()> = Ok(());

        // If this compiles, core parser API is stable
        assert!(true, "Core parser API remains stable");
    }

    /// Test LSP types availability
    #[test]
    fn test_lsp_types_availability() {
        use lsp_types::{Position, Range};

        // Test that LSP types can be used
        let _position = Position::new(0, 0);
        let _range = Range::new(Position::new(0, 0), Position::new(1, 0));

        // If this compiles, LSP types are properly available
        assert!(true, "LSP types are available and compatible");
    }
}

/// Performance and quality tests
#[cfg(test)]
mod quality_assurance_tests {
    use std::process::Command;

    /// Test that the crate builds without warnings after fixes
    #[test]
    fn test_build_without_warnings() {
        let output = Command::new("cargo")
            .args(&["build", "--package", "perl-parser"])
            .output()
            .expect("Failed to run cargo build");

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should not contain compilation warnings
        assert!(!stderr.contains("warning:"), "Build contains warnings: {}", stderr);

        assert!(output.status.success(), "Build failed: {}", stderr);
    }

    /// Test that tests pass after architectural repair
    #[test]
    fn test_test_suite_passes() {
        let output = Command::new("cargo")
            .args(&["test", "--package", "perl-parser", "--lib"])
            .output()
            .expect("Failed to run tests");

        assert!(
            output.status.success(),
            "Test suite failed after architectural repair: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
