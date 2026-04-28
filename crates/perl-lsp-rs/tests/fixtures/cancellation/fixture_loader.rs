//! Comprehensive fixture loading utilities for LSP cancellation tests
//! Provides structured access to test data for all acceptance criteria validation
//!
//! This module provides centralized fixture loading for the comprehensive LSP
//! cancellation test suite, supporting all test files created by test-creator.

#![allow(unused_imports, dead_code)] // Some imports may not be used yet in scaffolding

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};

/// Path to the cancellation fixtures directory
pub const CANCELLATION_FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cancellation");

/// Comprehensive fixture data structure for all cancellation test scenarios
#[derive(Debug, Clone)]
pub struct CancellationFixtures {
    /// Protocol fixtures for JSON-RPC 2.0 compliance testing
    pub protocol: ProtocolFixtures,
    /// Performance test data for micro-benchmarks and validation
    pub performance: PerformanceFixtures,
    /// Parser integration test data with comprehensive Perl syntax
    pub parser: ParserFixtures,
    /// Workspace integration fixtures for cross-file navigation
    pub workspace: WorkspaceFixtures,
    /// Edge case and error handling test scenarios
    pub edge_cases: EdgeCaseFixtures,
}

/// Protocol fixtures for LSP JSON-RPC 2.0 compliance testing (AC1-AC5)
#[derive(Debug, Clone)]
pub struct ProtocolFixtures {
    /// Valid cancellation request messages
    pub cancel_requests: Value,
    /// Error response fixtures with -32800 codes
    pub error_responses: Value,
    /// Multi-provider coordination scenarios
    pub multi_provider_coordination: Value,
}

/// Performance test fixtures for quantitative validation (AC12)
#[derive(Debug, Clone)]
pub struct PerformanceFixtures {
    /// Micro-benchmark test data for <100μs cancellation checks
    pub micro_benchmark_data: Value,
    /// Memory validation scenarios for <1MB overhead requirement
    pub memory_validation_perl: String,
    /// Threading configuration scenarios for RUST_TEST_THREADS=2 compatibility
    pub threading_scenarios: Value,
}

/// Parser integration fixtures for comprehensive syntax coverage (AC6-AC8)
#[derive(Debug, Clone)]
pub struct ParserFixtures {
    /// Incremental parsing test scenarios with complex Perl constructs
    pub incremental_parsing_perl: String,
    /// Complex syntax patterns for edge case parsing validation
    pub complex_syntax_perl: String,
}

/// Workspace integration fixtures for cross-file navigation testing
#[derive(Debug, Clone)]
pub struct WorkspaceFixtures {
    /// Multi-file project structure for dual indexing validation
    pub multi_file_project_perl: String,
    /// Database module for Package::function resolution testing
    pub database_module_perl: String,
    /// Utils module for bare function call testing
    pub utils_module_perl: String,
}

/// Edge case and error handling fixtures for robustness testing (AC9-AC11)
#[derive(Debug, Clone)]
pub struct EdgeCaseFixtures {
    /// Race condition scenarios for thread safety validation
    pub race_conditions: Value,
    /// Malformed request handling scenarios
    pub malformed_requests: Value,
    /// System recovery scenarios after failures
    pub recovery_scenarios_perl: String,
}

/// Global fixture loader with lazy initialization for efficient test execution
static FIXTURE_LOADER: LazyLock<Arc<Mutex<Option<CancellationFixtures>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

impl CancellationFixtures {
    /// Load all cancellation test fixtures with comprehensive error handling
    ///
    /// This function provides centralized fixture loading for all LSP cancellation
    /// tests, ensuring consistent data access across the test suite.
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Check if fixtures are already loaded (thread-safe singleton pattern)
        {
            let loader = match FIXTURE_LOADER.lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            if let Some(fixtures) = &*loader {
                return Ok(fixtures.clone());
            }
        }

        // Load fixtures from files
        let fixtures = Self::load_from_files()?;

        // Cache loaded fixtures for subsequent calls
        {
            let mut loader = match FIXTURE_LOADER.lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            *loader = Some(fixtures.clone());
        }

