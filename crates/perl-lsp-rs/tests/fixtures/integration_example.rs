//! Integration example demonstrating comprehensive Perl LSP test fixture usage
//!
//! This module provides integration examples showing how to use the comprehensive
//! fixture infrastructure for testing Perl LSP executeCommand functionality with
//! realistic test data and proper validation patterns.

#[cfg(test)]
mod integration_tests {
    use super::super::fixtures::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Example test demonstrating fixture integration for executeCommand testing
    #[test]
    fn example_execute_command_fixture_integration() -> TestResult {
        // Load executeCommand-specific fixtures
        let execute_fixtures = quick_access::get_execute_command_fixtures();
        assert!(!execute_fixtures.is_empty(), "Should have executeCommand fixtures");

        // Load corresponding Perl syntax fixtures
        let syntax_fixtures = loader::load_fixtures_by_category(TestCategory::ExecuteCommand);
        assert!(!syntax_fixtures.perl_syntax.is_empty(), "Should have Perl syntax fixtures");

        // Example validation using protocol fixture
        let perl_critic_fixture = get_protocol_fixture_by_name("perl_run_critic_external_success")
            .ok_or("Should have perl.runCritic fixture")?;

        assert_eq!(
            perl_critic_fixture.request_method,
            "workspace/executeCommand",
            "Should be executeCommand request"
        );
        assert!(
            perl_critic_fixture.response_time_ms.unwrap_or(0) < 2000,
            "Should meet <2s performance requirement"
        );

        // Example validation using syntax fixture
        let policy_violations_fixture = get_fixture_by_name("basic_policy_violations")
            .ok_or("Should have policy violations fixture")?;

        assert!(
            policy_violations_fixture.expected_violations > 0,
            "Should have expected policy violations"
        );
        assert!(
            policy_violations_fixture.parsing_time_us.unwrap_or(0) < 200,
            "Should parse quickly"
        );

        println!("✅ ExecuteCommand fixture integration validated");
        Ok(())
    }

    /// Example test demonstrating dual indexing corpus validation
    #[test]
    fn example_dual_indexing_corpus_validation() -> TestResult {
        // Load dual indexing corpus
        let corpus = quick_access::get_dual_indexing_corpus();
        assert!(!corpus.is_empty(), "Should have dual indexing corpus entries");

        // Validate coverage efficiency
        let avg_efficiency = calculate_average_indexing_efficiency();
        assert!(
            avg_efficiency > 95.0,
            "Should achieve >95% average indexing efficiency, got: {}%",
            avg_efficiency
        );

        // Example corpus entry validation
        let basic_resolution = get_corpus_entry_by_name("basic_package_resolution")
            .ok_or("Should have basic package resolution corpus")?;

        assert!(
            !basic_resolution.expected_qualified_refs.is_empty(),
            "Should have qualified references"
        );
        assert!(
            !basic_resolution.expected_bare_refs.is_empty(),
            "Should have bare references"
        );
        assert!(
            basic_resolution.indexing_efficiency > 95.0,
            "Should achieve high indexing efficiency"
        );

        // Property-based validation
        use corpus::dual_indexing_corpus::property_testing::*;
        assert!(
            validate_dual_indexing_coverage(basic_resolution),
            "Should validate dual indexing coverage"
        );
        assert!(
            validate_cross_file_links(basic_resolution),
            "Should validate cross-file links"
        );

        println!("✅ Dual indexing corpus validation passed");
        Ok(())
    }

    /// Example test demonstrating enhanced builtin function fixture usage
    #[test]
    fn example_enhanced_builtin_fixtures() {
        // Load builtin function fixtures
        let builtin_fixtures = quick_access::get_builtin_function_fixtures();
        assert!(!builtin_fixtures.is_empty(), "Should have builtin function fixtures");

        // Load deterministic parsing fixtures
        let deterministic_fixtures = load_deterministic_fixtures();
        assert!(
            !deterministic_fixtures.is_empty(),
            "Should have deterministic fixtures"
        );

        // Example validation of map function parsing
        let map_fixtures = load_fixtures_by_type(BuiltinType::Map);
        assert!(!map_fixtures.is_empty(), "Should have map function fixtures");

        for fixture in &map_fixtures {
            assert!(fixture.deterministic, "Map fixtures should be deterministic");
            assert!(
                fixture.parsing_time_us.unwrap_or(0) < 200,
                "Map parsing should be fast"
            );
        }

        // Example validation of empty block edge cases
        let empty_block_fixtures = load_fixtures_by_parsing_mode(BlockParsingMode::Empty);
        assert!(
            !empty_block_fixtures.is_empty(),
            "Should have empty block fixtures"
        );

        println!("✅ Enhanced builtin function fixtures validated");
    }

