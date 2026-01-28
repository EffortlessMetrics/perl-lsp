//! Comprehensive LSP cancellation test fixtures module
//!
//! This module provides centralized access to all test fixtures required for
//! comprehensive LSP cancellation testing across all acceptance criteria.
//!
//! ## Fixture Organization
//!
//! The fixtures are organized by test category to support the comprehensive test
//! suite created by test-creator:
//!
//! - **Protocol fixtures**: JSON-RPC 2.0 compliance testing (AC1-AC5)
//! - **Performance fixtures**: Quantitative performance validation (AC12)
//! - **Parser fixtures**: Comprehensive Perl syntax integration (AC6-AC8)
//! - **Workspace fixtures**: Cross-file navigation testing
//! - **Edge case fixtures**: Robustness and error handling (AC9-AC11)
//!
//! ## Usage Examples
//!
//! ```rust,no_run
//! use crate::fixtures::cancellation::{CancellationFixtures, get_cancellation_request_for_provider};
//!
//! // Load all fixtures
//! let fixtures = CancellationFixtures::load().unwrap();
//!
//! // Access protocol fixtures for AC1 testing
//! let valid_requests = fixtures.protocol().get_valid_cancellation_requests();
//!
//! // Access performance fixtures for AC12 testing
//! let micro_benchmarks = fixtures.performance().get_micro_benchmark_scenarios();
//!
//! // Get provider-specific cancellation request
//! let completion_cancel = get_cancellation_request_for_provider("completion").unwrap();
//!
//! // Access Perl code fixtures for parser integration
//! let perl_code = fixtures.parser().incremental_parsing_perl;
//! ```
//!
//! ## Integration with Test Infrastructure
//!
//! These fixtures are designed to work seamlessly with the LSP test infrastructure
//! in `crates/perl-lsp/tests/support/` and provide comprehensive coverage for:
//!
//! - JSON-RPC 2.0 protocol compliance validation
//! - Performance requirements verification (<100μs, <50ms, <1MB)
//! - Thread safety testing with RUST_TEST_THREADS=2 compatibility
//! - Comprehensive Perl syntax parsing scenarios
//! - Cross-file workspace navigation with dual indexing
//! - Edge case and error recovery scenarios
//!
//! ## Crate Organization
//!
//! Fixtures support proper crate-specific testing:
//! - `cargo test -p perl-lsp` - LSP server integration tests
//! - `cargo test -p perl-parser` - Parser component tests
//! - `cargo test -p perl-corpus` - Corpus validation tests
//!
//! ## Thread Safety
//!
//! All fixture loading is thread-safe with lazy initialization patterns,
//! supporting concurrent test execution and CI/CD environments.

pub mod fixture_loader;

// Re-export primary fixtures interface
pub use fixture_loader::{
    CancellationFixtures,
    ProtocolFixtures,
    PerformanceFixtures,
    ParserFixtures,
    WorkspaceFixtures,
    EdgeCaseFixtures,
    CANCELLATION_FIXTURES_DIR,
};

// Re-export convenience functions
pub use fixture_loader::{
    get_cancellation_request_for_provider,
    get_parser_integration_perl_code,
    get_workspace_module_perl_code,
    get_performance_thresholds,
};

/// Test utility functions for fixture validation and setup
pub mod test_utils {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    /// Validate that all required fixtures are present and well-formed
    pub fn validate_fixture_completeness() -> Result<(), Box<dyn std::error::Error>> {
        let fixtures = CancellationFixtures::load()?;

        // Validate protocol fixtures
        validate_protocol_fixtures(&fixtures)?;

        // Validate performance fixtures
        validate_performance_fixtures(&fixtures)?;

        // Validate parser fixtures
        validate_parser_fixtures(&fixtures)?;

        // Validate workspace fixtures
        validate_workspace_fixtures(&fixtures)?;

        // Validate edge case fixtures
        validate_edge_case_fixtures(&fixtures)?;

        Ok(())
    }