        Ok(fixtures)
    }

    /// Internal fixture loading implementation
    fn load_from_files() -> Result<Self, Box<dyn std::error::Error>> {
        let base_path = PathBuf::from(CANCELLATION_FIXTURES_DIR);

        // Load protocol fixtures (JSON files)
        let protocol = ProtocolFixtures {
            cancel_requests: load_json_fixture(&base_path.join("protocol/cancel_requests.json"))?,
            error_responses: load_json_fixture(&base_path.join("protocol/error_responses.json"))?,
            multi_provider_coordination: load_json_fixture(&base_path.join("protocol/multi_provider_coordination.json"))?,
        };

        // Load performance fixtures (mixed JSON and Perl files)
        let performance = PerformanceFixtures {
            micro_benchmark_data: load_json_fixture(&base_path.join("performance/micro_benchmark_data.json"))?,
            memory_validation_perl: load_text_fixture(&base_path.join("performance/memory_validation_data.pl"))?,
            threading_scenarios: load_json_fixture(&base_path.join("performance/threading_scenarios.json"))?,
        };

        // Load parser fixtures (Perl files)
        let parser = ParserFixtures {
            incremental_parsing_perl: load_text_fixture(&base_path.join("parser/incremental_parsing.pl"))?,
            complex_syntax_perl: load_text_fixture(&base_path.join("parser/complex_syntax.pm"))?,
        };

        // Load workspace fixtures (Perl modules)
        let workspace = WorkspaceFixtures {
            multi_file_project_perl: load_text_fixture(&base_path.join("workspace/multi_file_project.pm"))?,
            database_module_perl: load_text_fixture(&base_path.join("workspace/database_module.pm"))?,
            utils_module_perl: load_text_fixture(&base_path.join("workspace/utils_module.pm"))?,
        };

        // Load edge case fixtures (mixed JSON and Perl files)
        let edge_cases = EdgeCaseFixtures {
            race_conditions: load_json_fixture(&base_path.join("edge_cases/race_conditions.json"))?,
            malformed_requests: load_json_fixture(&base_path.join("edge_cases/malformed_requests.json"))?,
            recovery_scenarios_perl: load_text_fixture(&base_path.join("edge_cases/recovery_scenarios.pl"))?,
        };

        Ok(CancellationFixtures {
            protocol,
            performance,
            parser,
            workspace,
            edge_cases,
        })
    }

    /// Get protocol fixtures for AC1-AC5 testing
    pub fn protocol(&self) -> &ProtocolFixtures {
        &self.protocol
    }

    /// Get performance fixtures for AC12 testing
    pub fn performance(&self) -> &PerformanceFixtures {
        &self.performance
    }

    /// Get parser integration fixtures for AC6-AC8 testing
    pub fn parser(&self) -> &ParserFixtures {
        &self.parser
    }

    /// Get workspace integration fixtures for cross-file navigation testing
    pub fn workspace(&self) -> &WorkspaceFixtures {
        &self.workspace
    }

    /// Get edge case fixtures for AC9-AC11 testing
    pub fn edge_cases(&self) -> &EdgeCaseFixtures {
        &self.edge_cases
    }
}

impl ProtocolFixtures {
    /// Get valid cancellation request scenarios for AC1 testing
    pub fn get_valid_cancellation_requests(&self) -> &Value {
        &self.cancel_requests["valid_cancellation_requests"]
    }

    /// Get provider-specific cancellation scenarios for AC3 testing
    pub fn get_provider_specific_requests(&self) -> &Value {
        &self.cancel_requests["provider_specific_requests"]
    }

    /// Get concurrent cancellation scenarios for AC5 testing
    pub fn get_concurrent_cancellation_scenarios(&self) -> &Value {
        &self.cancel_requests["concurrent_cancellation_scenarios"]
    }

    /// Get error response fixtures for AC4 testing
    pub fn get_cancellation_error_responses(&self) -> &Value {
        &self.error_responses["cancellation_error_responses"]
    }

    /// Get multi-provider coordination scenarios for comprehensive testing
    pub fn get_multi_provider_scenarios(&self) -> &Value {
        &self.multi_provider_coordination["multi_provider_scenarios"]
    }

