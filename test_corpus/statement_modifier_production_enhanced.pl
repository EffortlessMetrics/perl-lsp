#!/usr/bin/env perl
# Test: Enhanced Statement Modifier Production Scenarios
# Impact: Comprehensive testing of postfix conditional and loop modifiers
# NodeKinds: StatementModifier, PostfixIf, PostfixUnless, PostfixWhile, PostfixUntil, PostfixFor
# 
# This file tests the parser's ability to handle:
# 1. Complex postfix conditional patterns
# 2. Performance-optimized modifier usage
# 3. Nested and chained statement modifiers
# 4. Real-world production patterns
# 5. Error handling with modifiers
# 6. Data processing pipelines with modifiers
# 7. Advanced control flow patterns
# 8. Edge cases and ambiguous syntax

use strict;
use warnings;

print "=== Enhanced Statement Modifier Production Tests ===\n\n";

# Test 1: Basic postfix conditionals
print "=== Basic Postfix Conditionals ===\n";

# Simple if modifier
my $debug = 1;
print "Debug mode enabled\n" if $debug;
print "This won't print\n" if !$debug;

# Complex if modifier with expressions
my $user_level = 5;
my $feature_enabled = 1;
print "Advanced features available\n" if $user_level > 3 && $feature_enabled;

# Unless modifier
my $error_occurred = 0;
print "No errors detected\n" unless $error_occurred;
print "Error handling needed\n" unless !$error_occurred;

# Unless with complex conditions
my $valid_input = 1;
my $permissions_ok = 1;
print "Input validation skipped\n" unless $valid_input && $permissions_ok;

# Ternary vs modifier comparison
my $value = 42;
my $result = $value > 10 ? "large" : "small";
print "Ternary result: $result\n";

print "Value is large\n" if $value > 10;
print "Value is small\n" unless $value > 10;

print "\n";

# Test 2: Postfix loop modifiers
print "=== Postfix Loop Modifiers ===\n";

# While modifier
my $count = 0;
print "Count: $count\n" while $count++ < 5;

# While with function call
sub should_continue {
    my ($c) = @_;
    return $c < 3;
}

my $while_count = 0;
print "While loop: $while_count\n" while should_continue($while_count++);

# Until modifier
my $until_count = 0;
print "Until loop: $until_count\n" until $until_count++ >= 3;

# For/foreach modifier with arrays
my @fruits = qw(apple banana cherry date);
print "Fruit: $_\n" for @fruits;

# For with range
print "Number: $_\n" for 1..5;

# For with hash
my %colors = (red => '#FF0000', green => '#00FF00', blue => '#0000FF');
print "Color: $_ -> $colors{$_}\n" for sort keys %colors;

# Complex for with expression
print "Doubled: " . ($_ * 2) . "\n" for grep { $_ % 2 == 0 } (1..10);

print "\n";

# Test 3: Performance-optimized modifier patterns
print "=== Performance-Optimized Patterns ===\n";

# Fast early exit
sub fast_check {
    my ($data) = @_;
    return "invalid" unless defined $data;
    return "empty" unless length $data;
    return "too_short" unless length $data >= 3;
    return "valid";
}

print "Check 1: " . fast_check(undef) . "\n";
print "Check 2: " . fast_check("") . "\n";
print "Check 3: " . fast_check("ab") . "\n";
print "Check 4: " . fast_check("valid") . "\n";

# Efficient filtering
my @large_dataset = (1..1000);
my $filtered_count = 0;
$filtered_count++ if $_ % 2 == 0 && $_ % 3 == 0 for @large_dataset;
print "Numbers divisible by 6 in 1..1000: $filtered_count\n";

# Memory-efficient processing
sub process_stream {
    my ($stream) = @_;
    my $processed = 0;
    
    # Process items without storing entire result
    $processed++ if process_item($_) for @$stream;
    
    return $processed;
}

sub process_item {
    my ($item) = @_;
    return $item % 2 == 0;  # Process only even numbers
}

my @stream_data = (1..100);
my $stream_result = process_stream(\@stream_data);
print "Stream processed $stream_result items\n\n";

# Test 4: Real-world production patterns
print "=== Real-World Production Patterns ===\n";

# Pattern 1: Debug logging
my $log_level = 2;  # 1=ERROR, 2=WARN, 3=INFO, 4=DEBUG

sub log_message {
    my ($level, $message) = @_;
    print "[$level] $message\n" if $level <= $log_level;
}

log_message(1, "Critical error occurred");
log_message(2, "Warning: Low disk space");
log_message(3, "Info: User logged in");
log_message(4, "Debug: Variable value check");

