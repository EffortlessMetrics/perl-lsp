//! Comprehensive Perl LSP test fixtures module
//!
//! Central fixture loading infrastructure for Perl LSP executeCommand implementation testing.
//! Provides unified access to all fixture types with proper crate organization and
//! deterministic loading patterns for reproducible test execution.
//!
//! Features:
//! - Comprehensive Perl syntax fixtures with ~100% language coverage
//! - LSP protocol mock data with JSON-RPC compliance testing
//! - Dual indexing validation corpus with 98% reference coverage
//! - Performance benchmark data supporting <1ms parsing requirements
//! - Thread-safe fixture loading with adaptive threading support
//! - Unicode-safe test data with UTF-8/UTF-16 boundary validation

// Module declarations for fixture categories
pub mod parser {
    pub mod comprehensive_syntax_fixtures;
    pub mod modern_perl_syntax_fixtures;
}

pub mod lsp {
    pub mod protocol_mock_data;
    pub mod concurrent_request_fixtures;
}

pub mod corpus {
    pub mod dual_indexing_corpus;
    pub mod property_based_testing_fixtures;
}

pub mod builtins {
    pub mod enhanced_builtin_fixtures;
}

pub mod substitution {
    pub mod comprehensive_substitution_fixtures;
}

pub mod incremental {
    pub mod node_reuse_fixtures;
    pub mod performance_validation_fixtures;
}

pub mod integration {
    pub mod lsp_workflow_fixtures;
}

pub mod security {
    pub mod security_validation_fixtures;
}

pub mod mocks {
    pub mod test_infrastructure_mocks;
}

// Re-export all fixture types for easy access
pub use parser::comprehensive_syntax_fixtures::*;
pub use parser::modern_perl_syntax_fixtures::*;
pub use lsp::protocol_mock_data::*;
pub use lsp::concurrent_request_fixtures::*;
pub use corpus::dual_indexing_corpus::*;
pub use corpus::property_based_testing_fixtures::*;
pub use builtins::enhanced_builtin_fixtures::*;
pub use substitution::comprehensive_substitution_fixtures::*;
pub use incremental::node_reuse_fixtures::*;
pub use incremental::performance_validation_fixtures::*;
pub use integration::lsp_workflow_fixtures::*;
pub use security::security_validation_fixtures::*;
pub use mocks::test_infrastructure_mocks::*;

use std::collections::HashMap;
use std::sync::LazyLock;

/// Comprehensive fixture registry for all test data
#[cfg(test)]
pub struct FixtureRegistry {
    pub perl_syntax: HashMap<&'static str, PerlSyntaxFixture>,
    pub lsp_protocol: HashMap<&'static str, LspProtocolFixture>,
    pub dual_indexing: HashMap<&'static str, DualIndexingCorpusEntry>,
    pub builtin_functions: HashMap<&'static str, BuiltinFunctionFixture>,
    pub substitution_ops: HashMap<&'static str, SubstitutionFixture>,
    pub incremental_parsing: HashMap<&'static str, IncrementalParsingFixture>,
}

/// Global fixture registry with lazy initialization
#[cfg(test)]
pub static GLOBAL_FIXTURE_REGISTRY: LazyLock<FixtureRegistry> = LazyLock::new(|| {
    let mut registry = FixtureRegistry {
        perl_syntax: HashMap::new(),
        lsp_protocol: HashMap::new(),
        dual_indexing: HashMap::new(),
        builtin_functions: HashMap::new(),
        substitution_ops: HashMap::new(),
        incremental_parsing: HashMap::new(),
    };

    // Load Perl syntax fixtures
    for fixture in load_comprehensive_syntax_fixtures() {
        registry.perl_syntax.insert(fixture.name, fixture);
    }
    registry.perl_syntax.insert("good_practices", load_good_practices_fixture());

    // Load LSP protocol fixtures
    let lsp_fixtures = vec![
        server_initialization_fixture(),
        perl_run_critic_external_success_fixture(),
        perl_run_critic_builtin_fallback_fixture(),
        perl_run_critic_error_handling_fixture(),
        code_action_extract_variable_fixture(),
        code_action_organize_imports_fixture(),
        definition_cross_file_dual_indexing_fixture(),
        workspace_symbols_enhanced_navigation_fixture(),
    ];

    for fixture in lsp_fixtures {
        registry.lsp_protocol.insert(fixture.name, fixture);
    }

    // Load performance and error fixtures
    for fixture in load_performance_validation_fixtures() {
        registry.lsp_protocol.insert(fixture.name, fixture);
    }

    for fixture in load_error_response_fixtures() {
        registry.lsp_protocol.insert(fixture.name, fixture);
    }

    // Load dual indexing corpus
    for entry in load_dual_indexing_corpus() {
        registry.dual_indexing.insert(entry.name, entry);
    }

    // Load builtin function fixtures
    for fixture in load_all_builtin_fixtures() {
        registry.builtin_functions.insert(fixture.name, fixture);
    }

    // Load substitution operator fixtures
    for fixture in load_all_substitution_fixtures() {
        registry.substitution_ops.insert(fixture.name, fixture);
    }

    // Load incremental parsing fixtures
    for fixture in load_all_incremental_fixtures() {
        registry.incremental_parsing.insert(fixture.name, fixture);
    }

    registry
});

