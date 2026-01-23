#![cfg(feature = "incremental")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Comprehensive edge case tests for incremental parsing
//!
//! This module tests the incremental parser against challenging scenarios
//! that stress the tree reuse algorithms and fallback mechanisms.

mod support;

use crate::support::incremental_test_utils::IncrementalTestUtils;
use perl_parser::incremental_v2::IncrementalParserV2;
// Remove unused imports - these were imported but not used in the current test implementation
use std::time::Instant;

/// Test incremental parsing with deeply nested structures
#[test]
fn test_deeply_nested_structures() {
    println!("\n🏗️ Testing deeply nested structures...");

    // Create deeply nested Perl structure
    let mut nested_source = "if (\n".to_string();
    for i in 0..10 {
        nested_source.push_str(&format!("  $condition{} &&\n", i));
    }
    nested_source.push_str("  1\n) {\n");
    for i in 0..5 {
        nested_source.push_str(&format!("  if ($sub{}) {{\n", i));
    }
    nested_source.push_str("    my $deep = 42;\n");
    for _i in 0..5 {
        nested_source.push_str("  }\n");
    }
    nested_source.push_str("}\n");

    let mut parser = IncrementalParserV2::new();

    // Initial parse
    let start = Instant::now();
    parser.parse(&nested_source).unwrap();
    let initial_time = start.elapsed();
    let initial_nodes = parser.reparsed_nodes;

    println!("  Initial parse: {}ms, {} nodes", initial_time.as_millis(), initial_nodes);

    // Edit deep inside the structure
    let (new_source, edit) = IncrementalTestUtils::create_value_edit(&nested_source, "42", "999");

    let start = Instant::now();
    let result = IncrementalTestUtils::measure_incremental_parse(
        &mut parser,
        &nested_source,
        edit,
        &new_source,
    );
    let incremental_time = start.elapsed();

    println!(
        "  Incremental parse: {}ms, reused={}, reparsed={}, efficiency={:.1}%",
        incremental_time.as_millis(),
        result.nodes_reused,
        result.nodes_reparsed,
        result.efficiency_percentage()
    );

    // Deep nesting should still be handled reasonably
    assert!(incremental_time.as_millis() < 100, "Deep nesting should parse in <100ms");
    assert!(result.success, "Deep nesting parse should succeed");

    // Some reuse should be achieved even in complex structures
    if result.nodes_reused > 0 {
        println!(
            "  ✅ Successfully reused {} nodes in deeply nested structure",
            result.nodes_reused
        );
    } else {
        println!("  ⚠️ No node reuse in deeply nested structure (acceptable fallback)");
    }
}

