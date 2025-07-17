#!/usr/bin/perl
use strict;
use warnings;

my $greeting = "Hello from xtask!";
print "$greeting\n";

sub test_function {
    my ($x, $y) = @_;
    return $x + $y;
}

my $result = test_function(5, 3);
print "Result: $result\n";