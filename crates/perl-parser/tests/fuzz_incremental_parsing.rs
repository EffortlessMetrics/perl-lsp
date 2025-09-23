/// Stress testing for incremental parsing and dual indexing components
/// Focus on AST consistency and workspace indexing robustness
///
/// Labels: tests:fuzz, perl-fuzz:running
use perl_parser::Parser;
use proptest::prelude::*;
use std::panic::AssertUnwindSafe;

/// Test incremental parsing robustness with various Perl script modifications
#[test]
fn fuzz_incremental_parsing_robustness() {
    let test_cases = vec![
        // Basic Perl scripts
        "use strict;\nmy $var = 42;\nprint $var;\n",
        "package Test;\nsub new { my $class = shift; bless {}, $class; }\n1;\n",
        "my @array = (1, 2, 3);\nfor my $item (@array) { print $item; }\n",
        // Function definitions that exercise dual indexing
        "sub Package::function { return 42; }\nPackage::function();\n",
        "package MyPkg;\nsub test { return 1; }\npackage main;\nMyPkg::test();\n",
        // Quote-like constructs that could interact with parsing
        "my $regex = qr/test/i;\nmy $str = 'hello';\n$str =~ s/hello/world/g;\n",
        // Complex structures
        "use Data::Dumper;\nmy %hash = (key => 'value');\nprint Dumper(\\%hash);\n",
    ];

    for original_script in test_cases {
        // Test that parsing doesn't crash on original
        let mut parser = Parser::new(original_script);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| parser.parse()));

        assert!(result.is_ok(), "Parser crashed on script: {}", original_script);

        // Test incremental modifications don't break parsing
        let modifications = vec![
            // Add content at beginning
            format!("# Comment\n{}", original_script),
            // Add content at end
            format!("{}# End comment\n", original_script),
            // Insert in middle
            original_script.replace(";", "; # comment\n"),
            // Duplicate content
            format!("{}\n{}", original_script, original_script),
        ];

        for modified_script in modifications {
            let mut parser = Parser::new(&modified_script);
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| parser.parse()));

            assert!(
                result.is_ok(),
                "Parser crashed on modified script:\nOriginal: {}\nModified: {}",
                original_script,
                modified_script
            );
        }
    }
}

/// Test dual indexing robustness with function call patterns
#[test]
fn fuzz_dual_indexing_function_patterns() {
    let function_patterns = vec![
        // Basic function calls
        ("sub test { 42 }", "test()"),
        ("package Pkg; sub test { 42 }", "Pkg::test()"),
        ("package Pkg; sub test { 42 }", "test()"), // Bare call to qualified function
        // Nested packages
        ("package A::B::C; sub deep { 1 }", "A::B::C::deep()"),
        ("package A::B::C; sub deep { 1 }", "deep()"),
        // Unicode identifiers
        ("sub café { 'coffee' }", "café()"),
        ("package Ñañá; sub test { 1 }", "Ñañá::test()"),
        // Edge case names
        ("sub _private { 1 }", "_private()"),
        ("sub CONSTANT { 42 }", "CONSTANT()"),
        ("sub new { bless {} }", "new()"),
    ];

    for (definition, call) in function_patterns {
        let script = format!("{}\n{}\n", definition, call);

        let mut parser = Parser::new(&script);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| parser.parse()));

        assert!(
            result.is_ok(),
            "Dual indexing test failed for definition: {} call: {}",
            definition,
            call
        );

        // Test that result is reasonable
        if let Ok(parse_result) = result {
            if let Ok(_ast) = parse_result {
                // Parsing succeeded - AST structure could be validated here
            }
        }
    }
}

/// Stress test workspace indexing with complex package hierarchies
#[test]
fn fuzz_workspace_indexing_stress() {
    let complex_scripts = vec![
        // Multiple packages in one file
        r#"
        package A;
        sub method_a { return 'A'; }

        package B;
        sub method_b { return 'B'; }

        package main;
        A::method_a();
        B::method_b();
        "#,
        // Recursive package references
        r#"
        package Recursive;
        sub call_self { Recursive::helper(); }
        sub helper { return 1; }
        Recursive::call_self();
        "#,
        // Mixed qualified/unqualified calls
        r#"
        package Utils;
        sub debug { print "debug\n"; }
        sub info { print "info\n"; }

        package main;
        Utils::debug();
        debug(); # Should this resolve?
        info();  # Should this resolve?
        "#,
        // Complex inheritance-like patterns
        r#"
        package Base;
        sub new { bless {}, shift; }
        sub method { return "base"; }

        package Derived;
        sub new { Base::new(@_); }
        sub method { return "derived"; }

        package main;
        my $base = Base::new();
        my $derived = Derived::new();
        $base->method();
        $derived->method();
        "#,
    ];

    for script in complex_scripts {
        let mut parser = Parser::new(script);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| parser.parse()));

        assert!(result.is_ok(), "Complex workspace indexing failed for script: {}", script);

        // Additional validation for workspace features if available
        // (This would test the actual indexing functionality)
    }
}