/// Fixture loading utilities for test execution
#[cfg(test)]
pub mod loader {
    use super::*;

    /// Load fixtures by test category for organized testing
    pub fn load_fixtures_by_category(category: TestCategory) -> FixtureSet {
        // Helper to ensure fixtures exist (fail-fast)
        fn require_fixture(name: &str) -> PerlSyntaxFixture {
            match get_fixture_by_name(name) {
                Some(f) => f.clone(),
                None => must(Err::<PerlSyntaxFixture, _>(format!("missing fixture: {name}"))),
            }
        }

        match category {
            TestCategory::BasicParsing => FixtureSet {
                perl_syntax: load_fixtures_by_category(SyntaxCategory::BasicSyntax),
                lsp_protocol: vec![],
                dual_indexing: vec![],
                builtin_functions: vec![],
                substitution_ops: vec![],
                incremental_parsing: vec![],
            },
            TestCategory::ExecuteCommand => FixtureSet {
                perl_syntax: vec![
                    require_fixture("basic_policy_violations"),
                    require_fixture("good_practices"),
                ],
                lsp_protocol: get_fixtures_by_navigation_type(NavigationType::ExecuteCommand),
                dual_indexing: vec![],
                builtin_functions: vec![],
                substitution_ops: vec![],
                incremental_parsing: vec![],
            },
            TestCategory::CodeActions => FixtureSet {
                perl_syntax: vec![
                    require_fixture("cross_file_navigation_dual_indexing"),
                ],
                lsp_protocol: get_fixtures_by_navigation_type(NavigationType::CodeAction),
                dual_indexing: load_corpus_by_category(DualIndexingCategory::CrossFileNavigation),
                builtin_functions: vec![],
                substitution_ops: vec![],
            },
            TestCategory::DualIndexing => FixtureSet {
                perl_syntax: load_dual_indexing_fixtures(),
                lsp_protocol: vec![],
                dual_indexing: load_dual_indexing_corpus(),
                builtin_functions: vec![],
                substitution_ops: vec![],
            },
            TestCategory::Performance => FixtureSet {
                perl_syntax: vec![
                    require_fixture("performance_benchmark_large"),
                ],
                lsp_protocol: get_performance_fixtures(),
                dual_indexing: vec![],
                builtin_functions: load_performance_fixtures(),
                substitution_ops: vec![],
            },
            TestCategory::Unicode => FixtureSet {
                perl_syntax: load_unicode_safe_fixtures(),
                lsp_protocol: vec![],
                dual_indexing: vec![],
                builtin_functions: vec![],
                substitution_ops: load_fixtures_by_complexity(PatternComplexity::Unicode),
            },
            TestCategory::ErrorHandling => FixtureSet {
                perl_syntax: vec![
                    require_fixture("error_scenarios_comprehensive"),
                ],
                lsp_protocol: load_error_response_fixtures(),
                dual_indexing: vec![],
                builtin_functions: vec![],
                substitution_ops: vec![],
                incremental_parsing: vec![],
            },
        }
    }

