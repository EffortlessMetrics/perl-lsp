#!/bin/bash

echo "=== Pure Rust (Pest) vs C Parser Benchmark ==="
echo ""
echo "Building implementations..."

# Build Pure Rust parser
echo "Building Pure Rust parser..."
cd crates/tree-sitter-perl-rs
cargo build --release --features pure-rust --bin parse-rust 2>/dev/null
cd ../..

# Build C parser
echo "Building C parser..."
cd crates/tree-sitter-perl-c
cargo build --release --bin parse_c 2>/dev/null
cd ../..

# Create test file
cat > test_benchmark.pl << 'EOF'
#!/usr/bin/env perl
use strict;
use warnings;

# Variables
my $scalar = "Hello, World!";
my @array = (1..10);
my %hash = map { $_ => $_ * 2 } 1..5;

# References (testing the \ operator)
my $sref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Modern octal
my $perms = 0o755;
my $old_perms = 0755;

# Ellipsis
sub todo {
    ...
}

# Unicode identifiers
my $π = 3.14159;
my $café = "coffee shop";
sub 日本語 { return "Japanese" }

# Control flow
for my $i (@array) {
    print "$i\n" if $i % 2 == 0;
}

# Regex
my $text = "foo bar baz";
$text =~ s/foo/FOO/g;

1;
EOF

echo ""
echo "Running 5 iterations each..."
echo ""

# Run Pure Rust parser
echo "Pure Rust (Pest) Parser:"
for i in {1..5}; do
    echo -n "  Run $i: "
    (time -p ./crates/tree-sitter-perl-rs/target/release/parse-rust test_benchmark.pl >/dev/null 2>&1) 2>&1 | grep real | awk '{print $2 "s"}'
done

echo ""
echo "C Parser:"
for i in {1..5}; do
    echo -n "  Run $i: "
    (time -p ./crates/tree-sitter-perl-c/target/release/parse_c test_benchmark.pl >/dev/null 2>&1) 2>&1 | grep real | awk '{print $2 "s"}'
done

echo ""
echo "Note: Times include process startup overhead"