//! Demo of high-performance incremental parsing with <1ms updates
//!
//! Run with: cargo run -p perl-parser --example incremental_parsing_demo

use perl_parser::{
    incremental_document::IncrementalDocument,
    incremental_edit::IncrementalEdit,
};
use std::time::Instant;
use yansi::Paint;

fn main() {
    println!("🚀 {} {}", "Incremental Parsing Demo".bold().cyan(), "- Achieving <1ms updates".yellow());
    println!("{}", "=".repeat(60));
    
    // Initial source code
    let initial_source = r#"#!/usr/bin/perl
use strict;
use warnings;

# Configuration
my $debug = 0;
my $verbose = 1;
my $max_retries = 3;

# Main calculation
sub calculate {
    my ($a, $b) = @_;
    my $result = $a + $b;
    
    if ($debug) {
        print "Debug: $a + $b = $result\n";
    }
    
    return $result;
}

# Process data
sub process_data {
    my @numbers = (10, 20, 30, 40, 50);
    my $total = 0;
    
    foreach my $num (@numbers) {
        $total += $num;
        print "Processing: $num\n" if $verbose;
    }
    
    return $total;
}

# Main execution
my $sum = calculate(42, 58);
my $total = process_data();

print "Sum: $sum\n";
print "Total: $total\n";
"#;

    // Create incremental document
    println!("\n📄 {} initial document...", "Parsing".green());
    let start = Instant::now();
    let mut doc = IncrementalDocument::new(initial_source.to_string()).unwrap();
    let initial_time = start.elapsed();
    
    println!("✅ Initial parse completed in {:.2}ms", initial_time.as_secs_f64() * 1000.0);
    println!("   Nodes: {}", count_nodes(&doc));
    
    // Simulate realistic editing scenarios
    println!("\n📝 {} incremental edits...", "Applying".cyan());
    println!("{}", "-".repeat(60));
    
    // Edit 1: Change debug flag from 0 to 1
    perform_edit(&mut doc, "Edit 1: Enable debug mode", 
                 initial_source.find("debug = 0").unwrap() + 8,
                 initial_source.find("debug = 0").unwrap() + 9,
                 "1");
    
    // Edit 2: Change max_retries from 3 to 5
    let source = doc.text();
    perform_edit(&mut doc, "Edit 2: Increase max retries",
                 source.find("max_retries = 3").unwrap() + 14,
                 source.find("max_retries = 3").unwrap() + 15,
                 "5");
    
    // Edit 3: Change calculation values
    let source = doc.text();
    perform_edit(&mut doc, "Edit 3: Update calculation values",
                 source.find("calculate(42").unwrap() + 10,
                 source.find("calculate(42").unwrap() + 12,
                 "99");
    
    // Edit 4: Add a new number to the array
    let source = doc.text();
    let array_pos = source.find("50);").unwrap();
    perform_edit(&mut doc, "Edit 4: Add number to array",
                 array_pos + 2,
                 array_pos + 2,
                 ", 60, 70");
    
    // Edit 5: Change a string literal
    let source = doc.text();
    perform_edit(&mut doc, "Edit 5: Update output message",
                 source.find("\"Sum: ").unwrap() + 1,
                 source.find("\"Sum: ").unwrap() + 5,
                 "Result:");
    
    // Display final metrics
    println!("\n📊 {} Summary", "Performance".bold().green());
    println!("{}", "=".repeat(60));
    
    let metrics = doc.metrics();
    println!("📈 Parse Performance:");
    println!("   • Last parse time: {}{:.2}ms{}", 
             if metrics.last_parse_time_ms < 1.0 { "✅ ".green() } else { "⚠️ ".yellow() },
             metrics.last_parse_time_ms, 
             if metrics.last_parse_time_ms < 1.0 { " (< 1ms target)" } else { "" });
    
    println!("\n🔄 Subtree Reuse:");
    println!("   • Nodes reused: {}", metrics.nodes_reused.to_string().green());
    println!("   • Nodes reparsed: {}", metrics.nodes_reparsed.to_string().yellow());
    let reuse_rate = if metrics.nodes_reused + metrics.nodes_reparsed > 0 {
        100.0 * metrics.nodes_reused as f64 / (metrics.nodes_reused + metrics.nodes_reparsed) as f64
    } else {
        0.0
    };
    println!("   • Reuse rate: {:.1}%", reuse_rate);
    
    println!("\n💾 Cache Performance:");
    println!("   • Cache hits: {}", metrics.cache_hits.to_string().green());
    println!("   • Cache misses: {}", metrics.cache_misses.to_string().yellow());
    let hit_rate = if metrics.cache_hits + metrics.cache_misses > 0 {
        100.0 * metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64
    } else {
        0.0
    };
    println!("   • Hit rate: {:.1}%", hit_rate);
    
    // Benchmark series of rapid edits
    println!("\n⚡ {} (100 rapid edits)", "Stress Test".bold().yellow());
    println!("{}", "-".repeat(60));
    
    let mut times = Vec::new();
    let source = doc.text();
    
    for i in 0..100 {
        // Find a number to change
        if let Some(pos) = source.find("42") {
            let new_value = (42 + i).to_string();
            let edit = IncrementalEdit::new(
                pos,
                pos + 2,
                new_value,
            );
            
            let start = Instant::now();
            doc.apply_edit(edit).unwrap();
            let elapsed = start.elapsed();
            times.push(elapsed.as_secs_f64() * 1000.0);
        }
    }
    
    if !times.is_empty() {
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        let median = times[times.len() / 2];
        let p95 = times[(times.len() as f64 * 0.95) as usize];
        let p99 = times[(times.len() as f64 * 0.99) as usize];
        
        println!("📊 Incremental Parse Times (100 edits):");
        println!("   • Average: {:.3}ms", avg);
        println!("   • Median: {:.3}ms", median);
        println!("   • 95th percentile: {:.3}ms", p95);
        println!("   • 99th percentile: {:.3}ms", p99);
        println!("   • Best: {:.3}ms", times[0]);
        println!("   • Worst: {:.3}ms", times[times.len() - 1]);
        
        let under_1ms = times.iter().filter(|&&t| t < 1.0).count();
        println!("\n✅ {}/{} edits completed in <1ms ({:.1}%)", 
                under_1ms, times.len(), 
                100.0 * under_1ms as f64 / times.len() as f64);
    }
    
    println!("\n🎉 {} Complete!", "Demo".bold().green());
    println!("Incremental parsing enables real-time IDE experiences with <1ms updates!");
}