    /// Get thread safety scenarios for AC2 testing
    pub fn get_thread_safety_scenarios(&self) -> &Value {
        &self.multi_provider_coordination["thread_safety_scenarios"]
    }
}

impl PerformanceFixtures {
    /// Get micro-benchmark scenarios for <100μs cancellation check validation
    pub fn get_micro_benchmark_scenarios(&self) -> &Value {
        &self.micro_benchmark_data["micro_benchmark_scenarios"]
    }

    /// Get end-to-end performance scenarios for <50ms response validation
    pub fn get_e2e_performance_scenarios(&self) -> &Value {
        &self.micro_benchmark_data["end_to_end_performance_scenarios"]
    }

    /// Get memory performance scenarios for <1MB overhead validation
    pub fn get_memory_performance_scenarios(&self) -> &Value {
        &self.micro_benchmark_data["memory_performance_scenarios"]
    }

    /// Get threading efficiency scenarios for RUST_TEST_THREADS=2 compatibility
    pub fn get_threading_efficiency_scenarios(&self) -> &Value {
        &self.threading_scenarios["threading_configuration_scenarios"]
    }

    /// Get Perl code for memory validation testing
    pub fn get_memory_validation_perl_code(&self) -> &str {
        &self.memory_validation_perl
    }
}

impl EdgeCaseFixtures {
    /// Get race condition scenarios for thread safety validation
    pub fn get_race_condition_scenarios(&self) -> &Value {
        &self.race_conditions["race_condition_scenarios"]
    }

    /// Get malformed request scenarios for robustness testing
    pub fn get_malformed_request_scenarios(&self) -> &Value {
        &self.malformed_requests["malformed_cancellation_requests"]
    }

    /// Get recovery test scenarios Perl code
    pub fn get_recovery_scenarios_perl_code(&self) -> &str {
        &self.recovery_scenarios_perl
    }
}

/// Load JSON fixture with comprehensive error handling
fn load_json_fixture(path: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read fixture file {}: {}", path.display(), e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON fixture {}: {}", path.display(), e).into())
}

/// Load text fixture (Perl files) with UTF-8 handling
fn load_text_fixture(path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read text fixture {}: {}", path.display(), e).into())
}

/// Specialized fixture accessors for common test patterns

/// Get a specific cancellation request by provider type
pub fn get_cancellation_request_for_provider(provider: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let fixtures = CancellationFixtures::load()?;
    let provider_requests = &fixtures.protocol.cancel_requests["provider_specific_requests"];

    if let Some(request) = provider_requests.get(provider) {
        Ok(request.clone())
    } else {
        Err(format!("No cancellation request fixture found for provider: {}", provider).into())
    }
}

/// Get Perl code for parser integration testing
pub fn get_parser_integration_perl_code(scenario: &str) -> Result<String, Box<dyn std::error::Error>> {
    let fixtures = CancellationFixtures::load()?;

    match scenario {
        "incremental_parsing" => Ok(fixtures.parser.incremental_parsing_perl.clone()),
        "complex_syntax" => Ok(fixtures.parser.complex_syntax_perl.clone()),
        _ => Err(format!("Unknown parser scenario: {}", scenario).into())
    }
}

/// Get workspace module Perl code for cross-file testing
pub fn get_workspace_module_perl_code(module: &str) -> Result<String, Box<dyn std::error::Error>> {
    let fixtures = CancellationFixtures::load()?;

    match module {
        "multi_file_project" => Ok(fixtures.workspace.multi_file_project_perl.clone()),
        "database_module" => Ok(fixtures.workspace.database_module_perl.clone()),
        "utils_module" => Ok(fixtures.workspace.utils_module_perl.clone()),
        _ => Err(format!("Unknown workspace module: {}", module).into())
    }
}

/// Performance measurement utilities for test validation

