//! Performance test for scope analysis
//! Run with: cargo test -p perl-semantic-analyzer --test scope_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, scope_analyzer::ScopeAnalyzer};
use std::time::Instant;

#[test]
#[ignore] // Only run when explicitly requested
fn benchmark_scope_analysis() {
    // Generate large test code
    let mut code = String::from("package TestPackage;\n\n");

    // Generate 500 subroutines with mixed content
    for i in 0..500 {
        code.push_str(&format!(
            r#"
sub test_{} {{
    my $x = 1;
    print "Hello";
    my @arr = (1, 2, 3);
    push @arr, 4;

    # Use some built-ins that are in PHF
    open(FH, "<", "file.txt");
    close(FH);
    my $len = length($x);

    # Use some that are NOT in PHF (fallback path)
    sysclose(FH);
    return unless defined $x;

    # Barewords (exercising is_known_function)
    my $t = time;
    my $r = rand;
    my $e = eof;
    next if $t;
    last unless $r;
}}
"#,
            i
        ));
    }

    println!("\nCode size: {} bytes", code.len());

    // Warm up
    let analyzer = ScopeAnalyzer::new();
    for _ in 0..3 {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let pragma_map = vec![];
            let _issues = analyzer.analyze(&ast, &code, &pragma_map);
        }
    }

    // Benchmark
    let iterations = 100;
    let mut total_time = std::time::Duration::ZERO;
    let mut total_issues = 0;

    for _ in 0..iterations {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let start = Instant::now();
            let pragma_map = vec![];
            let issues = analyzer.analyze(&ast, &code, &pragma_map);
            let duration = start.elapsed();

            total_time += duration;
            total_issues = issues.len();
        }
    }

    let avg_time = total_time / iterations;
    println!("\n=== Benchmark Results ===");
    println!("Average analysis time: {:?}", avg_time);
    println!("Total issues found: {}", total_issues);
}