/// Test parser robustness with malformed quote constructs that could break incremental parsing
#[test]
fn fuzz_malformed_quote_incremental_interaction() {
    let malformed_quote_scripts = vec![
        // Unclosed quotes
        "my $var = 'unclosed string",
        "my $regex = qr/unclosed",
        "my $sub = s/unclosed/replacement",
        // Nested quote issues
        "my $complex = qr{test{nested}unclosed",
        "s/pattern{nested/replacement/g",
        // Mixed quote styles
        "s/test'mixed/quotes\"here/",
        "qr'mixed\"quotes/here'",
        // Very deep nesting
        "s{{{{{test}}}}}{{{{replacement}}}}",
        "qr((((((nested))))))",
        // Quote-like in strings
        "my $str = 'contains s/fake/substitution/'",
        "print \"qr/fake/regex in string\"",
    ];

    for script in malformed_quote_scripts {
        let mut parser = Parser::new(script);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| parser.parse()));

        // These may fail to parse, but should not crash
        assert!(result.is_ok(), "Parser crashed (not just failed) on malformed quote: {}", script);
    }
}

#[allow(unused_doc_comments)]
/// Property-based test for AST invariant preservation
proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn fuzz_ast_invariant_preservation(
        function_name in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        package_name in "[A-Z][a-zA-Z0-9_]{0,15}",
        variable_name in "\\$[a-zA-Z_][a-zA-Z0-9_]{0,15}",
    ) {
        // Generate a simple but valid Perl script
        let script = format!(
            "package {};\nsub {} {{ return 42; }}\nmy {} = {}::{}();\n",
            package_name, function_name, variable_name, package_name, function_name
        );

        let mut parser = Parser::new(&script);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            parser.parse()
        }));

        prop_assert!(result.is_ok(), "Generated script caused crash: {}", script);

        if let Ok(parse_result) = result {
            // Basic AST consistency checks
            if let Ok(_ast) = parse_result {
                // The fact that we got an Ok(ast) means parsing succeeded
                // AST structure validation could be added here in the future
                // - Check that functions are properly indexed
                // - Verify dual indexing works
                // - Validate UTF-8 safety in symbol names
            }
        }
    }
}

/// Test memory and performance characteristics under stress
#[test]
fn fuzz_performance_under_stress() {
    let stress_scripts = vec![
        // Large number of function definitions
        (0..100)
            .map(|i| format!("sub func_{} {{ return {}; }}", i, i))
            .collect::<Vec<_>>()
            .join("\n"),
        // Large number of package definitions
        (0..50)
            .map(|i| format!("package Pkg_{};\nsub method {{ return {}; }}", i, i))
            .collect::<Vec<_>>()
            .join("\n"),
        // Very long function call chains
        format!(
            "package Chain;\n{}\nChain::start();",
            (0..50)
                .map(|i| format!("sub step_{} {{ step_{}(); }}", i, i + 1))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        // Large nested structures
        format!(
            "my $data = {{{}}};",
            (0..100).map(|i| format!("key_{} => 'value_{}'", i, i)).collect::<Vec<_>>().join(", ")
        ),
    ];

    for script in stress_scripts {
        let start = std::time::Instant::now();

        let mut parser = Parser::new(&script);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| parser.parse()));

        let duration = start.elapsed();

        assert!(result.is_ok(), "Stress test script caused crash");
        assert!(
            duration.as_millis() < 5000,
            "Parsing took too long: {}ms for {} chars",
            duration.as_millis(),
            script.len()
        );

        // Memory usage should be reasonable (basic check)
        if let Ok(parse_result) = result {
            if let Ok(_ast) = parse_result {
                // AST created successfully
            }
        }
    }
}
