#!/usr/bin/env perl
# Test: Ternary NodeKind
# Impact: Ensures parser handles conditional (ternary) expressions
# NodeKinds: Ternary
# 
# This file tests the parser's ability to handle:
# 1. Basic ternary expressions (condition ? then : else)
# 2. Nested ternary expressions
# 3. Ternary with complex expressions
# 4. Ternary in different contexts (assignment, return, function calls)
# 5. Ternary with different data types
# 6. Error cases and edge conditions

use strict;
use warnings;

# Basic ternary expressions
my $condition = 1;
my $basic_result = $condition ? "true" : "false";
print "Basic ternary: $basic_result\n";

# Ternary with numeric values
my $num_condition = 10;
my $num_result = $num_condition > 5 ? 100 : 200;
print "Numeric ternary: $num_result\n";

# Ternary with boolean expressions
my $bool_result = ($num_condition > 5 && $num_condition < 15) ? "in range" : "out of range";
print "Boolean ternary: $bool_result\n";

# Nested ternary expressions
my $nested_condition = 7;
my $nested_result = $nested_condition < 5 ? "less than 5" :
                    $nested_condition < 10 ? "between 5 and 9" :
                    $nested_condition < 15 ? "between 10 and 14" :
                    "15 or greater";
print "Nested ternary: $nested_result\n";

# Ternary in assignment context
my $assign_result = $condition ? ($num_condition * 2) : ($num_condition / 2);
print "Assignment ternary: $assign_result\n";

# Ternary in function call context
sub test_function {
    my ($param) = @_;
    return $param ? "received: $param" : "no parameter";
}

my $func_result = test_function($basic_result);
print "Function ternary: $func_result\n";

# Ternary with array operations
my @array1 = (1, 2, 3);
my @array2 = (4, 5, 6);
my $array_condition = 1;
my @array_result = $array_condition ? @array1 : @array2;
print "Array ternary: @array_result\n";

# Ternary with hash operations
my %hash1 = (a => 1, b => 2);
my %hash2 = (c => 3, d => 4);
my $hash_condition = 0;
my %hash_result = $hash_condition ? %hash1 : %hash2;
print "Hash ternary: " . join(', ', %hash_result) . "\n";

# Ternary with string operations
my $string_condition = "hello";
my $string_result = length($string_condition) > 3 ? uc($string_condition) : lc($string_condition);
print "String ternary: $string_result\n";

# Ternary with regex operations
my $regex_text = "Perl123";
my $regex_result = $regex_text =~ /^\d+$/ ? "all digits" :
                   $regex_text =~ /^[a-zA-Z]+$/ ? "all letters" :
                   "mixed content";
print "Regex ternary: $regex_result\n";

# Ternary with file test operations
my $filename = "test_corpus/basic_constructs.pl";
my $file_result = -f $filename ? "file exists" :
                  -d $filename ? "directory exists" :
                  "not found";
print "File test ternary: $file_result\n";

# Ternary with defined/undef checks
my $defined_var = undef;
my $undef_var = "defined";
my $defined_result = defined($defined_var) ? $defined_var : $undef_var;
print "Defined ternary: $defined_result\n";

# Ternary with reference operations
my $array_ref = [1, 2, 3];
my $ref_result = ref($array_ref) eq 'ARRAY' ? "array reference" :
                 ref($array_ref) eq 'HASH' ? "hash reference" :
                 "other reference";
print "Reference ternary: $ref_result\n";

# Ternary in list context
my ($list_result1, $list_result2) = $condition ? (1, 2) : (3, 4);
print "List ternary: $list_result1, $list_result2\n";

# Ternary with complex expressions
my $complex_condition = 15;
my $complex_result = $complex_condition > 10 ? 
                     ($complex_condition % 2 == 0 ? "even > 10" : "odd > 10") :
                     ($complex_condition % 2 == 0 ? "even <= 10" : "odd <= 10");
print "Complex ternary: $complex_result\n";

# Ternary with subroutine calls
sub get_value {
    my ($type) = @_;
    return $type eq 'high' ? 100 :
           $type eq 'medium' ? 50 :
           $type eq 'low' ? 10 :
           0;
}

my $sub_result = get_value('medium');
print "Subroutine ternary: $sub_result\n";

