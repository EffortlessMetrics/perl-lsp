#!/usr/bin/perl
my $name = "World";
my $count = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value');

# Simple interpolation
print "Hello, $name!\n";
print "Count is $count\n";

# Complex interpolation
print "Name in braces: ${name}\n";
print "Array: @array\n";
print "Complex: ${name}_suffix\n";

# Array interpolation
print "Array items: @{[1, 2, 3]}\n";
print "Array deref: @{$arrayref}\n";