    /// Example test demonstrating comprehensive substitution operator coverage
    #[test]
    fn example_substitution_operator_coverage() {
        // Load all substitution fixtures
        let substitution_fixtures = quick_access::get_substitution_fixtures();
        assert!(!substitution_fixtures.is_empty(), "Should have substitution fixtures");

        // Test delimiter type coverage
        let delimiter_types = vec![
            DelimiterType::Standard,
            DelimiterType::Balanced,
            DelimiterType::Alternative,
            DelimiterType::SingleQuote,
        ];

        for delimiter_type in delimiter_types {
            let fixtures = load_fixtures_by_delimiter_type(delimiter_type.clone());
            assert!(
                !fixtures.is_empty(),
                "Should have fixtures for delimiter type: {:?}",
                delimiter_type
            );
        }

        // Test pattern complexity coverage
        let complexity_levels = vec![
            PatternComplexity::Simple,
            PatternComplexity::Regex,
            PatternComplexity::Complex,
            PatternComplexity::Escaped,
            PatternComplexity::Unicode,
        ];

        for complexity in complexity_levels {
            let fixtures = load_fixtures_by_complexity(complexity.clone());
            // Note: Not all complexity levels may have fixtures
            if !fixtures.is_empty() {
                println!("✓ Found fixtures for complexity: {:?}", complexity);
            }
        }

        // Validate high accuracy fixtures
        let high_accuracy = load_high_accuracy_fixtures();
        assert!(
            !high_accuracy.is_empty(),
            "Should have high accuracy fixtures"
        );

        for fixture in &high_accuracy {
            assert!(
                fixture.parsing_accuracy > 98.0,
                "High accuracy fixtures should have >98% accuracy"
            );
        }

        println!("✅ Comprehensive substitution operator coverage validated");
    }

    /// Example test demonstrating incremental parsing performance validation
    #[test]
    fn example_incremental_parsing_performance() {
        // Load incremental parsing fixtures
        let incremental_fixtures = quick_access::get_incremental_parsing_fixtures();
        assert!(!incremental_fixtures.is_empty(), "Should have incremental parsing fixtures");

        // Load high-efficiency fixtures
        let high_efficiency = quick_access::get_high_efficiency_incremental_fixtures();
        assert!(!high_efficiency.is_empty(), "Should have high-efficiency fixtures");

        // Load fast parsing fixtures
        let fast_fixtures = quick_access::get_fast_incremental_fixtures();
        assert!(!fast_fixtures.is_empty(), "Should have fast incremental fixtures");

        // Validate performance requirements
        for fixture in &fast_fixtures {
            assert!(
                fixture.update_time_ms < 1.0,
                "Fast fixtures should update in <1ms, got: {}ms for '{}'",
                fixture.update_time_ms,
                fixture.name
            );
            assert!(
                fixture.expected_reuse_percentage >= fixture.reuse_efficiency_target,
                "Should meet reuse efficiency target"
            );
        }

        // Validate UTF-16 safety
        let utf16_safe = load_utf16_safe_fixtures();
        assert!(!utf16_safe.is_empty(), "Should have UTF-16 safe fixtures");

        for fixture in &utf16_safe {
            assert!(fixture.utf16_safe, "Should be UTF-16 safe");
        }

        println!("✅ Incremental parsing performance validation passed");
    }

    /// Example test demonstrating comprehensive fixture validation
    #[test]
    fn example_comprehensive_fixture_validation() {
        // Run comprehensive fixture integrity validation
        let validation_result = validation::validate_fixture_integrity();

        // Validate syntax coverage
        assert!(
            validation_result.syntax_coverage > 80.0,
            "Should achieve >80% syntax coverage, got: {}%",
            validation_result.syntax_coverage
        );

        // Validate LSP protocol compliance
        assert!(
            validation_result.lsp_compliance > 90.0,
            "Should achieve >90% LSP compliance, got: {}%",
            validation_result.lsp_compliance
        );

        // Validate dual indexing efficiency
        assert!(
            validation_result.dual_indexing_efficiency > 95.0,
            "Should achieve >95% dual indexing efficiency, got: {}%",
            validation_result.dual_indexing_efficiency
        );

        // Validate performance requirements
        assert!(
            validation_result.performance_requirements,
            "Should meet all performance requirements"
        );

        // Validate Unicode safety
        assert!(
            validation_result.unicode_safety,
            "Should have Unicode-safe fixtures"
        );

        // Validate thread safety
        assert!(
            validation_result.thread_safety,
            "Should have thread-safe fixtures"
        );

        println!("✅ Comprehensive fixture validation passed");
    }