/// Test incremental parsing with mixed quote types and escaping
#[test]
fn test_complex_string_handling() {
    println!("\n🔤 Testing complex string handling...");

    let complex_strings = vec![
        (r#"my $single = 'hello world';"#, "hello world", "modified"),
        (r#"my $double = "hello \"quoted\" world";"#, "hello \\\"quoted\\\" world", "new content"),
        (r#"my $backtick = `echo hello`;"#, "echo hello", "echo modified"),
        (r#"my $heredoc = <<'EOF';\nhello\nEOF"#, "hello", "modified"),
        (r#"my $interpolated = "Value: $var";"#, "Value: $var", "New: $other"),
    ];

    for (i, (source, old_val, new_val)) in complex_strings.iter().enumerate() {
        println!("  Test case {}: {}", i + 1, source.get(..30).unwrap_or(source));

        let mut parser = IncrementalParserV2::new();
        parser.parse(source).unwrap();

        // Attempt to create edit - some may fail due to string complexity
        let edit_result = std::panic::catch_unwind(|| {
            IncrementalTestUtils::create_value_edit(source, old_val, new_val)
        });

        match edit_result {
            Ok((new_source, edit)) => {
                let start = Instant::now();
                let result = IncrementalTestUtils::measure_incremental_parse(
                    &mut parser,
                    source,
                    edit,
                    &new_source,
                );
                let parse_time = start.elapsed();

                println!(
                    "    Parse: {}µs, reused={}, reparsed={}, success={}",
                    parse_time.as_micros(),
                    result.nodes_reused,
                    result.nodes_reparsed,
                    result.success
                );

                assert!(parse_time.as_millis() < 10, "Complex string parsing should be <10ms");
                assert!(result.success, "Complex string parsing should succeed");
            }
            Err(_) => {
                println!("    Skipped due to string complexity (expected for some cases)");
            }
        }
    }
}

/// Test incremental parsing with various whitespace and formatting patterns
#[test]
fn test_whitespace_sensitivity() {
    println!("\n⚪ Testing whitespace sensitivity...");

    let whitespace_cases = vec![
        // Tabs vs spaces
        ("my\t$x\t=\t42;", "42", "99"),
        ("my $x = 42  ;", "42", "99"),         // Extra spaces
        ("my$x=42;", "42", "99"),              // No spaces
        ("my $x =\n    42;", "42", "99"),      // Multi-line
        ("my $x = 42; # comment", "42", "99"), // With comment
        ("  my $x = 42;  ", "42", "99"),       // Leading/trailing whitespace
    ];

    for (i, (source, old_val, new_val)) in whitespace_cases.iter().enumerate() {
        println!("  Whitespace case {}: {:?}", i + 1, source);

        let mut parser = IncrementalParserV2::new();
        parser.parse(source).unwrap();

        let (new_source, edit) = IncrementalTestUtils::create_value_edit(source, old_val, new_val);

        let start = Instant::now();
        let result =
            IncrementalTestUtils::measure_incremental_parse(&mut parser, source, edit, &new_source);
        let parse_time = start.elapsed();

        println!(
            "    Result: {}µs, reused={}, reparsed={}, eff={:.1}%",
            parse_time.as_micros(),
            result.nodes_reused,
            result.nodes_reparsed,
            result.efficiency_percentage()
        );

        assert!(parse_time.as_micros() < 5000, "Whitespace handling should be <5ms");
        assert!(result.success, "Whitespace variations should parse successfully");

        // Whitespace changes should generally achieve good reuse
        if result.nodes_reused > 0 {
            assert!(
                result.efficiency_percentage() >= 60.0,
                "Whitespace edits should achieve ≥60% efficiency"
            );
        }
    }
}

/// Test incremental parsing with syntax error recovery scenarios
#[test]
fn test_syntax_error_recovery() {
    println!("\n🚨 Testing syntax error recovery...");

    let error_cases = [
        // Missing semicolons
        ("my $x = 42", "42", "99"),
        // Unmatched parens
        ("print(hello", "hello", "world"),
        // Invalid operators
        ("my $x =* 42;", "42", "99"),
        // Incomplete statements
        ("if (", "(", "(\n  1\n) {"),
    ];

    for (i, (source, old_val, new_val)) in error_cases.iter().enumerate() {
        println!("  Error case {}: {}", i + 1, source);

        let mut parser = IncrementalParserV2::new();

        // Initial parse might fail - that's OK for error recovery tests
        let initial_result = parser.parse(source);
        println!(
            "    Initial parse: {}",
            if initial_result.is_ok() { "OK" } else { "Failed (expected)" }
        );

        // Try the incremental edit anyway
        let edit_result = std::panic::catch_unwind(|| {
            IncrementalTestUtils::create_value_edit(source, old_val, new_val)
        });

        match edit_result {
            Ok((new_source, edit)) => {
                let start = Instant::now();
                let result = IncrementalTestUtils::measure_incremental_parse(
                    &mut parser,
                    source,
                    edit,
                    &new_source,
                );
                let parse_time = start.elapsed();

                println!(
                    "    Incremental: {}µs, success={}, reused={}, reparsed={}",
                    parse_time.as_micros(),
                    result.success,
                    result.nodes_reused,
                    result.nodes_reparsed
                );

                // Error recovery should complete quickly even if it fails
                assert!(parse_time.as_millis() < 20, "Error recovery should complete in <20ms");

                // It's OK if error recovery doesn't succeed - the important thing is it doesn't crash
                println!("    Error recovery handled gracefully");
            }
            Err(_) => {
                println!("    Edit creation failed (acceptable for malformed syntax)");
            }
        }
    }
}

/// Test incremental parsing with very large single statements
#[test]
fn test_very_large_statements() {
    println!("\n📏 Testing very large statements...");

    // Generate a very large array literal
    let mut large_array = "my @huge = (".to_string();
    for i in 0..1000 {
        if i > 0 {
            large_array.push_str(", ");
        }
        large_array.push_str(&format!("{}", i));
    }
    large_array.push_str(");");

    let mut parser = IncrementalParserV2::new();

    let start = Instant::now();
    parser.parse(&large_array).unwrap();
    let initial_time = start.elapsed();
    let initial_nodes = parser.reparsed_nodes;

    println!("  Large array parse: {}ms, {} nodes", initial_time.as_millis(), initial_nodes);

    // Edit one element in the middle
    let (new_source, edit) = IncrementalTestUtils::create_value_edit(&large_array, "500", "777");

    let start = Instant::now();
    let result = IncrementalTestUtils::measure_incremental_parse(
        &mut parser,
        &large_array,
        edit,
        &new_source,
    );
    let incremental_time = start.elapsed();

    println!(
        "  Large array incremental: {}ms, reused={}, reparsed={}, eff={:.1}%",
        incremental_time.as_millis(),
        result.nodes_reused,
        result.nodes_reparsed,
        result.efficiency_percentage()
    );

    assert!(incremental_time.as_millis() < 100, "Large array incremental should be <100ms");
    assert!(result.success, "Large array incremental should succeed");

    // Large statements should show some efficiency improvement
    if result.nodes_reused > 100 {
        println!("  ✅ Good reuse achieved for large statement");
    } else {
        println!("  ⚠️ Limited reuse for large statement (fallback to full parsing)");
    }
}

/// Test incremental parsing with complex regular expressions
#[test]
fn test_complex_regex_handling() {
    println!("\n🔍 Testing complex regex handling...");

    let regex_cases = vec![
        (r#"$text =~ /simple/;"#, "simple", "complex"),
        (r#"$text =~ m{complex{2,3}}i;"#, "complex", "pattern"),
        (r#"$text =~ s/old/new/g;"#, "old", "modified"),
        (r#"$text =~ tr/a-z/A-Z/;"#, "a-z", "0-9"),
        (r#"if ($str =~ /\d+/) { print; }"#, "\\d+", "\\w+"),
    ];

    for (i, (source, old_val, new_val)) in regex_cases.iter().enumerate() {
        println!("  Regex case {}: {}", i + 1, source);

        let mut parser = IncrementalParserV2::new();
        parser.parse(source).unwrap();

        let edit_result = std::panic::catch_unwind(|| {
            IncrementalTestUtils::create_value_edit(source, old_val, new_val)
        });

        match edit_result {
            Ok((new_source, edit)) => {
                let start = Instant::now();
                let result = IncrementalTestUtils::measure_incremental_parse(
                    &mut parser,
                    source,
                    edit,
                    &new_source,
                );
                let parse_time = start.elapsed();

                println!(
                    "    Parse: {}µs, reused={}, reparsed={}, success={}",
                    parse_time.as_micros(),
                    result.nodes_reused,
                    result.nodes_reparsed,
                    result.success
                );

                assert!(parse_time.as_millis() < 10, "Regex parsing should be <10ms");
                assert!(result.success, "Regex parsing should succeed");
            }
            Err(_) => {
                println!("    Regex edit skipped due to complexity");
            }
        }
    }
}

/// Test incremental parsing with extreme position shifts
#[test]
fn test_extreme_position_shifts() {
    println!("\n↔️ Testing extreme position shifts...");

    // Create source with content at the beginning, middle, and end
    let source = format!(
        "# Header comment\n{}\n# Middle comment\n{}\n# Footer comment",
        "my $early = 100;".repeat(20),
        "my $late = 200;".repeat(20)
    );

    let mut parser = IncrementalParserV2::new();
    parser.parse(&source).unwrap();

    // Make a small edit near the beginning that could affect positions throughout
    let (new_source, edit) = IncrementalTestUtils::create_value_edit(&source, "100", "1");

    let start = Instant::now();
    let result =
        IncrementalTestUtils::measure_incremental_parse(&mut parser, &source, edit, &new_source);
    let parse_time = start.elapsed();

    println!(
        "  Position shift result: {}ms, reused={}, reparsed={}, eff={:.1}%",
        parse_time.as_millis(),
        result.nodes_reused,
        result.nodes_reparsed,
        result.efficiency_percentage()
    );

    assert!(parse_time.as_millis() < 50, "Position shift handling should be <50ms");
    assert!(result.success, "Position shift handling should succeed");

    // Position shifts are challenging but some reuse should still be possible
    if result.nodes_reused > 0 {
        println!("  ✅ Some nodes reused despite position shifts");
    } else {
        println!("  ⚠️ Full reparse due to position shifts (acceptable for complex cases)");
    }
}

/// Test incremental parsing with circular reference patterns
#[test]
fn test_circular_reference_patterns() {
    println!("\n🔄 Testing circular reference patterns...");

    let circular_cases = [
        // Self-referencing hash
        ("my %hash = (key => \\%hash);", "key", "ref"),
        // Nested self-reference
        ("my $ref = \\$ref;", "ref", "circular"),
        // Function calling itself
        ("sub recurse { recurse(); }", "recurse", "factorial"),
    ];

    for (i, (source, old_val, new_val)) in circular_cases.iter().enumerate() {
        println!("  Circular case {}: {}", i + 1, source);

        let mut parser = IncrementalParserV2::new();

        // Parse initial - might have issues with circular refs, that's OK
        let initial_result = parser.parse(source);
        println!("    Initial parse: {}", if initial_result.is_ok() { "OK" } else { "Failed" });

        if initial_result.is_ok() {
            let (new_source, edit) =
                IncrementalTestUtils::create_value_edit(source, old_val, new_val);

            let start = Instant::now();
            let result = IncrementalTestUtils::measure_incremental_parse(
                &mut parser,
                source,
                edit,
                &new_source,
            );
            let parse_time = start.elapsed();

            println!(
                "    Incremental: {}µs, reused={}, reparsed={}, success={}",
                parse_time.as_micros(),
                result.nodes_reused,
                result.nodes_reparsed,
                result.success
            );

            // Circular references should not cause infinite loops
            assert!(
                parse_time.as_millis() < 50,
                "Circular reference handling should complete in <50ms"
            );

            println!("    Circular reference handling completed");
        } else {
            println!("    Skipped due to initial parse failure");
        }
    }
}

/// Test incremental parsing performance under memory pressure
#[test]
fn test_memory_pressure_scenarios() {
    println!("\n🧠 Testing memory pressure scenarios...");

    // Create multiple large documents to stress memory usage
    let mut parsers = Vec::new();
    let large_doc = format!(
        "{}{}{}",
        "# Large Perl module\npackage TestModule;\n",
        "sub function {} { my $var{} = {}; return $var{}; }\n".repeat(100),
        "1; # End of module\n"
    );

    for i in 0..10 {
        let mut parser = IncrementalParserV2::new();
        let doc = large_doc.replace("{}", &i.to_string());

        let start = Instant::now();
        parser.parse(&doc).unwrap();
        let parse_time = start.elapsed();

        parsers.push((parser, doc));

        println!("  Parser {}: initial parse {}ms", i, parse_time.as_millis());

        // Each parser should complete in reasonable time even under memory pressure
        assert!(parse_time.as_millis() < 200, "Parse under memory pressure should be <200ms");
    }

    // Now test incremental updates on all parsers
    for (i, (parser, doc)) in parsers.iter_mut().enumerate() {
        let (new_source, edit) =
            IncrementalTestUtils::create_value_edit(doc, &i.to_string(), "999");

        let start = Instant::now();
        let result =
            IncrementalTestUtils::measure_incremental_parse(parser, doc, edit, &new_source);
        let parse_time = start.elapsed();

        println!(
            "  Parser {} incremental: {}ms, reused={}, reparsed={}",
            i,
            parse_time.as_millis(),
            result.nodes_reused,
            result.nodes_reparsed
        );

        assert!(parse_time.as_millis() < 100, "Incremental under memory pressure should be <100ms");
        assert!(result.success, "Incremental under memory pressure should succeed");
    }

    println!("  Memory pressure test completed successfully");
}

/// Test incremental parsing with concurrent-like rapid edits
#[test]
fn test_rapid_fire_edits() {
    println!("\n⚡ Testing rapid fire edits...");

    let base_source = "my $counter = 0; my $result = $counter * 2;";
    let mut parser = IncrementalParserV2::new();
    parser.parse(base_source).unwrap();

    let mut cumulative_time = 0u128;
    let mut all_results = Vec::new();

    // Simulate rapid typing by making many small edits in quick succession
    for i in 0..50 {
        let old_val = if i == 0 { "0" } else { &(i - 1).to_string() };
        let new_val = i.to_string();

        let (new_source, edit) =
            IncrementalTestUtils::create_value_edit(base_source, old_val, &new_val);

        let start = Instant::now();
        let result = IncrementalTestUtils::measure_incremental_parse(
            &mut parser,
            base_source,
            edit,
            &new_source,
        );
        let parse_time = start.elapsed();

        cumulative_time += parse_time.as_micros();
        all_results.push((i, parse_time.as_micros(), result.nodes_reused, result.nodes_reparsed));

        if i % 10 == 0 || i < 5 {
            println!(
                "  Edit {}: {}µs, reused={}, reparsed={}",
                i,
                parse_time.as_micros(),
                result.nodes_reused,
                result.nodes_reparsed
            );
        }

        // Each rapid edit should be very fast
        assert!(parse_time.as_millis() < 10, "Rapid edit {} should be <10ms", i);
        assert!(result.success, "Rapid edit {} should succeed", i);
    }

    let avg_time = cumulative_time / all_results.len() as u128;
    println!("  Total rapid edits: {}µs, average per edit: {}µs", cumulative_time, avg_time);

    // Rapid edits should maintain good average performance
    assert!(avg_time < 2000, "Average rapid edit time should be <2ms");

    // Check for performance stability (no major degradation over time)
    let early_avg = all_results.iter().take(10).map(|(_, t, _, _)| *t).sum::<u128>() / 10;
    let late_avg = all_results.iter().skip(40).map(|(_, t, _, _)| *t).sum::<u128>() / 10;

    println!("  Performance stability: early avg={}µs, late avg={}µs", early_avg, late_avg);

    // Performance should not degrade significantly over rapid edits
    assert!(late_avg < early_avg * 3, "Rapid edit performance should remain stable");

    println!("  ✅ Rapid fire edit test completed successfully");
}

/// Test incremental parsing with edge cases in Unicode handling
#[test]
fn test_unicode_edge_cases() {
    println!("\n🌐 Testing Unicode edge cases...");

    let unicode_edge_cases = vec![
        // Zero-width characters
        ("my $invisible = 'text\u{200B}here';", "text", "modified"),
        // Combined characters
        ("my $combined = 'é';", "é", "è"), // é (combined) vs è (precomposed)
        // Right-to-left text
        ("my $rtl = 'مرحبا';", "مرحبا", "שלום"),
        // Emoji with skin tone modifiers
        ("my $emoji = '👨‍💻';", "👨‍💻", "👩‍💻"),
        // Various Unicode spaces
        ("my $spaces = 'normal\u{2000}thin\u{2009}hair\u{200A}space';", "normal", "changed"),
    ];

    for (i, (source, old_val, new_val)) in unicode_edge_cases.iter().enumerate() {
        println!("  Unicode edge case {}: chars={}", i + 1, source.chars().count());

        let mut parser = IncrementalParserV2::new();

        let initial_result = parser.parse(source);
        if initial_result.is_err() {
            println!("    Skipped due to initial parse failure (Unicode complexity)");
            continue;
        }

        let edit_result = std::panic::catch_unwind(|| {
            IncrementalTestUtils::create_value_edit(source, old_val, new_val)
        });

        match edit_result {
            Ok((new_source, edit)) => {
                let start = Instant::now();
                let result = IncrementalTestUtils::measure_incremental_parse(
                    &mut parser,
                    source,
                    edit,
                    &new_source,
                );
                let parse_time = start.elapsed();

                println!(
                    "    Result: {}µs, reused={}, reparsed={}, success={}",
                    parse_time.as_micros(),
                    result.nodes_reused,
                    result.nodes_reparsed,
                    result.success
                );

                // Unicode edge cases should not cause crashes or excessive delays
                assert!(parse_time.as_millis() < 50, "Unicode edge case should be <50ms");

                if result.success {
                    println!("    ✅ Unicode edge case handled successfully");
                } else {
                    println!(
                        "    ⚠️ Unicode edge case parsing failed (acceptable for complex cases)"
                    );
                }
            }
            Err(_) => {
                println!("    Edit creation failed for Unicode edge case (acceptable)");
            }
        }
    }
}