    /// Load performance validation fixtures for revolutionary improvements testing
    pub fn load_performance_validation_set() -> PerformanceValidationSet {
        // Helper to ensure fixtures exist (fail-fast)
        fn require_fixture(name: &str) -> PerlSyntaxFixture {
            match get_fixture_by_name(name) {
                Some(f) => f.clone(),
                None => must(Err::<PerlSyntaxFixture, _>(format!("missing fixture: {name}"))),
            }
        }

        fn require_sub_fixture(name: &str) -> SubstitutionFixture {
            match get_substitution_fixture_by_name(name) {
                Some(f) => f.clone(),
                None => must(Err::<SubstitutionFixture, _>(format!("missing substitution fixture: {name}"))),
            }
        }

        PerformanceValidationSet {
            parsing_benchmarks: vec![
                require_fixture("performance_benchmark_large"),
                require_fixture("enhanced_builtin_functions"),
            ],
            lsp_response_time: get_performance_fixtures(),
            builtin_performance: load_performance_fixtures(),
            substitution_performance: vec![
                require_sub_fixture("substitution_performance_stress"),
            ],
            incremental_parsing: load_fixtures_by_reuse_efficiency(90.0),
        }
    }

    /// Load thread-safe fixtures for adaptive threading tests
    pub fn load_thread_safe_fixtures() -> ThreadSafeFixtureSet {
        ThreadSafeFixtureSet {
            syntax_fixtures: load_unicode_safe_fixtures(),
            protocol_fixtures: get_thread_safe_fixtures(),
            corpus_entries: load_dual_indexing_corpus(),
            deterministic_builtins: load_deterministic_fixtures(),
        }
    }
}

/// Test category enumeration for organized fixture loading
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum TestCategory {
    BasicParsing,
    ExecuteCommand,
    CodeActions,
    DualIndexing,
    Performance,
    Unicode,
    ErrorHandling,
}

/// Organized fixture set for category-based testing
#[cfg(test)]
pub struct FixtureSet {
    pub perl_syntax: Vec<PerlSyntaxFixture>,
    pub lsp_protocol: Vec<&'static LspProtocolFixture>,
    pub dual_indexing: Vec<DualIndexingCorpusEntry>,
    pub builtin_functions: Vec<BuiltinFunctionFixture>,
    pub substitution_ops: Vec<SubstitutionFixture>,
    pub incremental_parsing: Vec<IncrementalParsingFixture>,
}

/// Performance validation fixture set
#[cfg(test)]
pub struct PerformanceValidationSet {
    pub parsing_benchmarks: Vec<PerlSyntaxFixture>,
    pub lsp_response_time: Vec<&'static LspProtocolFixture>,
    pub builtin_performance: Vec<BuiltinFunctionFixture>,
    pub substitution_performance: Vec<SubstitutionFixture>,
    pub incremental_parsing: Vec<IncrementalParsingFixture>,
}

/// Thread-safe fixture set for adaptive threading testing
#[cfg(test)]
pub struct ThreadSafeFixtureSet {
    pub syntax_fixtures: Vec<PerlSyntaxFixture>,
    pub protocol_fixtures: Vec<&'static LspProtocolFixture>,
    pub corpus_entries: Vec<DualIndexingCorpusEntry>,
    pub deterministic_builtins: Vec<BuiltinFunctionFixture>,
}

/// Validation utilities for fixture integrity
#[cfg(test)]
pub mod validation {
    use super::*;

    /// Validate fixture integrity and coverage
    pub fn validate_fixture_integrity() -> ValidationResult {
        let mut result = ValidationResult {
            syntax_coverage: 0.0,
            lsp_compliance: 0.0,
            dual_indexing_efficiency: 0.0,
            performance_requirements: false,
            unicode_safety: false,
            thread_safety: false,
        };

        // Validate syntax coverage
        let syntax_fixtures = load_comprehensive_syntax_fixtures();
        result.syntax_coverage = calculate_syntax_coverage(&syntax_fixtures);

        // Validate LSP protocol compliance
        let protocol_fixtures = get_thread_safe_fixtures();
        result.lsp_compliance = calculate_protocol_compliance(&protocol_fixtures);

        // Validate dual indexing efficiency
        result.dual_indexing_efficiency = calculate_average_indexing_efficiency();

        // Validate performance requirements
        let performance_fixtures = get_performance_fixtures();
        result.performance_requirements = validate_performance_requirements(&performance_fixtures);

        // Validate Unicode safety
        let unicode_fixtures = load_unicode_safe_fixtures();
        result.unicode_safety = validate_unicode_safety(&unicode_fixtures);

        // Validate thread safety
        let thread_safe_fixtures = get_thread_safe_fixtures();
        result.thread_safety = validate_thread_safety(&thread_safe_fixtures);

        result
    }