    /// Example test demonstrating organized fixture loading by category
    #[test]
    fn example_organized_fixture_loading() {
        // Test all test categories
        let categories = vec![
            TestCategory::BasicParsing,
            TestCategory::ExecuteCommand,
            TestCategory::CodeActions,
            TestCategory::DualIndexing,
            TestCategory::Performance,
            TestCategory::Unicode,
            TestCategory::ErrorHandling,
        ];

        for category in categories {
            let fixture_set = loader::load_fixtures_by_category(category.clone());

            match category {
                TestCategory::BasicParsing => {
                    assert!(
                        !fixture_set.perl_syntax.is_empty(),
                        "Basic parsing should have syntax fixtures"
                    );
                }
                TestCategory::ExecuteCommand => {
                    assert!(
                        !fixture_set.lsp_protocol.is_empty(),
                        "ExecuteCommand should have protocol fixtures"
                    );
                }
                TestCategory::CodeActions => {
                    assert!(
                        !fixture_set.dual_indexing.is_empty(),
                        "Code actions should have dual indexing fixtures"
                    );
                }
                TestCategory::DualIndexing => {
                    assert!(
                        !fixture_set.dual_indexing.is_empty(),
                        "Dual indexing should have corpus fixtures"
                    );
                }
                TestCategory::Performance => {
                    assert!(
                        !fixture_set.incremental_parsing.is_empty(),
                        "Performance should have incremental parsing fixtures"
                    );
                }
                TestCategory::Unicode => {
                    assert!(
                        !fixture_set.substitution_ops.is_empty(),
                        "Unicode should have substitution fixtures"
                    );
                }
                TestCategory::ErrorHandling => {
                    assert!(
                        !fixture_set.lsp_protocol.is_empty(),
                        "Error handling should have protocol fixtures"
                    );
                }
            }

            println!("✓ Category {:?} fixtures loaded successfully", category);
        }

        println!("✅ Organized fixture loading validation passed");
    }
}

/// Example usage documentation
#[cfg(test)]
mod usage_examples {
    use super::super::fixtures::*;

    /// How to use Perl syntax fixtures in your tests
    #[allow(dead_code)]
    fn example_perl_syntax_fixture_usage() {
        // Load comprehensive syntax fixtures
        let syntax_fixtures = load_comprehensive_syntax_fixtures();

        // Filter by category
        let basic_fixtures = load_fixtures_by_category(SyntaxCategory::BasicSyntax);

        // Get specific fixture
        if let Some(fixture) = get_fixture_by_name("basic_policy_violations") {
            // Use fixture in test
            let perl_code = fixture.perl_code;
            let expected_violations = fixture.expected_violations;

            // Test parser with fixture data
            assert!(!perl_code.is_empty());
            assert!(expected_violations > 0);
        }
    }

    /// How to use LSP protocol fixtures in your tests
    #[allow(dead_code)]
    fn example_lsp_protocol_fixture_usage() {
        // Load protocol fixtures
        let protocol_fixtures = get_execute_command_fixtures();

        // Get specific fixture
        if let Some(fixture) = get_protocol_fixture_by_name("perl_run_critic_external_success") {
            // Use fixture in LSP test
            let request_method = fixture.request_method;
            let request_params = &fixture.request_params;
            let expected_response = &fixture.expected_response;

            // Test LSP server with fixture data
            assert_eq!(request_method, "workspace/executeCommand");
            assert!(!request_params.is_null());
            assert!(!expected_response.is_null());
        }
    }

    /// How to use dual indexing corpus in your tests
    #[allow(dead_code)]
    fn example_dual_indexing_corpus_usage() {
        // Load corpus entries
        let corpus = load_dual_indexing_corpus();

        // Get specific entry
        if let Some(entry) = get_corpus_entry_by_name("basic_package_resolution") {
            // Use corpus entry in navigation test
            let perl_files = &entry.perl_files;
            let qualified_refs = &entry.expected_qualified_refs;
            let bare_refs = &entry.expected_bare_refs;

            // Test dual indexing with corpus data
            assert!(!perl_files.is_empty());
            assert!(!qualified_refs.is_empty());
            assert!(!bare_refs.is_empty());
        }
    }
}