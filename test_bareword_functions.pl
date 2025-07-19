#!/usr/bin/env perl

# Test user-defined function calls without parentheses
sub my_func { }
sub process_data { }

# Simple calls
my_func;
my_func 42;
my_func "hello";
my_func $x;
my_func $x, $y;
my_func $x, $y, $z;

# As part of expressions
$result = my_func $data;
@results = process_data $input, $options;

# Mixed with operators
print my_func $x if $condition;

# Chained calls
process_data my_func $x;