/// Extract performance thresholds from fixtures
pub fn get_performance_thresholds() -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
    let fixtures = CancellationFixtures::load()?;
    let mut thresholds = HashMap::new();

    // Extract micro-benchmark thresholds
    if let Some(scenarios) = fixtures.performance.micro_benchmark_data["micro_benchmark_scenarios"].as_array() {
        for scenario in scenarios {
            if let Some(requirements) = scenario.get("requirements") {
                if let Some(max_latency) = requirements.get("max_latency_microseconds") {
                    let latency_value = max_latency.as_f64().ok_or("max_latency_microseconds should be a number")?;
                    thresholds.insert("max_cancellation_check_latency_us".to_string(), latency_value);
                }
            }
        }
    }

    // Extract end-to-end thresholds
    if let Some(scenarios) = fixtures.performance.micro_benchmark_data["end_to_end_performance_scenarios"].as_array() {
        for scenario in scenarios {
            if let Some(requirements) = scenario.get("requirements") {
                if let Some(max_response) = requirements.get("max_response_time_ms") {
                    let response_value = max_response.as_f64().ok_or("max_response_time_ms should be a number")?;
                    thresholds.insert("max_e2e_cancellation_response_ms".to_string(), response_value);
                }
            }
        }
    }

    // Extract memory thresholds
    if let Some(scenarios) = fixtures.performance.micro_benchmark_data["memory_performance_scenarios"].as_array() {
        for scenario in scenarios {
            if let Some(requirements) = scenario.get("requirements") {
                if let Some(max_memory) = requirements.get("max_memory_overhead_kb") {
                    let memory_value = max_memory.as_f64().ok_or("max_memory_overhead_kb should be a number")?;
                    thresholds.insert("max_memory_overhead_kb".to_string(), memory_value);
                }
            }
        }
    }

    Ok(thresholds)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_fixture_loading() -> TestResult {
        let fixtures = CancellationFixtures::load()?;

        // Verify protocol fixtures loaded
        assert!(!fixtures.protocol.cancel_requests.is_null());
        assert!(!fixtures.protocol.error_responses.is_null());

        // Verify performance fixtures loaded
        assert!(!fixtures.performance.micro_benchmark_data.is_null());
        assert!(!fixtures.performance.memory_validation_perl.is_empty());

        // Verify parser fixtures loaded
        assert!(!fixtures.parser.incremental_parsing_perl.is_empty());
        assert!(!fixtures.parser.complex_syntax_perl.is_empty());

        // Verify workspace fixtures loaded
        assert!(!fixtures.workspace.multi_file_project_perl.is_empty());

        // Verify edge case fixtures loaded
        assert!(!fixtures.edge_cases.race_conditions.is_null());
        assert!(!fixtures.edge_cases.recovery_scenarios_perl.is_empty());

        Ok(())
    }

    #[test]
    fn test_protocol_fixture_access() -> TestResult {
        let fixtures = CancellationFixtures::load()?;

        let valid_requests = fixtures.protocol.get_valid_cancellation_requests();
        assert!(valid_requests.is_array());

        let error_responses = fixtures.protocol.get_cancellation_error_responses();
        assert!(error_responses.is_array());

        Ok(())
    }

    #[test]
    fn test_performance_threshold_extraction() -> TestResult {
        let thresholds = get_performance_thresholds()?;
        assert!(thresholds.contains_key("max_cancellation_check_latency_us"));
        assert!(thresholds.contains_key("max_e2e_cancellation_response_ms"));
        assert!(thresholds.contains_key("max_memory_overhead_kb"));

        Ok(())
    }

    #[test]
    fn test_provider_specific_fixture_access() -> TestResult {
        let completion_request = get_cancellation_request_for_provider("completion")
            .ok_or("Failed to get completion cancellation request")?;
        assert!(!completion_request.is_null());

        let hover_request = get_cancellation_request_for_provider("hover")
            .ok_or("Failed to get hover cancellation request")?;
        assert!(!hover_request.is_null());

        Ok(())
    }

    #[test]
    fn test_perl_code_fixture_access() -> TestResult {
        let incremental_code = get_parser_integration_perl_code("incremental_parsing")
            .ok_or("Failed to get incremental parsing Perl code")?;
        assert!(incremental_code.contains("IncrementalParsing"));

        let workspace_code = get_workspace_module_perl_code("database_module")
            .ok_or("Failed to get database module Perl code")?;
        assert!(workspace_code.contains("MultiFileProject::Database"));

        Ok(())
    }
}