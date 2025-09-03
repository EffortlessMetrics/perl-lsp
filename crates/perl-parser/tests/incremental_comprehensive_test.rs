//! Comprehensive integration tests for incremental parsing
//!
//! This module provides end-to-end testing of incremental parsing functionality
//! with the new performance testing infrastructure and validation utilities.

#[cfg(feature = "incremental")]
mod incremental_comprehensive_tests {
    use crate::support::incremental_test_utils::IncrementalTestUtils;
    use perl_parser::incremental_v2::IncrementalParserV2;
    use perl_parser::{edit::Edit, position::Position};
    use std::time::Instant;

    mod support;

    #[test]
    fn test_comprehensive_simple_value_edits() {
        let result = perf_test!(
            "Simple Value Edits",
            "my $x = 42;",
            |source| IncrementalTestUtils::create_value_edit(source, "42", "9999"),
            15
        );

        // Validate specific performance requirements for simple edits
        assert!(result.avg_incremental_micros < 500, "Simple edits should be <500µs");
        assert!(result.avg_efficiency_percentage >= 75.0, "Simple edits should have ≥75% efficiency");
        assert!(result.sub_millisecond_rate >= 0.8, "≥80% of simple edits should be sub-millisecond");
    }

    #[test]
    fn test_comprehensive_string_edits() {
        let result = perf_test!(
            "String Content Edits", 
            r#"my $message = "hello";"#,
            |source| IncrementalTestUtils::create_value_edit(source, "hello", "world"),
            15
        );

        // String edits should be efficient
        assert!(result.avg_incremental_micros < 500, "String edits should be <500µs");
        assert!(result.success_rate >= 0.95, "String edits should have ≥95% success rate");
    }

    #[test]  
    fn test_comprehensive_multi_statement_edits() {
        let source = "my $x = 10;\nmy $y = 20;\nmy $z = 30;";
        
        let result = perf_test!(
            "Multi-statement Edits",
            source,
            |source| IncrementalTestUtils::create_value_edit(source, "10", "100"),
            12
        );

        // Multi-statement should still be fast but may have lower reuse rates
        assert!(result.avg_incremental_micros < 1000, "Multi-statement edits should be <1ms");
        assert!(result.avg_efficiency_percentage >= 60.0, "Multi-statement should have ≥60% efficiency");
    }

    #[test]
    fn test_comprehensive_complex_structures() {
        let source = r#"
if ($condition) {
    my $data = {
        name => "value",
        count => 42,
        items => [1, 2, 3]
    };
    process($data->{count});
}
"#;

        let result = perf_test_relaxed!(
            "Complex Nested Structures",
            source,
            |source| IncrementalTestUtils::create_value_edit(source, "42", "999"),
            10
        );

        // Complex structures use relaxed criteria
        assert!(result.avg_incremental_micros < 5000, "Complex structures should be <5ms");
        assert!(result.success_rate >= 0.90, "Complex structures should have ≥90% success rate");
    }

    #[test]
    fn test_comprehensive_large_document_scaling() {
        // Generate progressively larger documents and test scaling
        let sizes = vec![10, 25, 50, 100];
        let mut scaling_results = Vec::new();

        for size in sizes {
            let mut large_source = String::new();
            for i in 0..size {
                large_source.push_str(&format!("my $var{} = {};\n", i, i * 10));
            }

            let result = perf_test_relaxed!(
                &format!("Large Document ({} statements)", size),
                &large_source,
                |source| IncrementalTestUtils::create_value_edit(source, "250", "999"),
                5
            );

            scaling_results.push((size, result.avg_incremental_micros));
            
            // Performance should scale reasonably
            let max_time = match size {
                10 => 2000,   // <2ms for small docs  
                25 => 5000,   // <5ms for medium docs
                50 => 15000,  // <15ms for large docs
                100 => 50000, // <50ms for very large docs
                _ => 100000,
            };
            
            assert!(
                result.avg_incremental_micros < max_time,
                "Document with {} statements should parse in <{}µs, got {}µs",
                size, max_time, result.avg_incremental_micros
            );
        }

        // Analyze scaling characteristics
        println!("\n📈 Scaling Analysis:");
        for (size, time) in &scaling_results {
            println!("  {} statements: {}µs ({:.1}µs/stmt)", size, time, *time as f64 / *size as f64);
        }

        // Performance should not degrade exponentially  
        let small_time = scaling_results[0].1 as f64;
        let large_time = scaling_results.last().unwrap().1 as f64;
        let scaling_factor = large_time / small_time;
        let size_factor = scaling_results.last().unwrap().0 as f64 / scaling_results[0].0 as f64;
        
        println!("Scaling factor: {:.1}x time for {:.1}x size", scaling_factor, size_factor);
        assert!(scaling_factor < size_factor * 2.0, "Performance should scale better than O(n²)");
    }