    /// Extract test scenario counts for coverage validation
    pub fn get_fixture_coverage_stats() -> Result<HashMap<String, usize>, Box<dyn std::error::Error>> {
        let fixtures = CancellationFixtures::load()?;
        let mut stats = HashMap::new();

        // Count protocol scenarios
        stats.insert("valid_cancellation_requests".to_string(),
                    count_array_elements(&fixtures.protocol.cancel_requests, "valid_cancellation_requests"));

        stats.insert("error_response_scenarios".to_string(),
                    count_array_elements(&fixtures.protocol.error_responses, "cancellation_error_responses"));

        stats.insert("concurrent_scenarios".to_string(),
                    count_array_elements(&fixtures.protocol.cancel_requests, "concurrent_cancellation_scenarios"));

        // Count performance scenarios
        stats.insert("micro_benchmark_scenarios".to_string(),
                    count_array_elements(&fixtures.performance.micro_benchmark_data, "micro_benchmark_scenarios"));

        stats.insert("threading_scenarios".to_string(),
                    count_array_elements(&fixtures.performance.threading_scenarios, "threading_configuration_scenarios"));

        // Count edge case scenarios
        stats.insert("race_condition_scenarios".to_string(),
                    count_array_elements(&fixtures.edge_cases.race_conditions, "race_condition_scenarios"));

        stats.insert("malformed_request_categories".to_string(),
                    count_array_elements(&fixtures.edge_cases.malformed_requests, "malformed_cancellation_requests"));

        Ok(stats)
    }

    /// Generate test workspace files from fixtures
    pub fn create_test_workspace_from_fixtures() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let fixtures = CancellationFixtures::load()?;
        let mut workspace_files = HashMap::new();

        // Add parser integration files
        workspace_files.insert(
            "file:///test/incremental_parsing.pl".to_string(),
            fixtures.parser.incremental_parsing_perl.clone()
        );

        workspace_files.insert(
            "file:///test/complex_syntax.pm".to_string(),
            fixtures.parser.complex_syntax_perl.clone()
        );

        // Add workspace module files
        workspace_files.insert(
            "file:///lib/MultiFileProject/Core.pm".to_string(),
            fixtures.workspace.multi_file_project_perl.clone()
        );

        workspace_files.insert(
            "file:///lib/MultiFileProject/Database.pm".to_string(),
            fixtures.workspace.database_module_perl.clone()
        );

        workspace_files.insert(
            "file:///lib/MultiFileProject/Utils.pm".to_string(),
            fixtures.workspace.utils_module_perl.clone()
        );

