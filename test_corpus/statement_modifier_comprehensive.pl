#!/usr/bin/env perl
# Test: StatementModifier NodeKind
# Impact: Ensures parser handles postfix conditional and loop modifiers
# NodeKinds: StatementModifier
# 
# This file tests the parser's ability to handle:
# 1. Postfix if modifiers
# 2. Postfix unless modifiers
# 3. Postfix while modifiers
# 4. Postfix until modifiers
# 5. Postfix for modifiers (foreach)
# 6. Postfix when modifiers
# 7. Complex statement modifier scenarios
# 8. Error cases and edge conditions

use strict;
use warnings;

# Basic postfix if modifier
print "This will print\n" if 1;
print "This won't print\n" if 0;

# Postfix if with variables
my $condition = 1;
my $message = "Conditional message";
print "$message\n" if $condition;

# Postfix if with complex expressions
print "Complex condition true\n" if ($condition && $message ne "");

# Postfix unless modifier
print "This won't print\n" unless 1;
print "This will print\n" unless 0;

# Postfix unless with negated logic
print "Not zero\n" unless $condition == 0;

# Postfix while modifier
my $count = 0;
print "$count\n" while $count++ < 5;

# Postfix while with condition function
sub should_continue {
    my ($c) = @_;
    return $c < 3;
}

my $while_count = 0;
print "While loop: $while_count\n" while should_continue($while_count++);

# Postfix until modifier
my $until_count = 0;
print "Until loop: $until_count\n" until $until_count++ >= 3;

# Postfix until with complex condition
my $complex_until = 0;
print "Complex until: $complex_until\n" until ($complex_until++ > 2 || $complex_until == 5);

# Postfix for modifier (foreach)
my @array = (1, 2, 3);
print "Array element: $_\n" for @array;

# Postfix for with range
print "Range: $_\n" for 1..5;

# Postfix for with hash
my %hash = (a => 1, b => 2, c => 3);
print "Hash: key=$_, value=$hash{$_}\n" for keys %hash;

# Postfix when modifier (given/when context)
# Note: when as a modifier is less common but valid in some contexts
given (5) {
    print "When modifier: matched 5\n" when $_ == 5;
    print "When modifier: matched 10\n" when $_ == 10;
}

# Complex statement modifier scenarios

# Scenario 1: Nested statement modifiers
my $nested_condition = 1;
my $nested_value = 42;
print "Nested result: $nested_value\n" if $nested_condition while $nested_value-- > 40;

# Scenario 2: Statement modifier with function calls
sub get_data {
    return "data from function";
}

sub check_condition {
    return 1;
}

print get_data() if check_condition();

# Scenario 3: Statement modifier with array operations
my @numbers = (1, 2, 3, 4, 5);
push @numbers, 6 if scalar @numbers < 10;
print "Numbers after push: @numbers\n";

# Scenario 4: Statement modifier with hash operations
my %config = (debug => 0, verbose => 1);
$config{debug} = 1 if $config{verbose};
print "Config: " . join(', ', %config) . "\n";

# Scenario 5: Statement modifier with regex
my $text = "Hello World";
$text =~ s/World/Perl/ if $text =~ /World/;
print "Modified text: $text\n";

# Statement modifiers with different data types

# Type 1: Statement modifier with strings
my $string_var = "test";
print "String length: " . length($string_var) . "\n" if length($string_var) > 2;

# Type 2: Statement modifier with numbers
my $number_var = 42;
print "Number is even\n" if $number_var % 2 == 0;

# Type 3: Statement modifier with references
my $array_ref = [1, 2, 3];
print "Array reference size: " . scalar @$array_ref . "\n" if ref($array_ref) eq 'ARRAY';

# Type 4: Statement modifier with undef
my $undef_var = undef;
print "Variable is undefined\n" unless defined $undef_var;

# Statement modifiers in different contexts

# Context 1: Statement modifier in subroutine
sub conditional_print {
    my ($message, $condition) = @_;
    print "$message\n" if $condition;
}

conditional_print("This prints", 1);
conditional_print("This doesn't print", 0);

# Context 2: Statement modifier in loop
for my $i (1..10) {
    print "Even number: $i\n" if $i % 2 == 0;
    next if $i == 5;  # Skip 5
    print "Processed: $i\n" unless $i == 5;
}

# Context 3: Statement modifier in eval
eval {
    my $eval_var = "eval test";
    print "Eval success: $eval_var\n" if $eval_var ne "";
    1;
} or do {
    warn "Eval failed: $@";
};

# Context 4: Statement modifier with file operations
# open my $fh, '<', 'test_file.txt' or die $!;
# print <$fh> if $fh;  # Print file content if filehandle is valid
# close $fh;

# Advanced statement modifier patterns

# Pattern 1: Statement modifier with logical operators
my $logical_var = 1;
print "Logical AND\n" if $logical_var && $logical_var > 0;
print "Logical OR\n" if $logical_var || $logical_var == 0;

# Pattern 2: Statement modifier with comparison chains
my $comparison_var = 10;
print "In range\n" if 5 < $comparison_var && $comparison_var < 15;

# Pattern 3: Statement modifier with ternary result
my $ternary_result = $logical_var ? "true" : "false";
print "Ternary result: $ternary_result\n" if $ternary_result eq "true";

