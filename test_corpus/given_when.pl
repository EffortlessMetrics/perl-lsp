#!/usr/bin/env perl
# Test: Comprehensive Given/When/Default and Match Operations
# Impact: Ensures parser handles switch-like constructs and smart matching
# NodeKinds: Given, When, Default, Match

use strict;
use warnings;
use feature 'switch';
no warnings 'experimental::smartmatch';

# Basic given/when/default
my $value = 2;
given ($value) {
    when (1) { print "One\n"; }
    when (2) { print "Two\n"; }
    default { print "Other\n"; }
}

# Given with string matching
my $string = "hello";
given ($string) {
    when (/^h/) { print "Starts with h\n"; }
    when (/o$/) { print "Ends with o\n"; }
    default { print "No match\n"; }
}

# Given with array matching
my @array = (1, 2, 3);
given (@array) {
    when ([1, 2, 3]) { print "Exact array match\n"; }
    when (@array > 2) { print "Array has more than 2 elements\n"; }
    default { print "No array match\n"; }
}

# Given with hash matching
my %hash = (a => 1, b => 2);
given (%hash) {
    when ({a => 1}) { print "Hash has a=1\n"; }
    when (exists $hash{b}) { print "Hash has key b\n"; }
    default { print "No hash match\n"; }
}

# Nested given/when
given ($value) {
    when (1) {
        given ($string) {
            when (/hello/) { print "One and hello\n"; }
            default { print "One but not hello\n"; }
        }
    }
    when (2) {
        print "Two\n";
    }
    default {
        print "Other value\n";
    }
}

# Given with continue blocks (if supported)
given ($value) {
    when (1) { print "Value is 1\n"; }
    when (2) { print "Value is 2\n"; }
    default { print "Value is other\n"; }
}
# Note: continue blocks with given/when may not be supported in all Perl versions

# Given with complex expressions
given ($value * 10 + 5) {
    when ($_ > 20) { print "Greater than 20\n"; }
    when ($_ < 10) { print "Less than 10\n"; }
    default { print "Between 10 and 20\n"; }
}

# When with multiple conditions
given ($value) {
    when (1) { print "One\n"; }
    when (2) { print "Two\n"; }
    when (3) { print "Three\n"; }
    when (4) { print "Four\n"; }
    default { print "Many other values\n"; }
}

# Default with complex logic
given ($value) {
    when (1) { print "Special case 1\n"; }
    when (2) { print "Special case 2\n"; }
    default {
        if ($value > 10) {
            print "Large number\n";
        } elsif ($value < 0) {
            print "Negative number\n";
        } else {
            print "Other positive number\n";
        }
    }
}

# Smart match operator (~~)
my $num = 42;
print "Number match\n" if $num ~~ 42;
print "Number range match\n" if $num ~~ (40..50);
print "Number regex match\n" if $num ~~ qr/^\d+$/;

# Array smart matching
my @numbers = (1, 2, 3, 4, 5);
print "Array contains 3\n" if @numbers ~~ 3;
print "Array matches pattern\n" if @numbers ~~ qr/3/;
print "Array size match\n" if @numbers ~~ 5;

# Hash smart matching
my %config = (debug => 1, verbose => 0);
print "Hash has debug key\n" if %config ~~ 'debug';
print "Hash matches pattern\n" if %config ~~ qr/debug/;
print "Hash key exists\n" if 'debug' ~~ %config;

# Code reference smart matching
my $is_even = sub { $_[0] % 2 == 0 };
print "Number is even\n" if $num ~~ $is_even;

# Type smart matching
my $array_ref = [1, 2, 3];
my $hash_ref = {a => 1};
print "Is array reference\n" if $array_ref ~~ 'ARRAY';
print "Is hash reference\n" if $hash_ref ~~ 'HASH';

# Complex smart match scenarios
my $pattern = qr/test/;
my $test_string = "this is a test";
print "String matches pattern\n" if $test_string ~~ $pattern;

# Given with smart match
my $test_value = "test string";
given ($test_value) {
    when ($pattern) { print "Matches pattern\n"; }
    when (length $_ > 10) { print "Long string\n"; }
    when (/string$/) { print "Ends with 'string'\n"; }
    default { print "No specific match\n"; }
}

# Given with undef handling
my $undef_value = undef;
given ($undef_value) {
    when (undef) { print "Value is undefined\n"; }
    when (defined) { print "Value is defined\n"; }
    default { print "Unexpected case\n"; }
}

# Given with references
my $ref_value = [1, 2, 3];
given ($ref_value) {
    when ('ARRAY') { print "Array reference\n"; }
    when ('HASH') { print "Hash reference\n"; }
    when ('CODE') { print "Code reference\n"; }
    default { print "Other reference type\n"; }
}

# Given with boolean context
given ($value) {
    when (1) { print "True value\n"; }
    when (0) { print "False value\n"; }
    when ("") { print "Empty string\n"; }
    default { print "Other truthy value\n"; }
}

# Complex when conditions with logical operators
given ($value) {
    when (1 || 2) { print "One or two\n"; }
    when (3 && 4) { print "Three and four (never true)\n"; }
    when ($value > 0 && $value < 10) { print "Between 0 and 10\n"; }
    default { print "No logical match\n"; }
}

# Given with subroutine calls
sub check_value {
    my ($v) = @_;
    return $v > 5;
}

given ($value) {
    when (check_value($_)) { print "Value > 5\n"; }
    when (check_value($_ * 2)) { print "Value * 2 > 5\n"; }
    default { print "Value not > 5\n"; }
}

# Nested smart matches
my @nested = [1, 2, [3, 4]];
print "Nested structure match\n" if @nested ~~ [1, 2, [3, 4]];

print "All given/when/default and match tests completed\n";