        Ok(workspace_files)
    }

    /// Helper function to validate protocol fixtures structure
    fn validate_protocol_fixtures(fixtures: &CancellationFixtures) -> Result<(), Box<dyn std::error::Error>> {
        // Validate cancel_requests structure
        require_array(&fixtures.protocol.cancel_requests, "valid_cancellation_requests")?;
        require_object(&fixtures.protocol.cancel_requests, "provider_specific_requests")?;

        // Validate error_responses structure
        require_array(&fixtures.protocol.error_responses, "cancellation_error_responses")?;

        // Validate multi_provider_coordination structure
        require_array(&fixtures.protocol.multi_provider_coordination, "multi_provider_scenarios")?;

        Ok(())
    }

    /// Helper function to validate performance fixtures structure
    fn validate_performance_fixtures(fixtures: &CancellationFixtures) -> Result<(), Box<dyn std::error::Error>> {
        // Validate micro_benchmark_data structure
        require_array(&fixtures.performance.micro_benchmark_data, "micro_benchmark_scenarios")?;
        require_array(&fixtures.performance.micro_benchmark_data, "end_to_end_performance_scenarios")?;

        // Validate memory_validation_perl content
        if fixtures.performance.memory_validation_perl.is_empty() {
            return Err("Memory validation Perl code is empty".into());
        }

        // Validate threading_scenarios structure
        require_array(&fixtures.performance.threading_scenarios, "threading_configuration_scenarios")?;

        Ok(())
    }

    /// Helper function to validate parser fixtures structure
    fn validate_parser_fixtures(fixtures: &CancellationFixtures) -> Result<(), Box<dyn std::error::Error>> {
        if fixtures.parser.incremental_parsing_perl.is_empty() {
            return Err("Incremental parsing Perl code is empty".into());
        }

        if fixtures.parser.complex_syntax_perl.is_empty() {
            return Err("Complex syntax Perl code is empty".into());
        }

        // Validate Perl code contains expected patterns
        if !fixtures.parser.incremental_parsing_perl.contains("IncrementalParsing") {
            return Err("Incremental parsing Perl code missing expected package declaration".into());
        }

        if !fixtures.parser.complex_syntax_perl.contains("ComplexSyntax") {
            return Err("Complex syntax Perl code missing expected package declaration".into());
        }

        Ok(())
    }

    /// Helper function to validate workspace fixtures structure
    fn validate_workspace_fixtures(fixtures: &CancellationFixtures) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_files = [
            (&fixtures.workspace.multi_file_project_perl, "MultiFileProject::Core"),
            (&fixtures.workspace.database_module_perl, "MultiFileProject::Database"),
            (&fixtures.workspace.utils_module_perl, "MultiFileProject::Utils"),
        ];

        for (content, expected_package) in workspace_files {
            if content.is_empty() {
                return Err(format!("Workspace file for {} is empty", expected_package).into());
            }

            if !content.contains(expected_package) {
                return Err(format!("Workspace file missing expected package: {}", expected_package).into());
            }
        }

        Ok(())
    }

    /// Helper function to validate edge case fixtures structure
    fn validate_edge_case_fixtures(fixtures: &CancellationFixtures) -> Result<(), Box<dyn std::error::Error>> {
        // Validate race_conditions structure
        require_array(&fixtures.edge_cases.race_conditions, "race_condition_scenarios")?;

        // Validate malformed_requests structure
        require_array(&fixtures.edge_cases.malformed_requests, "malformed_cancellation_requests")?;

        // Validate recovery_scenarios_perl content
        if fixtures.edge_cases.recovery_scenarios_perl.is_empty() {
            return Err("Recovery scenarios Perl code is empty".into());
        }

        if !fixtures.edge_cases.recovery_scenarios_perl.contains("RecoveryScenarios") {
            return Err("Recovery scenarios Perl code missing expected package declaration".into());
        }

        Ok(())
    }

    /// Helper function to require array field exists
    fn require_array(value: &Value, field: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(field_value) = value.get(field) {
            if field_value.is_array() {
                Ok(())
            } else {
                Err(format!("Field '{}' is not an array", field).into())
            }
        } else {
            Err(format!("Required field '{}' not found", field).into())
        }
    }

    /// Helper function to require object field exists
    fn require_object(value: &Value, field: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(field_value) = value.get(field) {
            if field_value.is_object() {
                Ok(())
            } else {
                Err(format!("Field '{}' is not an object", field).into())
            }
        } else {
            Err(format!("Required field '{}' not found", field).into())
        }
    }

    /// Helper function to count array elements
    fn count_array_elements(value: &Value, field: &str) -> usize {
        value.get(field)
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_fixture_completeness_validation() -> TestResult {
        validate_fixture_completeness()?;
        Ok(())
    }

    #[test]
    fn test_fixture_coverage_stats() -> TestResult {
        let stats = get_fixture_coverage_stats()?;

        // Verify we have reasonable coverage
        assert!(stats["valid_cancellation_requests"] > 0, "No valid cancellation request fixtures");
        assert!(stats["error_response_scenarios"] > 0, "No error response fixtures");
        assert!(stats["micro_benchmark_scenarios"] > 0, "No micro-benchmark fixtures");
        assert!(stats["race_condition_scenarios"] > 0, "No race condition fixtures");

        println!("Fixture coverage stats: {:?}", stats);
        Ok(())
    }

    #[test]
    fn test_workspace_creation_from_fixtures() -> TestResult {
        let workspace = create_test_workspace_from_fixtures()?;

        assert!(workspace.len() >= 5, "Test workspace should have at least 5 files");

        // Verify key workspace files are present
        assert!(workspace.contains_key("file:///test/incremental_parsing.pl"));
        assert!(workspace.contains_key("file:///lib/MultiFileProject/Core.pm"));
        assert!(workspace.contains_key("file:///lib/MultiFileProject/Database.pm"));
        Ok(())
    }

    #[test]
    fn test_performance_thresholds_extraction() -> TestResult {
        let thresholds = get_performance_thresholds()?;

        // Verify critical performance thresholds are present
        assert!(thresholds.contains_key("max_cancellation_check_latency_us"));
        assert!(thresholds.contains_key("max_e2e_cancellation_response_ms"));
        assert!(thresholds.contains_key("max_memory_overhead_kb"));

        // Verify threshold values are reasonable
        assert!(thresholds["max_cancellation_check_latency_us"] <= 100.0);
        assert!(thresholds["max_e2e_cancellation_response_ms"] <= 50.0);
        assert!(thresholds["max_memory_overhead_kb"] <= 1024.0);
        Ok(())
    }
}