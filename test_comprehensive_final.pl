#\!/usr/bin/perl
use strict;
use warnings;

# Variables
my $scalar = 42;
my @array = (1, 2, 3);
my %hash = (key => "value");

# String interpolation
my $name = "World";
my $greeting = "Hello, $name\!";
my $array_str = "Array: @array";

# Operators
my $sum = 10 + 20;
my $result = $scalar > 10 ? "big" : "small";

# Control flow
if ($scalar == 42) {
    print "The answer\!\n";
}

# Loops
foreach my $item (@array) {
    print "Item: $item\n";
}

# Subroutines
sub add {
    my ($x, $y) = @_;
    return $x + $y;
}

# Regular expressions
if ($greeting =~ /World/) {
    print "Found World\n";
}

my $regex = qr/pattern/i;

print "Done\!\n";
