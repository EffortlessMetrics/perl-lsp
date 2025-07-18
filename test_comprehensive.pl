#!/usr/bin/perl
use strict;
use warnings;

# Comment test
my $x = 5;  # inline comment

# Control flow
if ($x > 3) {
    print "x is big\n";
} else {
    print "x is small\n";
}

# Loops
for (my $i = 0; $i < 10; $i++) {
    print "$i ";
}

foreach my $item (1..5) {
    print "Item: $item\n";
}

# Subroutines
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

my $sum = add(3, 4);

# Method calls
my $obj = MyClass->new();
$obj->method();

# Operators
my $y = $x * 2 + 3;
my $z = $x ** 2;
my $str = "Hello" . " World";

# Return
sub test_return {
    return if $x < 0;
    return ($x, $y, $z);
}

1;