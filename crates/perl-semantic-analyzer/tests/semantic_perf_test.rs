//! Performance test for semantic analysis of builtin functions
//! Run with: cargo test -p perl-semantic-analyzer --test semantic_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::semantic::SemanticAnalyzer;
use std::time::Instant;

#[test]
#[ignore] // Only run when explicitly requested
fn benchmark_builtin_analysis() {
    // Generate large test code with many builtin calls
    let mut code = String::from("package TestPackage;\n\nsub test_sub {\n");

    // Add 10,000 builtin function calls
    // We use a mix of builtins that are in the old list and new ones
    // to test both hit rate and general performance
    for i in 0..10000 {
        // Alternating between common builtins
        if i % 5 == 0 {
            code.push_str("    print \"hello\";\n");
        } else if i % 5 == 1 {
            code.push_str("    my $x = abs(-42);\n"); // abs was missing
        } else if i % 5 == 2 {
            code.push_str("    mkdir \"dir\";\n"); // mkdir was missing
        } else if i % 5 == 3 {
            code.push_str("    close($fh);\n");
        } else {
            // A control keyword to ensure we test the branching logic
            code.push_str("    next if $x;\n");
        }
    }
    code.push_str("}\n");

    println!("\nCode size: {} bytes", code.len());

    // Parse first (outside benchmark loop to isolate analysis)
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse benchmark code");

    // Warm up
    for _ in 0..5 {
        let _analyzer = SemanticAnalyzer::analyze_with_source(&ast, &code);
    }

    // Benchmark
    let iterations = 20;
    let mut total_time = std::time::Duration::ZERO;
    let mut token_count = 0;

    for _ in 0..iterations {
        let start = Instant::now();
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, &code);
        let duration = start.elapsed();

        total_time += duration;
        token_count = analyzer.semantic_tokens().len();
    }

    let avg_time = total_time / iterations;
    println!("\n=== Semantic Analysis Benchmark Results ===");
    println!("Average analysis time: {:?}", avg_time);
    println!("Total semantic tokens: {}", token_count);

    // Performance requirement: arbitrary baseline, but we want to see improvement
    // The main metric is relative improvement
    println!(
        "Tokens per millisecond: {:.2}",
        token_count as f64 / avg_time.as_millis().max(1) as f64
    );
}
