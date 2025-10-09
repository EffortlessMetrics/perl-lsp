#!/usr/bin/env perl
use strict;
use warnings;

# Script with expression evaluation for DAP testing

sub calculate {
    my ($x, $y) = @_;
    my $sum = $x + $y;
    my $product = $x * $y;
    
    # Breakpoint here for evaluation testing
    print "Sum: $sum, Product: $product\n";
    
    return ($sum, $product);
}

my $a = 10;
my $b = 20;
my ($result1, $result2) = calculate($a, $b);

print "Results: $result1, $result2\n";

exit 0;
