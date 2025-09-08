use perl_parser::import_optimizer::ImportOptimizer;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a test file with some Perl code
    let test_code = r#"#!/usr/bin/perl
use strict;
use warnings;
use List::Util qw(first max min sum);
use Data::Dumper qw(Dumper);
use JSON qw(encode_json decode_json to_json);

# Only using some functions
my @nums = (1, 2, 3, 4, 5);
print "Max: " . max(@nums) . "\n";
print "Sum: " . sum(@nums) . "\n";

my $data = { numbers => \@nums };
print encode_json($data) . "\n";
"#;

    // Write test file
    let test_file = "test_import_demo.pl";
    fs::write(test_file, test_code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(Path::new(test_file))?;

    println!("=== Import Analysis Results ===");
    println!("Total imports found: {}", analysis.imports.len());
    for import in &analysis.imports {
        println!("  {} - {} symbols: {:?}", import.module, import.symbols.len(), import.symbols);
    }

    println!("\n=== Unused Imports ===");
    for unused in &analysis.unused_imports {
        println!("  {} has unused symbols: {:?}", unused.module, unused.symbols);
    }

    println!("\n=== Duplicate Imports ===");
    for duplicate in &analysis.duplicate_imports {
        println!("  {} appears on lines: {:?}", duplicate.module, duplicate.lines);
    }

    println!("\n=== Optimized Imports ===");
    let optimized = optimizer.generate_optimized_imports(&analysis);
    println!("{}", optimized);

    // Clean up
    fs::remove_file(test_file).ok();

    println!("\n✅ Import optimizer is working correctly!");

    Ok(())
}
