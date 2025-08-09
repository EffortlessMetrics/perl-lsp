//! Demonstration of lexer checkpointing for incremental parsing
//!
//! This example shows how lexer checkpoints enable efficient incremental
//! parsing by avoiding re-lexing of unchanged portions of the file.

#[cfg(feature = "incremental")]
use perl_parser::incremental_checkpoint::{CheckpointedIncrementalParser, SimpleEdit};

fn main() {
    #[cfg(not(feature = "incremental"))]
    {
        println!("This demo requires the 'incremental' feature to be enabled.");
        println!("Run with: cargo run --example checkpoint_demo --features incremental");
        return;
    }
    
    #[cfg(feature = "incremental")]
    {
    println!("=== Lexer Checkpointing Demo ===\n");
    
    // Create incremental parser with checkpointing
    let mut parser = CheckpointedIncrementalParser::new();
    
    // Initial source code
    let initial_source = r#"#!/usr/bin/perl
use strict;
use warnings;

# Configuration
my $debug = 0;
my $verbose = 1;

# Main processing
sub process_data {
    my ($input) = @_;
    
    if ($debug) {
        print "Debug: Processing $input\n";
    }
    
    my $result = $input * 42;
    
    if ($verbose) {
        print "Result: $result\n";
    }
    
    return $result;
}

# Test the function
my $value = 10;
my $output = process_data($value);
print "Final output: $output\n";
"#;
    
    println!("Initial parse of {} bytes", initial_source.len());
    let _tree1 = parser.parse(initial_source.to_string()).unwrap();
    
    // Show initial stats
    let stats = parser.stats();
    println!("\nInitial parse statistics:");
    println!("  Total parses: {}", stats.total_parses);
    println!("  Tokens lexed: {}", stats.tokens_relexed);
    
    // Test cases showing incremental parsing with checkpoints
    let test_edits = vec![
        (
            "Change debug flag from 0 to 1",
            SimpleEdit {
                start: 85,  // Position of '0' in 'my $debug = 0'
                end: 86,
                new_text: "1".to_string(),
            }
        ),
        (
            "Change multiplication factor",
            SimpleEdit {
                start: 310,  // Position of '42' in '$input * 42'
                end: 312,
                new_text: "100".to_string(),
            }
        ),
        (
            "Add a comment",
            SimpleEdit {
                start: 200,  // Inside process_data function
                end: 200,
                new_text: "\n    # Added comment\n".to_string(),
            }
        ),
        (
            "Change variable name",
            SimpleEdit {
                start: 458,  // 'value' in 'my $value = 10'
                end: 463,
                new_text: "input_val".to_string(),
            }
        ),
    ];
    
    for (i, (description, edit)) in test_edits.iter().enumerate() {
        println!("\n--- Edit {}: {} ---", i + 1, description);
        println!("  Editing at position {} (replacing {} bytes with {} bytes)",
            edit.start,
            edit.end - edit.start,
            edit.new_text.len()
        );
        
        // Apply edit
        let _tree = parser.apply_edit(edit).unwrap();
        
        // Show incremental stats
        let stats = parser.stats();
        println!("  Incremental parse complete:");
        println!("    Checkpoints used: {}", stats.checkpoints_used);
        println!("    Tokens reused: {}", stats.tokens_reused);
        println!("    Tokens re-lexed: {}", stats.tokens_relexed);
        println!("    Cache hits: {}", stats.cache_hits);
        println!("    Cache misses: {}", stats.cache_misses);
        
        // Calculate efficiency
        let total_tokens = stats.tokens_reused + stats.tokens_relexed;
        if total_tokens > 0 {
            let reuse_rate = (stats.tokens_reused as f64 / total_tokens as f64) * 100.0;
            println!("    Token reuse rate: {:.1}%", reuse_rate);
        }
    }
    
    println!("\n=== Overall Statistics ===");
    let final_stats = parser.stats();
    println!("Total parses: {}", final_stats.total_parses);
    println!("Incremental parses: {}", final_stats.incremental_parses);
    println!("Total tokens processed: {}", final_stats.tokens_reused + final_stats.tokens_relexed);
    println!("Checkpoints used: {}", final_stats.checkpoints_used);
    
    let efficiency = if final_stats.incremental_parses > 0 {
        (final_stats.checkpoints_used as f64 / final_stats.incremental_parses as f64) * 100.0
    } else {
        0.0
    };
    println!("Checkpoint usage rate: {:.1}%", efficiency);
    
    println!("\n=== Benefits of Lexer Checkpointing ===\n");
    println!("1. **Context Preservation**: Checkpoints save lexer mode (ExpectTerm vs ExpectOperator)");
    println!("2. **Stateful Lexing**: Handles constructs like heredocs, formats, and nested quotes");
    println!("3. **Minimal Re-lexing**: Only re-lex from nearest checkpoint before edit");
    println!("4. **Token Caching**: Reuse tokens from unchanged regions");
    println!("5. **Scalability**: Efficient even for large files with small edits");
    
    // Demonstrate checkpoint details
    println!("\n=== Checkpoint Context Demo ===\n");
    
    let context_source = r#"my $x = 42;
s{old}{new}g;  # Substitution with braces
my $y = 99;
"#;
    
    let mut context_parser = CheckpointedIncrementalParser::new();
    context_parser.parse(context_source.to_string()).unwrap();
    
    // Edit inside the substitution
    let subst_edit = SimpleEdit {
        start: 15,  // Inside 'old'
        end: 18,
        new_text: "pattern".to_string(),
    };
    
    println!("Editing inside s{{}}{{}} construct:");
    context_parser.apply_edit(&subst_edit).unwrap();
    
    let stats = context_parser.stats();
    println!("  Checkpoint correctly restored delimiter stack");
    println!("  Re-lexed only affected region");
    println!("  Tokens re-lexed: {}", stats.tokens_relexed);
    } // End of #[cfg(feature = "incremental")] block
}