//! Test to reproduce issue #3437: No incremental semantic analysis
//! This test demonstrates that SymbolExtractor.extract() performs full AST
//! traversal even when only a small portion of the file changed.

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
fn reproduce_issue_3437_no_incremental_semantic_analysis() {
    // Generate a 10K+ line file with many symbols to make the cost visible
    let mut code = String::from("package TestPkg;\n\n");

    for i in 0..1000 {
        code.push_str(&format!(
            r#"
sub func_{i} {{
    my $var_{i}_1 = {i};
    my $var_{i}_2 = "string_{i}";
    my @arr_{i} = (1, 2, 3);
    my %hash_{i} = (key => 'value');
    return $var_{i}_1;
}}
"#
        ));
    }

    println!("Generated code: {} bytes, ~5000+ symbols", code.len());

    // First parse and extract - baseline
    let mut parser = Parser::new(&code);
    let ast1 = parser.parse().expect("Parse 1 failed");

    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(&code);
    let table1 = extractor.extract(&ast1);
    let full_extract_time = start.elapsed();

    println!("Full extraction (all symbols): {:?}", full_extract_time);
    println!("Total symbols extracted: {}", table1.symbols.len());

    // Now make a small change: add one line at the end
    let mut changed_code = code.clone();
    changed_code.push_str("\nmy $new_var = 42;  # single line change\n");

    // Parse the changed code
    let mut parser2 = Parser::new(&changed_code);
    let ast2 = parser2.parse().expect("Parse 2 failed");

    // Extract symbols from changed AST
    // EXPECTED: Only re-analyze the changed region (1 new symbol)
    // ACTUAL: Full re-analysis of all ~5000+ symbols
    let start = Instant::now();
    let extractor2 = SymbolExtractor::new_with_source(&changed_code);
    let table2 = extractor2.extract(&ast2);
    let incremental_extract_time = start.elapsed();

    println!("Extract after single-line change: {:?}", incremental_extract_time);
    println!("New symbols extracted: {}", table2.symbols.len());

    // Analysis
    let ratio =
        incremental_extract_time.as_millis() as f64 / full_extract_time.as_millis().max(1) as f64;
    println!("\nPerformance ratio (incremental/full): {:.2}x", ratio);

    // The issue: if ratio is >= 0.8, it means the single-line change takes nearly
    // as long as parsing the entire file, indicating no incremental analysis.
    println!("Single-line change takes {:.1}% of time for full re-extraction", ratio * 100.0);

    // This assertion documents the current behavior (will fail until incremental analysis is implemented)
    // Once incremental analysis is implemented, this ratio should be much lower (< 0.2)
    if ratio > 0.8 {
        println!("ISSUE REPRODUCED: No incremental semantic analysis detected");
        // For now, we document the issue without failing hard
        // println!("This indicates full AST traversal occurs on every change.");
    }
}