# Pattern 4: Statement modifier with defined-or
my $defined_or_var = undef;
my $default_value = $defined_or_var // "default";
print "Defined-or result: $default_value\n" if $default_value ne "";

# Pattern 5: Statement modifier with regex capture
my $regex_text = "Version 1.2.3";
print "Version captured: $1\n" if $regex_text =~ /Version (\d+\.\d+\.\d+)/;

# Statement modifiers with special variables

# Special 1: Statement modifier with $_
$_ = "special variable";
print "Default variable: $_\n" if $_ ne "";

# Special 2: Statement modifier with $.
# (Assuming we're reading from a file)
# print "Line number: $.\n" if $. > 10;

# Special 3: Statement modifier with @ARGV
# print "Processing: $ARGV[0]\n" if @ARGV && $ARGV[0] =~ /\.pl$/;

# Special 4: Statement modifier with %ENV
print "Home directory: $ENV{HOME}\n" if exists $ENV{HOME};

# Statement modifiers with error handling

# Error 1: Statement modifier with die
# die "Fatal error occurred\n" if $critical_condition;

# Error 2: Statement modifier with warn
warn "Warning condition met\n" if $warning_condition;

# Error 3: Statement modifier with croak (from Carp)
# use Carp 'croak';
# croak "Critical condition\n" if $critical_error;

# Performance considerations

# Performance 1: Statement modifier vs block if
# Fast way
print "Fast check\n" if $fast_condition;

# Slower way (for comparison)
# if ($fast_condition) {
#     print "Slow check\n";
# }

# Performance 2: Statement modifier in tight loop
my @large_array = (1..10000);
my $processed = 0;
$processed++ if $_ % 2 == 0 for @large_array;
print "Processed $processed even numbers\n";

# Performance 3: Statement modifier with early exit
sub fast_check {
    my ($data) = @_;
    return "success" if $data && length($data) > 0;
    return "failure";
}

# Edge cases and complex scenarios

# Edge 1: Multiple statement modifiers
my $multi_var = 1;
print "First modifier\n" if $multi_var;
print "Second modifier\n" if $multi_var == 1;

# Edge 2: Statement modifier with complex expression
my $complex_var = 5;
print "Complex result: " . ($complex_var * 2 + 1) . "\n" if $complex_var > 3 && $complex_var < 10;

# Edge 3: Statement modifier with subroutine reference
my $sub_ref = sub { return "subroutine result"; };
print $sub_ref->() if ref($sub_ref) eq 'CODE';

# Edge 4: Statement modifier with package variables
package StatementModifierTest;
our $package_var = "package value";
package main;
print $StatementModifierTest::package_var . "\n" if defined $StatementModifierTest::package_var;

# Statement modifiers with object-oriented patterns

# OO 1: Statement modifier with method call
# my $obj = Some::Class->new();
# print $obj->get_value() if $obj->is_valid();

# OO 2: Statement modifier with inheritance
# package Base;
# sub check { return 1; }
# package Derived;
# use base 'Base';
# package main;
# my $derived = bless {}, 'Derived';
# print "Derived check passed\n" if $derived->check();

# Statement modifiers with modern Perl features

# Modern 1: Statement modifier with signatures (Perl 5.20+)
# sub modern_sub ($value) {
#     print "Value: $value\n" if defined $value;
# }

# Modern 2: Statement modifier with try/catch (Perl 5.34+)
# try {
#     risky_operation();
# } catch ($e) {
#     warn "Caught: $e\n" if $e;
# }

# Statement modifiers with cross-file interactions

# Cross-file 1: Statement modifier with module use
# use Some::Module if $ENV{DEBUG};
# print "Debug mode enabled\n" if $Some::Module::DEBUG;

# Cross-file 2: Statement modifier with require
# require 'external.pl' if -f 'external.pl';
# print "External file loaded\n" if defined &external_function;

# Cross-file 3: Statement modifier with do
# my $config = do 'config.pl' if -f 'config.pl';
# print "Config loaded\n" if $config;

# Real-world statement modifier examples

# Example 1: Debug output
my $debug_mode = 1;
print "Debug: variable = $variable\n" if $debug_mode;

# Example 2: Conditional logging
my $log_level = 2;
print "INFO: Normal operation\n" if $log_level >= 1;
print "DEBUG: Detailed info\n" if $log_level >= 2;

# Example 3: Input validation
my $input = "user input";
print "Input valid\n" if $input =~ /^\w+$/;

# Example 4: Configuration checks
my %settings = (feature_x => 1, feature_y => 0);
enable_feature_x() if $settings{feature_x};
enable_feature_y() if $settings{feature_y};

# Example 5: Resource management
my $resource_allocated = 1;
cleanup_resource() if $resource_allocated;

# Statement modifier best practices

# Good: Simple, readable conditions
print "Processing complete\n" if $success;

# Good: Clear intent
next if $skip_item;

# Good: Concise error handling
warn "Invalid input: $input\n" unless valid_input($input);

# Avoid: Too complex conditions
# print "Complex\n" if $a && $b || $c && ($d || $e) && $f;

# Avoid: Side effects in conditions
# print "Side effect\n" if ($x = get_value()) > 0;

print "StatementModifier tests completed successfully\n";