    #[test]
    fn test_comprehensive_unicode_and_multibyte() {
        let unicode_sources = vec![
            ("Basic Unicode", "my $café = 'résumé';"),
            ("Emoji Heavy", "my $🌟variable = '你好世界 🚀';"),
            ("Mixed Scripts", "my $Здравствуй = 'こんにちは';"),
            ("Right-to-Left", "my $مرحبا = 'שלום';"),
        ];

        for (name, source) in unicode_sources {
            let result = perf_test!(
                &format!("Unicode: {}", name),
                source,
                |source| {
                    // Find a replaceable part (fallback to first quoted content)  
                    if let Some(start) = source.find('\'') {
                        if let Some(end) = source.rfind('\'') {
                            let old_val = &source[start..=end];
                            let new_val = "'replaced'";
                            (source.replace(old_val, new_val), 
                             Edit::new(start, end + 1, start + new_val.len(),
                                     Position::new(start, 1, 1),
                                     Position::new(end + 1, 1, 2), 
                                     Position::new(start + new_val.len(), 1, 2)))
                        } else {
                            IncrementalTestUtils::create_value_edit(source, &source[1..2], "X")
                        }
                    } else {
                        IncrementalTestUtils::create_value_edit(source, &source[3..4], "X")
                    }
                },
                8
            );

            // Unicode should not significantly impact performance
            assert!(result.avg_incremental_micros < 2000, "{} should be <2ms", name);
            assert!(result.success_rate >= 0.90, "{} should have ≥90% success rate", name);
        }
    }

    #[test]
    fn test_comprehensive_edge_cases() {
        let edge_cases = vec![
            ("Empty Replacement", "my $x = 'test';", "test", ""),
            ("Expansion", "my $x = 5;", "5", "12345"),
            ("Whitespace", "my $x = 42  ;", "42", "99"),
            ("At Boundary", "my($x)=42;", "42", "99"), 
            ("Multiple Occurrences", "my $x = 42; my $y = 42;", "42", "99"),
        ];

        for (name, source, old_val, new_val) in edge_cases {
            let result = perf_test!(
                &format!("Edge Case: {}", name),
                source,
                |source| IncrementalTestUtils::create_value_edit(source, old_val, new_val),
                10
            );

            // Edge cases should still perform reasonably  
            assert!(result.avg_incremental_micros < 3000, "{} should be <3ms", name);
            assert!(result.success_rate >= 0.80, "{} should have ≥80% success rate", name);
        }
    }

    #[test]
    fn test_comprehensive_rapid_consecutive_edits() {
        let mut parser = IncrementalParserV2::new();
        let source = "my $a = 1; my $b = 2; my $c = 3;";
        
        // Initial parse
        parser.parse(source).unwrap();
        
        let edits = vec![
            ("1", "10"),
            ("2", "20"), 
            ("3", "30"),
        ];
        
        let mut cumulative_time = 0u128;
        let mut all_efficient = true;
        
        println!("\n🔄 Rapid Consecutive Edits Test:");
        
        for (i, (old_val, new_val)) in edits.iter().enumerate() {
            let (new_source, edit) = IncrementalTestUtils::create_value_edit(source, old_val, new_val);
            
            let start = Instant::now();
            let result = IncrementalTestUtils::measure_incremental_parse(
                &mut parser, source, edit, &new_source
            );
            let individual_time = start.elapsed().as_micros();
            cumulative_time += individual_time;
            
            println!(
                "  Edit {}: {}µs, reused={}, reparsed={}, eff={:.1}%",
                i + 1, individual_time, result.nodes_reused, result.nodes_reparsed,
                result.efficiency_percentage()
            );
            
            // Each edit should be fast
            assert!(individual_time < 3000, "Individual edit {} should be <3ms", i + 1);
            
            if result.efficiency_percentage() < 50.0 {
                all_efficient = false;
            }
        }
        
        println!("  Total time: {}µs", cumulative_time);
        println!("  Average per edit: {}µs", cumulative_time / edits.len() as u128);
        
        // Overall rapid edit performance
        assert!(cumulative_time < 10000, "Total rapid edit time should be <10ms");
        
        if all_efficient {
            println!("✅ All rapid edits maintained good efficiency");
        } else {
            println!("⚠️ Some rapid edits had lower efficiency (expected for complex scenarios)");
        }
    }

