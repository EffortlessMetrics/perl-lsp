//! Performance test for scope analyzer
//! Run with: cargo test -p perl-semantic-analyzer --test scope_analyzer_perf -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, scope_analyzer::ScopeAnalyzer};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_scope_analysis() {
    // Generate large test code with many barewords
    let mut code = String::from("use strict;\n");

    // Generate 10,000 barewords.
    // These will be parsed as Identifiers and trigger is_known_function check.
    // Half are unknown (false), half are known (true) IF parsed as identifier.
    // To ensure they are identifiers, we can use them in a way that parser likely treats as such,
    // or just rely on unknown ones.

    for i in 0..10000 {
        code.push_str(&format!(
            r#"
unknown_func_{};
items_{};
check_{};
verify_{};
validate_{};
"#,
            i, i, i, i, i
        ));
    }

    println!("\nCode size: {} bytes", code.len());

    // Parse once
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");
    let pragma_map = vec![]; // Simplified for benchmark

    // Warm up
    let analyzer = ScopeAnalyzer::new();
    for _ in 0..5 {
        let _ = analyzer.analyze(&ast, &code, &pragma_map);
    }

    // Benchmark
    let iterations = 20;
    let mut total_time = std::time::Duration::ZERO;
    let mut issue_count = 0;

    for _ in 0..iterations {
        let start = Instant::now();
        let issues = analyzer.analyze(&ast, &code, &pragma_map);
        let duration = start.elapsed();
        total_time += duration;
        issue_count = issues.len();
    }

    let avg_time = total_time / iterations;
    println!("\n=== Benchmark Results ===");
    println!("Average analysis time: {:?}", avg_time);
    println!("Total issues found: {}", issue_count);
}
