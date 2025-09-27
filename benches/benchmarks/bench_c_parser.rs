use std::time::Instant;
use tree_sitter_perl_c;

fn main() {
    let test_code = r#"#!/usr/bin/env perl
use strict;
use warnings;

# Variables with references
my $scalar = "Hello, World!";
my @array = (1..100);
my %hash = map { $_ => $_ * 2 } 1..50;
my $sref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Modern features
my $perms = 0o755;
sub todo { ... }
my $π = 3.14159;
my $café = "coffee shop";

# Complex code
for my $i (@array) {
    if ($i % 2 == 0) {
        print "$i is even\n";
    } elsif ($i % 3 == 0) {
        print "$i is divisible by 3\n"; 
    } else {
        print "$i is odd\n";
    }
}

# Regex operations
my $text = "The quick brown fox jumps over the lazy dog";
$text =~ s/quick/fast/g;
$text =~ s/lazy/sleepy/g;

# Subroutines
sub calculate {
    my ($x, $y, $z) = @_;
    return ($x + $y) * $z;
}

my $result = calculate(10, 20, 3);
"#;

    println!("Benchmarking C Parser");
    println!("Code size: {} bytes", test_code.len());
    println!();
    
    // Warmup
    for _ in 0..10 {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_perl_c::language()).unwrap();
        let _ = parser.parse(test_code, None);
    }
    
    // Benchmark
    let iterations = 1000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_perl_c::language()).unwrap();
        let result = parser.parse(test_code, None);
        if result.is_none() {
            eprintln!("Parse error!");
            return;
        }
    }
    
    let duration = start.elapsed();
    let avg_time = duration / iterations;
    
    println!("Iterations: {}", iterations);
    println!("Total time: {:?}", duration);
    println!("Average time per parse: {:?}", avg_time);
    println!("Throughput: {:.2} MB/s", (test_code.len() as f64 * iterations as f64) / duration.as_secs_f64() / 1_000_000.0);
}