//! Simple demo of incremental parsing with performance metrics
//!
//! Run with: cargo run -p perl-parser --example incremental_simple_demo

use perl_parser::{
    incremental_document::IncrementalDocument,
    incremental_edit::IncrementalEdit,
};
use std::time::Instant;

fn main() {
    println!("=== Incremental Parsing Demo - Targeting <1ms Updates ===\n");
    
    // Initial source
    let source = r#"
my $x = 42;
my $y = 100;
my $z = $x + $y;
print "Result: $z\n";
"#;

    // Parse initial document
    println!("Parsing initial document...");
    let start = Instant::now();
    let mut doc = IncrementalDocument::new(source.to_string()).unwrap();
    let initial_time = start.elapsed().as_secs_f64() * 1000.0;
    
    println!("Initial parse: {:.3}ms\n", initial_time);
    
    // Test 1: Change a number (42 -> 99)
    println!("Test 1: Changing number 42 to 99");
    let edit = IncrementalEdit::new(
        source.find("42").unwrap(),
        source.find("42").unwrap() + 2,
        "99".to_string(),
    );
    
    let start = Instant::now();
    doc.apply_edit(edit).unwrap();
    let time = start.elapsed().as_secs_f64() * 1000.0;
    
    let metrics = doc.metrics();
    println!("  Time: {:.3}ms {}", time, if time < 1.0 { "(✓ <1ms)" } else { "" });
    println!("  Nodes reused: {}", metrics.nodes_reused);
    println!("  Nodes reparsed: {}", metrics.nodes_reparsed);
    println!("  Cache hits: {}", metrics.cache_hits);
    println!();
    
    // Test 2: Change another number (100 -> 200)
    let source = doc.text();
    println!("Test 2: Changing number 100 to 200");
    let edit = IncrementalEdit::new(
        source.find("100").unwrap(),
        source.find("100").unwrap() + 3,
        "200".to_string(),
    );
    
    let start = Instant::now();
    doc.apply_edit(edit).unwrap();
    let time = start.elapsed().as_secs_f64() * 1000.0;
    
    let metrics = doc.metrics();
    println!("  Time: {:.3}ms {}", time, if time < 1.0 { "(✓ <1ms)" } else { "" });
    println!("  Nodes reused: {}", metrics.nodes_reused);
    println!("  Nodes reparsed: {}", metrics.nodes_reparsed);
    println!("  Cache hits: {}", metrics.cache_hits);
    println!();
    
    // Test 3: Change variable name ($x -> $value)
    let source = doc.text();
    println!("Test 3: Renaming variable $x to $value");
    if let Some(pos) = source.find("$x") {
        let edit = IncrementalEdit::new(
            pos,
            pos + 2,
            "$value".to_string(),
        );
        
        let start = Instant::now();
        doc.apply_edit(edit).unwrap();
        let time = start.elapsed().as_secs_f64() * 1000.0;
        
        let metrics = doc.metrics();
        println!("  Time: {:.3}ms {}", time, if time < 1.0 { "(✓ <1ms)" } else { "" });
        println!("  Nodes reused: {}", metrics.nodes_reused);
        println!("  Nodes reparsed: {}", metrics.nodes_reparsed);
        println!();
    }
    
    // Benchmark: 50 rapid edits
    println!("Benchmark: 50 rapid number changes");
    let mut times = Vec::new();
    
    for i in 0..50 {
        let source = doc.text();
        if let Some(pos) = source.find("99") {
            let new_value = (99 + i).to_string();
            let edit = IncrementalEdit::new(
                pos,
                pos + 2,
                new_value,
            );
            
            let start = Instant::now();
            doc.apply_edit(edit).unwrap();
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    
    // Calculate statistics
    let avg = if !times.is_empty() {
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_time = times.iter().sum::<f64>() / times.len() as f64;
        let under_1ms = times.iter().filter(|&&t| t < 1.0).count();
        
        println!("  Average: {:.3}ms", avg_time);
        println!("  Median: {:.3}ms", times[times.len() / 2]);
        println!("  Best: {:.3}ms", times[0]);
        println!("  Worst: {:.3}ms", times[times.len() - 1]);
        println!("  Under 1ms: {}/{} ({:.0}%)\n", 
                under_1ms, times.len(), 
                100.0 * under_1ms as f64 / times.len() as f64);
        avg_time
    } else {
        f64::MAX
    };
    
    // Final statistics
    let final_metrics = doc.metrics();
    println!("=== Final Summary ===");
    println!("Document version: {}", doc.version);
    println!("Total nodes reused: {}", final_metrics.nodes_reused);
    println!("Total nodes reparsed: {}", final_metrics.nodes_reparsed);
    
    let reuse_rate = if final_metrics.nodes_reused + final_metrics.nodes_reparsed > 0 {
        100.0 * final_metrics.nodes_reused as f64 / 
        (final_metrics.nodes_reused + final_metrics.nodes_reparsed) as f64
    } else {
        0.0
    };
    println!("Overall reuse rate: {:.1}%", reuse_rate);
    
    println!("\n✓ Incremental parsing demo complete!");
    if avg < 1.0 {
        println!("✓ Achieved <1ms average parse time!");
    }
}