use perl_parser::{Parser, FoldingRangeExtractor};

fn main() {
    let content = r#"#!/usr/bin/perl
use strict;
use warnings;

sub function1 {
    my $x = 1;
    if ($x > 0) {
        print "Positive\n";
    }
}

sub function2 {
    my @array = (1, 2, 3);
    foreach my $item (@array) {
        print "$item\n";
    }
}

package MyPackage;

sub method1 {
    return 42;
}

1;"#;

    let mut parser = Parser::new(content);
    
    match parser.parse() {
        Ok(ast) => {
            println!("AST parsed successfully");
            let mut extractor = FoldingRangeExtractor::new();
            let ranges = extractor.extract(&ast);
            println!("Found {} folding ranges", ranges.len());
            for range in &ranges {
                println!("  Range: offset {} to {}, kind: {:?}", 
                    range.start_offset, range.end_offset, range.kind);
                // Show the actual lines
                let start_line = content[..range.start_offset].chars().filter(|&c| c == '\n').count();
                let end_line = content[..range.end_offset].chars().filter(|&c| c == '\n').count();
                println!("    Lines: {} to {}", start_line, end_line);
            }
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}