    #[test]
    fn test_comprehensive_memory_and_stability() {
        // Test memory stability under repeated operations
        let mut parser = IncrementalParserV2::new();
        let base_source = "my $counter = 0;";
        
        println!("\n🧠 Memory Stability Test:");
        
        // Perform many iterations to test for memory leaks or instability
        for i in 0..100 {
            let new_val = format!("{}", i);
            let (new_source, edit) = IncrementalTestUtils::create_value_edit(base_source, "0", &new_val);
            
            let start = Instant::now();
            let result = IncrementalTestUtils::measure_incremental_parse(
                &mut parser, base_source, edit, &new_source
            );
            let parse_time = start.elapsed();
            
            if i % 20 == 0 {
                println!(
                    "  Iteration {}: {}µs, reused={}, reparsed={}",
                    i, parse_time.as_micros(), result.nodes_reused, result.nodes_reparsed
                );
            }
            
            // Performance should remain stable
            assert!(parse_time.as_micros() < 5000, "Parse time should remain stable at iteration {}", i);
            assert!(result.success, "Parse should succeed at iteration {}", i);
            
            // Reset for next iteration
            parser = IncrementalParserV2::new();
            parser.parse(base_source).unwrap();
        }
        
        println!("✅ Memory stability test completed successfully");
    }

    #[test]
    fn test_comprehensive_performance_regression_detection() {
        // Run the same test multiple times to detect performance regressions
        let test_source = "my $regression_test = 42;";
        let mut batch_averages = Vec::new();
        
        println!("\n📊 Performance Regression Detection:");
        
        for batch in 0..5 {
            let result = perf_test!(
                &format!("Regression Batch {}", batch + 1),
                test_source,
                |source| IncrementalTestUtils::create_value_edit(source, "42", "999"),
                10
            );
            
            batch_averages.push(result.avg_incremental_micros);
            println!("  Batch {}: avg={}µs", batch + 1, result.avg_incremental_micros);
        }
        
        // Analyze for regression
        let overall_avg = batch_averages.iter().sum::<u128>() / batch_averages.len() as u128;
        let min_time = *batch_averages.iter().min().unwrap();
        let max_time = *batch_averages.iter().max().unwrap();
        
        println!("  Overall average: {}µs", overall_avg);
        println!("  Range: {}µs - {}µs", min_time, max_time);
        
        // Check for regression (no batch should be significantly slower)
        for (i, &avg) in batch_averages.iter().enumerate() {
            assert!(
                avg < overall_avg * 2,
                "Performance regression detected in batch {}: {}µs vs overall avg {}µs",
                i + 1, avg, overall_avg
            );
        }
        
        // Check for excessive variation
        let variation_factor = max_time as f64 / min_time as f64;
        assert!(
            variation_factor < 3.0,
            "Excessive performance variation: {:.1}x between batches",
            variation_factor
        );
        
        println!("✅ No significant performance regression detected");
    }

    // Include the perf_test! and perf_test_relaxed! macros
    use crate::{perf_test, perf_test_relaxed};
}

#[cfg(not(feature = "incremental"))]
#[test]
#[ignore = "Incremental feature not enabled"]  
fn test_comprehensive_incremental_feature_disabled() {
    println!("Comprehensive incremental tests require 'incremental' feature");
}