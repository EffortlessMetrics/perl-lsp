#!/bin/bash

echo "=== True Pure Rust (Pest) vs C Parser Benchmark ==="
echo ""

# Create test file with diverse Perl features
cat > benchmark_test.pl << 'EOF'
#!/usr/bin/env perl
use strict;
use warnings;

# Test reference operator (\)
my $scalar = "Hello";
my @array = (1..10);
my %hash = (a => 1, b => 2);
my $sref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Test modern octal (0o755) and traditional
my $modern_perms = 0o755;
my $old_perms = 0755;

# Test ellipsis operator (...)
sub not_implemented {
    ...  # Placeholder
}

# Unicode identifiers
my $π = 3.14159;
my $café = "coffee";
sub 日本語 { return "works" }

# Complex structures
for my $i (1..100) {
    if ($i % 15 == 0) {
        print "FizzBuzz\n";
    } elsif ($i % 3 == 0) {
        print "Fizz\n";
    } elsif ($i % 5 == 0) {
        print "Buzz\n";
    } else {
        print "$i\n";
    }
}

# Regex with substitutions
my $text = "The quick brown fox";
$text =~ s/quick/fast/g;
$text =~ s/brown/red/g;

# Method calls
package Foo;
sub new { bless {}, shift }
sub bar { "method called" }
my $obj = Foo->new();
$obj->bar();

# Heredoc
my $heredoc = <<'END_TEXT';
This is a heredoc
with multiple lines
END_TEXT

# Modern Perl features
given ($scalar) {
    when ("Hello") { print "Greeting\n" }
    default { print "Other\n" }
}

1;
EOF

echo "Test file size: $(wc -c < benchmark_test.pl) bytes"
echo ""

# Function to time a command multiple times
benchmark() {
    local name=$1
    local cmd=$2
    local times=()
    
    echo "$name:"
    
    # Warmup
    for i in {1..3}; do
        $cmd >/dev/null 2>&1
    done
    
    # Actual timing
    for i in {1..10}; do
        start=$(date +%s%N)
        $cmd >/dev/null 2>&1
        end=$(date +%s%N)
        elapsed=$((end - start))
        times+=($elapsed)
        echo "  Run $i: $(echo "scale=3; $elapsed/1000000" | bc) ms"
    done
    
    # Calculate average
    sum=0
    for t in "${times[@]}"; do
        sum=$((sum + t))
    done
    avg=$((sum / 10))
    echo "  Average: $(echo "scale=3; $avg/1000000" | bc) ms"
    echo ""
}

# Check if binaries exist
RUST_BIN="./crates/tree-sitter-perl-rs/target/release/parse-rust"
C_SCANNER_BIN="./target/release/tree-sitter-perl"  # This would be the C scanner version

if [ ! -f "$RUST_BIN" ]; then
    echo "Pure Rust binary not found at $RUST_BIN"
    echo "Building..."
    cd crates/tree-sitter-perl-rs && cargo build --release --features pure-rust --bin parse-rust && cd ../..
fi

echo "=== Benchmarking Pure Rust (Pest) Parser ==="
benchmark "Pure Rust Parser" "$RUST_BIN benchmark_test.pl"

# For C comparison, we need to use the parse binary that uses C scanner
# Since the tree-sitter-perl directory has workspace issues, let's check what we have
echo "=== C Parser Comparison ==="
echo "Note: The C parser from tree-sitter-perl has workspace configuration issues."
echo "The previous benchmark results showing 1.5% difference were likely comparing"
echo "the C scanner within tree-sitter-perl-rs (not pure Rust) with another C implementation."
echo ""
echo "The Pure Rust (Pest) parser above is the true 100% Rust implementation"
echo "with zero C dependencies."