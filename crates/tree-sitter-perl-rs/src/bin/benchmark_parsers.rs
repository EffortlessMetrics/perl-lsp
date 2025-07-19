//! Comprehensive benchmark runner for parser implementations

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::{
    parser_benchmark::{BenchmarkSuite, ParserBenchmark, ParserImpl},
    pure_rust_parser::PureRustPerlParser,
};

#[cfg(not(feature = "pure-rust"))]
fn main() {
    eprintln!("This benchmark requires the pure-rust feature");
    std::process::exit(1);
}

#[cfg(feature = "pure-rust")]
fn main() {
    println!("🚀 Parser Implementation Benchmark\n");
    
    let mut suite = BenchmarkSuite::new();
    
    // Test cases with increasing complexity
    let test_cases = vec![
        ("Simple assignment", "$x = 42"),
        ("Array literal", "@arr = (1, 2, 3, 4, 5)"),
        ("Hash literal", "%hash = (a => 1, b => 2, c => 3)"),
        ("Function call", "print(\"Hello, World!\\n\")"),
        ("Complex expression", "$result = ($a + $b) * ($c - $d) / $e"),
    ];
    
    // Run benchmarks for each test case
    for (name, input) in test_cases {
        println!("Benchmarking: {}", name);
        benchmark_input(&mut suite, name, input, 100);
    }
    
    // Test deep nesting (only in debug mode to show the difference)
    #[cfg(debug_assertions)]
    {
        println!("\n🔍 Debug Mode Deep Nesting Tests\n");
        
        for depth in [100, 500, 1000] {
            let mut expr = "42".to_string();
            for _ in 0..depth {
                expr = format!("({})", expr);
            }
            
            let name = format!("Nested depth {}", depth);
            println!("Benchmarking: {}", name);
            
            // For deep nesting, only test implementations that should work
            benchmark_deep_nesting(&mut suite, &name, &expr, depth);
        }
    }
    
    // Print results
    println!("\n" + "=".repeat(80).as_str());
    suite.summary();
    
    // Comparisons
    println!("\n📊 Performance Comparisons\n");
    suite.compare("Simple assignment - Recursive", "Simple assignment - Iterative");
    suite.compare("Simple assignment - Recursive + Stacker", "Simple assignment - Iterative");
    
    #[cfg(debug_assertions)]
    {
        println!("\n📊 Deep Nesting Comparisons (Debug Mode)\n");
        suite.compare("Nested depth 1000 - Stacker", "Nested depth 1000 - Iterative");
    }
}

#[cfg(feature = "pure-rust")]
fn benchmark_input(suite: &mut BenchmarkSuite, name: &str, input: &str, iterations: usize) {
    use tree_sitter_perl::bench_parsers;
    
    // Run all three implementations
    bench_parsers!(suite, input, iterations);
    
    // Rename the results to include the test name
    if let Some(results) = suite.results.remove("Recursive") {
        suite.results.insert(format!("{} - Recursive", name), results);
    }
    if let Some(results) = suite.results.remove("Recursive + Stacker") {
        suite.results.insert(format!("{} - Recursive + Stacker", name), results);
    }
    if let Some(results) = suite.results.remove("Iterative") {
        suite.results.insert(format!("{} - Iterative", name), results);
    }
}

#[cfg(feature = "pure-rust")]
fn benchmark_deep_nesting(suite: &mut BenchmarkSuite, name: &str, input: &str, depth: usize) {
    // For deep nesting, recursive without stacker will likely fail in debug
    #[cfg(debug_assertions)]
    if depth <= 100 {
        suite.bench(&format!("{} - Recursive", name), 1, || {
            let mut bench = ParserBenchmark::new();
            bench.bench_parser(ParserImpl::Recursive, input)
                .map(|_| ())
                .map_err(|e| e.to_string())
        });
    }
    
    // These should work even with deep nesting
    suite.bench(&format!("{} - Stacker", name), 3, || {
        let mut bench = ParserBenchmark::new();
        bench.bench_parser(ParserImpl::RecursiveWithStacker, input)
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    
    suite.bench(&format!("{} - Iterative", name), 3, || {
        let mut bench = ParserBenchmark::new();
        bench.bench_parser(ParserImpl::Iterative, input)
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
}