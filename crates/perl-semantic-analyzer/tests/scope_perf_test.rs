//! Performance test for scope analysis
//! Run with: cargo test -p perl-semantic-analyzer --test scope_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, scope_analyzer::ScopeAnalyzer};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_scope_analysis() {
    // Generate code with MANY barewords to stress is_known_function
    let mut code = String::with_capacity(10_000_000);
    code.push_str("use strict;\n");
    code.push_str("sub test {\n");

    // 100,000 print statements
    for _ in 0..100_000 {
        code.push_str("print; abs; defined; say; open; close; chomp; sort; map; keys;\n");
    }
    code.push_str("}\n");

    println!("Code size: {} bytes", code.len());

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");
    let pragma_map = vec![];

    // Warm up
    let analyzer = ScopeAnalyzer::new();
    let _ = analyzer.analyze(&ast, &code, &pragma_map);

    let iterations = 10;
    let mut total_time = std::time::Duration::ZERO;

    for _ in 0..iterations {
        let start = Instant::now();
        let issues = analyzer.analyze(&ast, &code, &pragma_map);
        let duration = start.elapsed();
        total_time += duration;

        println!("Analysis iteration: {:?}, issues found: {}", duration, issues.len());
    }

    let avg_time = total_time / iterations;
    println!("Average scope analysis time: {:?}", avg_time);
}