    /// Calculate comprehensive syntax coverage
    fn calculate_syntax_coverage(fixtures: &[PerlSyntaxFixture]) -> f32 {
        let categories = [
            SyntaxCategory::BasicSyntax,
            SyntaxCategory::BuiltinFunctions,
            SyntaxCategory::SubstitutionOperators,
            SyntaxCategory::PackageNavigation,
            SyntaxCategory::UnicodeSupport,
        ];

        let covered_categories = categories.iter().filter(|&&category| {
            fixtures.iter().any(|fixture| fixture.syntax_category == category)
        }).count();

        (covered_categories as f32 / categories.len() as f32) * 100.0
    }

    /// Calculate LSP protocol compliance
    fn calculate_protocol_compliance(fixtures: &[&LspProtocolFixture]) -> f32 {
        let required_methods = [
            "initialize",
            "workspace/executeCommand",
            "textDocument/codeAction",
            "textDocument/completion",
            "textDocument/hover",
            "textDocument/definition",
        ];

        let covered_methods = required_methods.iter().filter(|&&method| {
            fixtures.iter().any(|fixture| fixture.request_method == method)
        }).count();

        (covered_methods as f32 / required_methods.len() as f32) * 100.0
    }

    /// Validate performance requirements
    fn validate_performance_requirements(fixtures: &[&LspProtocolFixture]) -> bool {
        fixtures.iter().all(|fixture| {
            fixture.response_time_ms.map_or(true, |time| time <= 2000) // 2s max for executeCommand
        })
    }

    /// Validate Unicode safety
    fn validate_unicode_safety(fixtures: &[PerlSyntaxFixture]) -> bool {
        fixtures.iter().any(|fixture| fixture.unicode_safe)
    }

    /// Validate thread safety
    fn validate_thread_safety(fixtures: &[&LspProtocolFixture]) -> bool {
        fixtures.iter().all(|fixture| fixture.thread_safe)
    }
}

/// Fixture validation results
#[cfg(test)]
pub struct ValidationResult {
    pub syntax_coverage: f32,
    pub lsp_compliance: f32,
    pub dual_indexing_efficiency: f32,
    pub performance_requirements: bool,
    pub unicode_safety: bool,
    pub thread_safety: bool,
}

/// Quick access functions for common fixture retrieval patterns
#[cfg(test)]
pub mod quick_access {
    use super::*;

    /// Get executeCommand test fixtures
    pub fn get_execute_command_fixtures() -> Vec<&'static LspProtocolFixture> {
        get_fixtures_by_navigation_type(NavigationType::ExecuteCommand)
    }

    /// Get code action test fixtures
    pub fn get_code_action_fixtures() -> Vec<&'static LspProtocolFixture> {
        get_fixtures_by_navigation_type(NavigationType::CodeAction)
    }

    /// Get performance validation fixtures
    pub fn get_performance_validation_fixtures() -> Vec<&'static LspProtocolFixture> {
        get_performance_fixtures()
    }

    /// Get dual indexing test corpus
    pub fn get_dual_indexing_corpus() -> Vec<DualIndexingCorpusEntry> {
        load_dual_indexing_corpus()
    }

    /// Get enhanced builtin function fixtures
    pub fn get_builtin_function_fixtures() -> Vec<BuiltinFunctionFixture> {
        load_all_builtin_fixtures()
    }

    /// Get comprehensive substitution fixtures
    pub fn get_substitution_fixtures() -> Vec<SubstitutionFixture> {
        load_all_substitution_fixtures()
    }

    /// Get error handling fixtures
    pub fn get_error_handling_fixtures() -> Vec<&'static LspProtocolFixture> {
        load_error_response_fixtures()
    }

    /// Get thread-safe fixtures for adaptive threading
    pub fn get_thread_safe_test_fixtures() -> Vec<&'static LspProtocolFixture> {
        get_thread_safe_fixtures()
    }

    /// Get incremental parsing fixtures for node reuse validation
    pub fn get_incremental_parsing_fixtures() -> Vec<IncrementalParsingFixture> {
        load_all_incremental_fixtures()
    }

    /// Get high-efficiency node reuse fixtures (>90% reuse)
    pub fn get_high_efficiency_incremental_fixtures() -> Vec<IncrementalParsingFixture> {
        load_fixtures_by_reuse_efficiency(90.0)
    }

    /// Get fast incremental parsing fixtures (<1ms update time)
    pub fn get_fast_incremental_fixtures() -> Vec<IncrementalParsingFixture> {
        load_fixtures_by_update_time(1.0)
    }
}