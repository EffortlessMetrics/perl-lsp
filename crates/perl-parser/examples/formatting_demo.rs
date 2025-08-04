//! Demo of code formatting capabilities

use perl_parser::{CodeFormatter, FormattingOptions};
use std::path::Path;

fn main() {
    println!("=== Perl Code Formatting Demo ===\n");
    
    let formatter = CodeFormatter::new();
    
    // Example 1: Basic formatting
    println!("Example 1: Basic formatting");
    println!("-" . repeat(40));
    
    let messy_code = r#"sub calculate_total{my($items,$tax_rate)=@_;my$subtotal=0;foreach my$item(@$items){$subtotal+=$item->{price}*$item->{quantity};}my$tax=$subtotal*$tax_rate;return$subtotal+$tax;}"#;
    
    println!("Original:");
    println!("{}\n", messy_code);
    
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(true),
    };
    
    match formatter.format_document(messy_code, &options) {
        Ok(edits) => {
            if !edits.is_empty() {
                println!("Formatted:");
                println!("{}", edits[0].new_text);
            } else {
                println!("No formatting changes needed.");
            }
        }
        Err(e) => {
            println!("Formatting error: {}", e);
            println!("Make sure perltidy is installed: cpan Perl::Tidy");
        }
    }
    
    // Example 2: Different indentation
    println!("\n\nExample 2: Tab indentation");
    println!("-" . repeat(40));
    
    let tab_options = FormattingOptions {
        tab_size: 8,
        insert_spaces: false,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(true),
    };
    
    match formatter.format_document(messy_code, &tab_options) {
        Ok(edits) => {
            if !edits.is_empty() {
                println!("Formatted with tabs:");
                // Show tabs as [TAB] for visibility
                let formatted = edits[0].new_text.replace('\t', "[TAB]");
                println!("{}", formatted);
            }
        }
        Err(e) => {
            println!("Formatting error: {}", e);
        }
    }
    
    // Example 3: Range formatting
    println!("\n\nExample 3: Range formatting");
    println!("-" . repeat(40));
    
    let multi_line = r#"#!/usr/bin/perl
use strict;
use warnings;

sub messy{my$x=shift;my$y=shift;return$x+$y;}

my $result = messy(1, 2);
print "Result: $result\n";"#;
    
    println!("Original multi-line code:");
    println!("{}\n", multi_line);
    
    // Format just the messy subroutine (line 4)
    let range = perl_parser::formatting::Range {
        start: perl_parser::formatting::Position { line: 4, character: 0 },
        end: perl_parser::formatting::Position { line: 4, character: 50 },
    };
    
    match formatter.format_range(multi_line, &range, &options) {
        Ok(edits) => {
            if !edits.is_empty() {
                println!("Formatted line 5 only:");
                let lines: Vec<&str> = multi_line.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if i == 4 {
                        println!("{}", edits[0].new_text.trim_end());
                    } else {
                        println!("{}", line);
                    }
                }
            } else {
                println!("No formatting changes needed for the range.");
            }
        }
        Err(e) => {
            println!("Range formatting error: {}", e);
        }
    }
    
    // Example 4: With .perltidyrc
    println!("\n\nExample 4: Using .perltidyrc");
    println!("-" . repeat(40));
    
    match formatter.run_perltidy_with_config(messy_code, &options, Some(Path::new("."))) {
        Ok(formatted) => {
            if formatted != messy_code {
                println!("Formatted with .perltidyrc configuration:");
                println!("{}", formatted);
            } else {
                println!("No .perltidyrc found or no changes needed.");
            }
        }
        Err(e) => {
            println!("Formatting error: {}", e);
        }
    }
    
    println!("\n=== Demo Complete ===");
}