# Pattern 2: Input validation
sub validate_user_input {
    my ($input) = @_;
    
    return "empty" unless defined $input;
    return "whitespace only" unless $input =~ /\S/;
    return "too short" unless length $input >= 3;
    return "invalid characters" unless $input =~ /^[a-zA-Z0-9_]+$/;
    return "valid";
}

my @test_inputs = (undef, "", "ab", "valid_input", "invalid-input!");
for my $test_input (@test_inputs) {
    my $validation = validate_user_input($test_input);
    print "Input '" . (defined $test_input ? $test_input : 'undef') . "': $validation\n";
}

# Pattern 3: Configuration management
my %config = (
    debug => 1,
    verbose => 0,
    dry_run => 0,
    force => 0
);

sub execute_command {
    my ($cmd) = @_;
    
    print "DRY RUN: Would execute: $cmd\n" if $config{dry_run};
    print "VERBOSE: Executing: $cmd\n" if $config{verbose};
    print "DEBUG: Command details: " . join(', ', %$cmd) . "\n" if $config{debug} && ref $cmd eq 'HASH';
    
    # Simulate execution
    return "success" unless $config{dry_run};
    return "dry_run";
}

my $command = { action => 'create', target => 'user', name => 'testuser' };
my $exec_result = execute_command($command);
print "Command result: $exec_result\n";

# Pattern 4: Error handling with modifiers
sub safe_operation {
    my ($operation, $should_fail) = @_;
    
    print "Attempting operation: $operation\n";
    
    # Simulate failure
    die "Operation failed: $operation" if $should_fail;
    
    return "Operation succeeded: $operation";
}

sub handle_safely {
    my ($operation, $should_fail) = @_;
    
    my $result = eval { safe_operation($operation, $should_fail) };
    print "Error: $@" if $@;
    print "Success: $result\n" if $result;
    
    return $result || "handled";
}

handle_safely("test_operation", 0);
handle_safely("failing_operation", 1);

print "\n";

# Test 5: Advanced control flow patterns
print "=== Advanced Control Flow Patterns ===\n";

# Pattern 1: State machine with modifiers
my $state = 'initial';
my $transitions = 0;

sub transition_state {
    my ($event) = @_;
    
    $state = 'processing' if $state eq 'initial' && $event eq 'start';
    $state = 'complete' if $state eq 'processing' && $event eq 'finish';
    $state = 'error' if $event eq 'error';
    $transitions++;
    
    print "State transition: $event -> $state\n";
}

for my $event (qw(start process finish)) {
    transition_state($event);
}

print "Total transitions: $transitions\n";

# Pattern 2: Pipeline processing with modifiers
sub process_pipeline {
    my (@stages) = @_;
    my $data = { value => 1, processed => 0 };
    
    $data->{value} *= 2 if $stages[0] eq 'double';
    $data->{value} += 10 if $stages[1] eq 'add_ten';
    $data->{value} = sqrt($data->{value}) if $stages[2] eq 'sqrt';
    $data->{processed} = 1 if grep { $_ } @stages;
    
    return $data;
}

my @pipeline_stages = ('double', 'add_ten', 'sqrt');
my $pipeline_result = process_pipeline(@pipeline_stages);
print "Pipeline result: value=$pipeline_result->{value}, processed=$pipeline_result->{processed}\n";

# Pattern 3: Conditional chaining
sub conditional_chain {
    my ($value) = @_;
    
    my $result = $value;
    $result *= 2 if $result > 0;
    $result += 5 if $result < 20;
    $result = int($result) if $result != int($result);
    $result = "final:$result" if $result > 10;
    
    return $result;
}

for my $test_val (2, 15, 8) {
    my $chained = conditional_chain($test_val);
    print "Chain $test_val -> $chained\n";
}

print "\n";

# Test 6: Data processing with modifiers
print "=== Data Processing with Modifiers ===\n";

# Pattern 1: Data validation and transformation
my @raw_data = (
    { name => 'Alice', age => 25, active => 1 },
    { name => 'Bob', age => 17, active => 0 },
    { name => 'Charlie', age => 30, active => 1 },
    { name => '', age => 22, active => 1 },
    { name => 'Eve', age => 16, active => 0 }
);

my @processed_data;
for my $record (@raw_data) {
    # Skip invalid records
    next unless $record->{name} && length $record->{name} > 0;
    next unless $record->{age} && $record->{age} >= 18;
    next unless $record->{active};
    
    # Transform valid records
    $record->{status} = 'adult' if $record->{age} >= 18;
    $record->{category} = 'senior' if $record->{age} >= 30;
    $record->{name} = uc $record->{name} if $record->{name};
    
    push @processed_data, $record;
}

print "Processed " . scalar(@processed_data) . " valid records\n";
for my $record (@processed_data) {
    print "  $record->{name} (age $record->{age}, status $record->{status})\n";
}

