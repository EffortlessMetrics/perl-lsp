#!/bin/bash

# Benchmark Pure Rust (Pest) parser vs C parser

echo "=== Pure Rust (Pest) vs C Parser Benchmark ==="
echo "Building both implementations..."

# Build the Pure Rust parser with pure-rust feature
cd crates/tree-sitter-perl-rs
cargo build --release --features pure-rust --bin parse-rust
cd ../..

# Build the C parser from tree-sitter-perl-c
cd crates/tree-sitter-perl-c
cargo build --release --bin parse_c
cd ../..

# Create test files if they don't exist
cat > test_simple.pl << 'EOF'
print "Hello, World!\n";
EOF

cat > test_medium.pl << 'EOF'
#!/usr/bin/env perl
use strict;
use warnings;

my $scalar = "test";
my @array = (1, 2, 3, 4, 5);
my %hash = (a => 1, b => 2);

# Reference test
my $ref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Modern features
my $octal = 0o755;
print "..." if $scalar;

# Unicode
my $π = 3.14159;
my $café = "coffee";

sub process {
    my ($x, $y) = @_;
    return $x + $y;
}

for my $i (1..10) {
    print "$i\n" if $i % 2 == 0;
}
EOF

echo ""
echo "Running benchmarks..."
echo "File,Pure_Rust_Time(ms),C_Time(ms),Rust/C_Ratio"

# Function to benchmark a file
benchmark_file() {
    local file=$1
    local size=$(wc -c < "$file" | tr -d ' ')
    
    # Benchmark Pure Rust parser (average of 10 runs)
    rust_time=0
    for i in {1..10}; do
        time=$({ time -p ./crates/tree-sitter-perl-rs/target/release/parse-rust "$file" > /dev/null 2>&1; } 2>&1 | grep real | awk '{print $2}')
        rust_time=$(echo "$rust_time + $time" | bc)
    done
    rust_avg=$(echo "scale=3; $rust_time / 10 * 1000" | bc)
    
    # Benchmark C parser (average of 10 runs)
    c_time=0
    for i in {1..10}; do
        time=$({ time -p ./crates/tree-sitter-perl-c/target/release/parse_c "$file" > /dev/null 2>&1; } 2>&1 | grep real | awk '{print $2}')
        c_time=$(echo "$c_time + $time" | bc)
    done
    c_avg=$(echo "scale=3; $c_time / 10 * 1000" | bc)
    
    # Calculate ratio
    ratio=$(echo "scale=2; $rust_avg / $c_avg" | bc)
    
    echo "$file,$rust_avg,$c_avg,$ratio"
}

benchmark_file "test_simple.pl"
benchmark_file "test_medium.pl"

# Test with real Perl files if they exist
if [ -f "examples/hello.pl" ]; then
    benchmark_file "examples/hello.pl"
fi

echo ""
echo "=== Analysis ==="
echo "Ratio > 1.0 means Pure Rust is slower than C"
echo "Ratio < 1.0 means Pure Rust is faster than C"