#!/usr/bin/perl
use strict;
use warnings;

# Test fat arrow in hash literal
my %hash1 = (key => 'value');
my %hash2 = (a => 1, b => 2);

# Test hash references
my $href1 = { key => 'value' };
my $href2 = { a => 1, b => 2 };
my $href3 = { foo => 'bar', baz => 'qux' };

# Test empty hash ref
my $empty = {};

# Test mixed commas and fat arrows
my %hash3 = (first => 1, 'second', 2, third => 3);
my $href4 = { 'one', 1, two => 2 };

# Test in function calls
print(foo => 'bar', baz => 'qux');

# Test nested structures
my $complex = {
    name => 'John',
    age => 30,
    address => {
        street => '123 Main St',
        city => 'Anytown'
    }
};