fn perform_edit(doc: &mut IncrementalDocument, description: &str, start: usize, end: usize, new_text: &str) {
    println!("\n  {}", description.cyan());
    
    let edit = IncrementalEdit::new(start, end, new_text.to_string());
    
    let start_time = Instant::now();
    doc.apply_edit(edit).unwrap();
    let elapsed = start_time.elapsed();
    
    let time_ms = elapsed.as_secs_f64() * 1000.0;
    let metrics = doc.metrics();
    
    print!("    • Time: ");
    if time_ms < 1.0 {
        print!("{}{:.3}ms{}", "✅ ".green(), time_ms, " (< 1ms)".green());
    } else if time_ms < 5.0 {
        print!("{}{:.3}ms", "⚠️ ".yellow(), time_ms);
    } else {
        print!("{}{:.3}ms", "❌ ".red(), time_ms);
    }
    
    println!(" | Reused: {} | Reparsed: {}", 
            metrics.nodes_reused.to_string().green(),
            metrics.nodes_reparsed.to_string().yellow());
}

fn count_nodes(doc: &IncrementalDocument) -> usize {
    count_nodes_in_ast(doc.tree())
}

fn count_nodes_in_ast(node: &perl_parser::ast::Node) -> usize {
    use perl_parser::ast::NodeKind;
    
    let mut count = 1;
    
    match &node.kind {
        NodeKind::Program { statements } |
        NodeKind::Block { statements } => {
            for stmt in statements {
                count += count_nodes_in_ast(stmt);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            count += count_nodes_in_ast(left);
            count += count_nodes_in_ast(right);
        }
        NodeKind::Unary { operand, .. } => {
            count += count_nodes_in_ast(operand);
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                count += count_nodes_in_ast(arg);
            }
        }
        _ => {}
    }
    
    count
}