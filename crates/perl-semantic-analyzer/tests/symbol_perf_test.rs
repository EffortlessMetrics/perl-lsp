//! Performance test for symbol extraction
//! Run with: cargo test -p perl-semantic-analyzer --test symbol_perf_test -- --nocapture

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::{Duration, Instant};

#[test]
fn benchmark_symbol_extraction() {
    // Generate large test code
    let mut code = String::from("package TestPackage;\n\n");

    // Generate 100 subroutines with variables
    for i in 0..100 {
        code.push_str(&format!(
            r#"
# This is a comment for sub test_{}
# It describes what the subroutine does
sub test_{} {{
    my $x_{} = {};
    my $y_{} = "string_{}";
    my @arr_{} = (1, 2, 3);
    my %hash_{} = (key => 'value');

    return $x_{} + $y_{};
}}
"#,
            i, i, i, i, i, i, i, i, i, i
        ));
    }

    println!("\nCode size: {} bytes", code.len());
    println!("Estimated {} symbols", 100 * 5); // 100 subs * ~5 symbols each

    // Warm up
    for _ in 0..3 {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let extractor = SymbolExtractor::new_with_source(&code);
            let _table = extractor.extract(&ast);
        }
    }

    // Benchmark
    let iterations = 5;
    let mut total_time = std::time::Duration::ZERO;
    let mut symbol_count = 0;
    let mut ref_count = 0;

    for _ in 0..iterations {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let start = Instant::now();
            let extractor = SymbolExtractor::new_with_source(&code);
            let table = extractor.extract(&ast);
            let duration = start.elapsed();

            total_time += duration;
            symbol_count = table.symbols.len();
            ref_count = table.references.len();
            println!(
                "Symbols extracted: {}, References: {}, Time: {:?}",
                symbol_count, ref_count, duration
            );
        }
    }

    let avg_time = total_time / iterations;
    println!("\n=== Benchmark Results ===");
    println!("Average extraction time: {:?}", avg_time);
    println!("Total symbols: {}", symbol_count);
    println!("Total references: {}", ref_count);
    println!(
        "Symbols per millisecond: {:.0}",
        symbol_count as f64 / avg_time.as_millis().max(1) as f64
    );

    // Performance requirement: should process at least 1000 symbols/sec
    let symbols_per_sec = (symbol_count as f64 * 1000.0) / avg_time.as_millis().max(1) as f64;
    println!("Symbols per second: {:.0}", symbols_per_sec);

    assert!(
        symbol_count >= 500,
        "Expected at least 500 symbols from synthetic workload, found {symbol_count}"
    );
    assert!(
        ref_count >= 100,
        "Expected reference extraction to produce entries, found {ref_count}"
    );
    assert!(
        avg_time <= Duration::from_secs(2),
        "Symbol extraction too slow for default lane: {:?}",
        avg_time
    );
}

#[test]
fn test_symbol_extraction_with_comments() {
    let code = r#"
package Example;

# This is a variable
# With multiline documentation
my $documented_var = 42;

# This is a subroutine
# that does something interesting
# and has detailed documentation
sub documented_sub {
    my $x = 1;
    return $x;
}
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(code);
    let table = extractor.extract(&ast);
    let duration = start.elapsed();

    println!("Documentation extraction time: {:?}", duration);

    // Verify documentation was extracted
    assert!(table.symbols.contains_key("documented_var"));
    assert!(table.symbols.contains_key("documented_sub"));

    let var_symbols = &table.symbols["documented_var"];
    assert_eq!(var_symbols.len(), 1);
    assert!(var_symbols[0].documentation.is_some());

    let sub_symbols = &table.symbols["documented_sub"];
    assert_eq!(sub_symbols.len(), 1);
    assert!(sub_symbols[0].documentation.is_some());
}
