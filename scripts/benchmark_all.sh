#!/bin/bash
# Comprehensive benchmark script for comparing C vs Rust parsers

set -e

echo "=== Tree-sitter Perl Parser Benchmark Suite ==="
echo "Comparing C/tree-sitter parser with pure Rust parser"
echo

# Create test files if they don't exist
TEST_DIR="benchmark_tests"
mkdir -p "$TEST_DIR"

# Simple test file
cat > "$TEST_DIR/simple.pl" << 'EOF'
my $x = 42;
print "Hello, World!";
EOF

# Medium complexity test file
cat > "$TEST_DIR/medium.pl" << 'EOF'
#!/usr/bin/perl
use strict;
use warnings;

sub factorial {
    my $n = shift;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

my @numbers = (1, 2, 3, 4, 5);
foreach my $num (@numbers) {
    print "Factorial of $num is " . factorial($num) . "\n";
}

my %hash = (
    name => "John",
    age => 30,
    city => "New York"
);

while (my ($key, $value) = each %hash) {
    print "$key: $value\n";
}
EOF

# Complex test file
cat > "$TEST_DIR/complex.pl" << 'EOF'
#!/usr/bin/perl
package MyClass;

use strict;
use warnings;
use base qw(Exporter);

our @EXPORT_OK = qw(process_data);

sub new {
    my ($class, %args) = @_;
    my $self = {
        data => $args{data} || [],
        debug => $args{debug} || 0,
    };
    bless $self, $class;
    return $self;
}

sub process_data {
    my ($self, $callback) = @_;
    my @results;
    
    foreach my $item (@{$self->{data}}) {
        eval {
            push @results, $callback->($item);
        };
        if ($@) {
            warn "Error processing item: $@" if $self->{debug};
        }
    }
    
    return \@results;
}

package main;

my $obj = MyClass->new(
    data => [1..10],
    debug => 1
);

my $results = $obj->process_data(sub {
    my $x = shift;
    return $x ** 2;
});

print join(", ", @$results), "\n";

__END__
=head1 NAME
MyClass - Example class
=cut
EOF

echo "Test files created in $TEST_DIR/"
echo

# Function to run benchmarks
run_benchmark() {
    local name=$1
    local file=$2
    local iterations=$3
    
    echo "=== Benchmark: $name ==="
    echo "File: $file"
    echo "Iterations: $iterations"
    echo
    
    # Test with pure Rust parser
    echo "1. Pure Rust parser only:"
    cargo run --release --features "pure-rust test-utils" --bin compare_parsers -- "$file" "$iterations"
    echo
    
    # Test with C/tree-sitter parser
    echo "2. C/tree-sitter parser only:"
    cargo run --release --features "c-scanner test-utils" --bin compare_parsers -- "$file" "$iterations"
    echo
    
    # Test with both (if possible)
    echo "3. Direct comparison (if both available):"
    cargo run --release --features "test-utils" --bin compare_parsers -- "$file" "$iterations"
    echo
    echo "---"
    echo
}

# Run benchmarks
echo "Starting benchmarks..."
echo

# Quick tests (100 iterations)
run_benchmark "Simple Perl" "$TEST_DIR/simple.pl" 100
run_benchmark "Medium Complexity" "$TEST_DIR/medium.pl" 100
run_benchmark "Complex Perl" "$TEST_DIR/complex.pl" 100

# Performance tests (1000 iterations)
echo "=== Performance Tests (1000 iterations) ==="
run_benchmark "Simple Perl (Performance)" "$TEST_DIR/simple.pl" 1000
run_benchmark "Medium Complexity (Performance)" "$TEST_DIR/medium.pl" 1000

# Test with real-world files if available
if [ -f "examples/real_world.pl" ]; then
    echo "=== Real World Example ==="
    run_benchmark "Real World Perl" "examples/real_world.pl" 10
fi

echo
echo "=== Benchmark Summary ==="
echo "All benchmarks completed."
echo
echo "To run individual comparisons:"
echo "  cargo run --features 'pure-rust test-utils' --bin compare_parsers -- <file> [iterations]"
echo "  cargo run --features 'c-scanner test-utils' --bin compare_parsers -- <file> [iterations]"
echo
echo "To see test comparison:"
echo "  cargo run --features 'pure-rust test-utils' --bin compare_parsers -- --test"