# Pattern 2: Statistical analysis with modifiers
my @numbers = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
my ($sum, $even_count, $odd_count, $prime_count) = (0, 0, 0, 0);

$sum += $_ for @numbers;
$even_count++ if $_ % 2 == 0 for @numbers;
$odd_count++ if $_ % 2 == 1 for @numbers;

# Simple prime check
sub is_prime {
    my ($n) = @_;
    return 0 if $n < 2;
    return 0 if $n == 2;
    return 0 if $n % 2 == 0;
    
    for my $i (3..int(sqrt($n))) {
        return 0 if $n % $i == 0;
    }
    return 1;
}

$prime_count++ if is_prime($_) for @numbers;

print "Statistics for numbers 1-10:\n";
print "  Sum: $sum\n";
print "  Even count: $even_count\n";
print "  Odd count: $odd_count\n";
print "  Prime count: $prime_count\n";

# Pattern 3: Text processing with modifiers
my @text_lines = (
    "This is a normal line",
    "",
    "   ",  # Whitespace only
    "WARNING: This is a warning",
    "ERROR: This is an error",
    "INFO: This is info",
    "   Another line with leading spaces"
);

my ($normal_count, $warning_count, $error_count, $empty_count) = (0, 0, 0, 0);

for my $line (@text_lines) {
    $empty_count++ unless defined $line && length $line > 0;
    next unless defined $line && length $line > 0;
    
    $line =~ s/^\s+|\s+$//g;  # Trim whitespace
    
    $warning_count++ if $line =~ /^WARNING:/;
    $error_count++ if $line =~ /^ERROR:/;
    $normal_count++ if $line !~ /^(WARNING|ERROR):/;
}

print "Text analysis:\n";
print "  Normal lines: $normal_count\n";
print "  Warning lines: $warning_count\n";
print "  Error lines: $error_count\n";
print "  Empty lines: $empty_count\n";

print "\n";

# Test 7: Edge cases and ambiguous syntax
print "=== Edge Cases and Ambiguous Syntax ===\n";

# Edge case 1: Multiple modifiers on same line (not recommended but possible)
my $test_var = 5;
print "Complex condition\n" if $test_var > 0 && $test_var < 10;

# Edge case 2: Modifier with complex expressions
my $complex_result = 42;
print "Complex expression true\n" if ($complex_result * 2 + 1) > 80 && $complex_result % 7 == 0;

# Edge case 3: Modifier with subroutine calls
sub get_value { return 10; }
sub check_condition { return $_[0] > 5; }

print "Subroutine condition true\n" if check_condition(get_value());

# Edge case 4: Modifier with array/hash operations
my @test_array = (1, 2, 3);
push @test_array, 4 if scalar @test_array < 5;
print "Array after push: @test_array\n";

my %test_hash = (a => 1, b => 2);
$test_hash{c} = 3 if exists $test_hash{a};
print "Hash after conditional add: " . join(', ', %test_hash) . "\n";

# Edge case 5: Modifier with regex operations
my $test_string = "Hello World";
$test_string =~ s/World/Perl/ if $test_string =~ /World/;
print "Modified string: $test_string\n";

# Edge case 6: Modifier in list context
my @conditional_values = (1, 2, 3);
push @conditional_values, 4 if grep { $_ > 2 } @conditional_values;
print "Conditional array push: @conditional_values\n";

# Edge case 7: Modifier with undefined values
my $undefined_var;
print "Undefined is false\n" unless $undefined_var;
print "Defined is true\n" if defined $test_var;

print "\n";

# Test 8: Performance comparison
print "=== Performance Comparison ===\n";

# Compare modifier vs block syntax
sub benchmark_modifier_vs_block {
    my ($iterations) = @_;
    $iterations ||= 100000;
    
    my @test_data = (1..$iterations);
    my $modifier_count = 0;
    my $block_count = 0;
    
    # Benchmark modifier syntax
    my $start = time();
    $modifier_count++ if $_ % 2 == 0 for @test_data;
    my $modifier_time = time() - $start;
    
    # Benchmark block syntax
    $start = time();
    for my $item (@test_data) {
        if ($item % 2 == 0) {
            $block_count++;
        }
    }
    my $block_time = time() - $start;
    
    print "Performance comparison ($iterations iterations):\n";
    print "  Modifier syntax: $modifier_time seconds (count: $modifier_count)\n";
    print "  Block syntax: $block_time seconds (count: $block_count)\n";
    print "  Performance ratio: " . sprintf('%.2f', $modifier_time / $block_time) . "x\n";
}

benchmark_modifier_vs_block(500000);

print "\n=== Enhanced Statement Modifier Production Tests Completed ===\n";
print "This file demonstrates comprehensive statement modifier patterns\n";
print "for production Perl applications with performance considerations.\n";