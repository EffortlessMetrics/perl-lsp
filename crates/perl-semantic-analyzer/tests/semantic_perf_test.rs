//! Performance test for SemanticAnalyzer
//! Run with: cargo test -p perl-semantic-analyzer --test semantic_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, analysis::semantic::SemanticAnalyzer};
use std::time::Instant;

#[test]
#[ignore] // Only run when explicitly requested
fn benchmark_semantic_analysis_builtins() {
    // Generate large test code with many builtin calls
    let mut code = String::from("package TestPackage;\n\n");
    code.push_str("sub test_builtins {\n");

    // Add 1,000 calls to various builtins
    // Mix of ones present in semantic.rs (e.g. print) and ones missing (e.g. sysclose if I add it, or others)
    // Actually, let's use common ones to stress the matcher.
    for _ in 0..1000 {
        code.push_str("    print \"hello\";\n");
        code.push_str("    mkdir(\"dir\");\n"); // Was missing
        code.push_str("    rmdir(\"dir\");\n"); // Was missing
        code.push_str("    chmod(0755, \"file\");\n"); // Was missing
        code.push_str("    my $len = length(\"string\");\n");
        code.push_str("    push(@arr, 1);\n");
        code.push_str("    socket(my $sock, 1, 2, 3);\n"); // Was missing
        code.push_str("    bind($sock, 1);\n"); // Was missing
        code.push_str("    listen($sock, 1);\n"); // Was missing
        code.push_str("    accept(my $new, $sock);\n"); // Was missing
    }
    code.push_str("}\n");

    println!("\nCode size: {} bytes", code.len());
    println!("Estimated {} builtin calls", 1000 * 10);

    // Warm up
    for _ in 0..3 {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let _analyzer = SemanticAnalyzer::analyze(&ast);
        }
    }

    // Benchmark
    let iterations = 10;
    let mut total_time = std::time::Duration::ZERO;

    for _ in 0..iterations {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let start = Instant::now();
            let _analyzer = SemanticAnalyzer::analyze(&ast);
            let duration = start.elapsed();

            total_time += duration;
            // println!("Iteration time: {:?}", duration);
        }
    }

    let avg_time = total_time / iterations;
    println!("\n=== Benchmark Results (SemanticAnalyzer) ===");
    println!("Average analysis time: {:?}", avg_time);
    println!("Total builtin calls: {}", 10000);
    println!("Calls per millisecond: {:.0}", 10000.0 / avg_time.as_millis().max(1) as f64);
}
