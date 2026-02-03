//! Performance test for scope analysis
//! Run with: cargo test -p perl-semantic-analyzer --test scope_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, scope_analyzer::ScopeAnalyzer};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_scope_analysis() {
    // Generate large test code
    let mut code = String::from("package TestPackage;\n\n");
    // Add strict mode to trigger bareword checks where is_known_function is used
    code.push_str("use strict;\n");

    // Generate 500 subroutines with variables and function calls
    // We want to trigger is_known_function calls (for bareword checking)
    for i in 0..500 {
        code.push_str(&format!(
            r#"
sub test_{} {{
    my $x_{} = {};
    print "Hello";
    my $y = abs($x_{});
    if (defined $y) {{
        return $y;
    }}
    # Some barewords/function calls that will be checked against builtins
    split(/,/, "a,b,c");
    join("-", 1, 2, 3);
    keys %ENV;
    values %ENV;
    each %ENV;
    delete $ENV{{PATH}};
    exists $ENV{{PATH}};
    chomp($y);
    chop($y);
    uc($y);
    lc($y);
    length($y);
    substr($y, 0, 1);
    index($y, "H");
    rindex($y, "o");
    sprintf("%s", $y);
    printf("%s", $y);
    say $y;

    # Some control flow keywords
    next if $y;
    last if $y;
    redo if $y;

    # Some math
    sin(1); cos(1); exp(1); log(1); sqrt(1); int(1.5); rand(10);

    # Some system
    time();
}}
"#,
            i, i, i, i
        ));
    }

    println!("\nCode size: {} bytes", code.len());

    // Warm up
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![];
    for _ in 0..3 {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
             let _issues = analyzer.analyze(&ast, &code, &pragma_map);
        }
    }

    // Benchmark
    let iterations = 100;
    let mut total_time = std::time::Duration::ZERO;
    let mut issue_count = 0;

    for _ in 0..iterations {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let start = Instant::now();
            let issues = analyzer.analyze(&ast, &code, &pragma_map);
            let duration = start.elapsed();

            total_time += duration;
            issue_count = issues.len();
            assert_eq!(issue_count, 0, "Optimization caused regression: found {} issues where 0 were expected", issue_count);
        }
    }

    let avg_time = total_time / iterations;
    println!("\n=== Benchmark Results ===");
    println!("Average analysis time: {:?}", avg_time);
    println!("Total issues: {}", issue_count);
}
