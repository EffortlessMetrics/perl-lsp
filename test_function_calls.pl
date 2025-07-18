#!/usr/bin/perl
# Simple function calls
print "Hello\n";
print("Hello\n");

# Multiple arguments
print "Hello", " ", "World\n";
print("Hello", " ", "World\n");

# Complex arguments
my $result = join(",", 1, 2, 3);
my @sorted = sort { $a cmp $b } @array;

# Function calls with expressions
my $sum = add(5 + 3, 2 * 4);
my $value = process_data($hash{key}, $array[0]);