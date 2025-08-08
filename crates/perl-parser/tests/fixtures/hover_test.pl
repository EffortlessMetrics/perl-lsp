#!/usr/bin/perl
use strict;
use warnings;

# Test fixture for hover documentation

sub calculate_sum {
    my ($a, $b) = @_;
    return $a + $b;
}

my $result = calculate_sum(5, 10);
print "Result: $result\n";

# Built-in function for hover test
my @array = (1, 2, 3, 4, 5);
my $count = scalar @array;
my $joined = join(", ", @array);

# Package and method for hover
package Calculator;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub add {
    my ($self, $x, $y) = @_;
    return $x + $y;
}

1;