# Ternary with logical operators
my $logical_condition = 0;
my $logical_result = $logical_condition || $condition ? "true" : "false";
print "Logical ternary: $logical_result\n";

# Ternary with bitwise operations
my $bitwise_condition = 5;  # binary 101
my $bitwise_result = ($bitwise_condition & 1) ? "odd" : "even";
print "Bitwise ternary: $bitwise_result\n";

# Ternary with comparison chains
my $value = 75;
my $grade = $value >= 90 ? "A" :
            $value >= 80 ? "B" :
            $value >= 70 ? "C" :
            $value >= 60 ? "D" :
            "F";
print "Grade ternary: $grade\n";

# Ternary in print statement
print "Inline ternary: ", $condition ? "success" : "failure", "\n";

# Ternary with array elements
my @numbers = (10, 20, 30, 40);
my $element_result = $numbers[1] > 15 ? $numbers[2] : $numbers[0];
print "Array element ternary: $element_result\n";

# Ternary with hash elements
my %config = (debug => 1, verbose => 0);
my $config_result = $config{debug} ? "debug mode" : "normal mode";
print "Hash element ternary: $config_result\n";

# Ternary with shift/unshift operations
my @stack = (1, 2, 3);
my $stack_result = scalar(@stack) > 2 ? shift(@stack) : unshift(@stack, 0);
print "Stack operation ternary: $stack_result, stack now: @stack\n";

# Ternary with push/pop operations
my @queue = (1, 2, 3);
my $queue_result = scalar(@queue) > 0 ? pop(@queue) : push(@queue, 0);
print "Queue operation ternary: $queue_result, queue now: @queue\n";

# Ternary with grep/map operations
my @data = (1, 2, 3, 4, 5);
my @filtered = $condition ? grep { $_ % 2 == 0 } @data : map { $_ * 2 } @data;
print "Grep/Map ternary: @filtered\n";

# Ternary with sort operations
my @unsorted = (3, 1, 4, 1, 5);
my @sorted_data = $condition ? sort @unsorted : reverse sort @unsorted;
print "Sort ternary: @sorted_data\n";

# Ternary with join/split operations
my $text_data = "a,b,c,d";
my $join_result = $condition ? join('-', split(',', $text_data)) : join('|', split(',', $text_data));
print "Join/Split ternary: $join_result\n";

# Ternary with time operations
my $time_condition = localtime;
my $time_result = $time_condition =~ /AM/ ? "morning" : "afternoon/evening";
print "Time ternary: $time_result\n";

# Ternary with eval error handling
my $eval_result = eval { 
    $condition ? "success" : die "error";
} || "caught error";
print "Eval ternary: $eval_result\n";

# Complex nested ternary with multiple operations
sub complex_calculation {
    my ($x, $y, $operation) = @_;
    
    return $operation eq 'add' ? $x + $y :
           $operation eq 'subtract' ? $x - $y :
           $operation eq 'multiply' ? $x * $y :
           $operation eq 'divide' ? ($y != 0 ? $x / $y : "division by zero") :
           "unknown operation";
}

my $calc_result = complex_calculation(10, 5, 'divide');
print "Complex calculation ternary: $calc_result\n";

# Ternary with object-oriented operations (blessed reference)
my $obj = bless { value => 42 }, 'TestClass';
my $obj_result = ref($obj) ? $obj->{value} : "not an object";
print "Object ternary: $obj_result\n";

# Ternary with package variable checks
package TestPackage;
our $package_var = "package_value";
package main;
my $package_result = defined $TestPackage::package_var ? $TestPackage::package_var : "undefined";
print "Package variable ternary: $package_result\n";

# Edge case: Ternary with undefined values
my $undef_condition = undef;
my $edge_result = $undef_condition ? "defined" : "undefined";
print "Edge case ternary: $edge_result\n";

# Edge case: Ternary with empty strings
my $empty_condition = "";
my $empty_result = $empty_condition ? "non-empty" : "empty";
print "Empty string ternary: $empty_result\n";

# Edge case: Ternary with zero values
my $zero_condition = 0;
my $zero_result = $zero_condition ? "non-zero" : "zero";
print "Zero ternary: $zero_result\n";

print "Ternary expressions tests completed successfully\n";