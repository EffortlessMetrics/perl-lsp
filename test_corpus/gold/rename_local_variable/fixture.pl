#!/usr/bin/perl
use strict;
use warnings;

sub calculate {
    my $counter = 0;
    $counter += $_ for 1..10;
    return $counter;
}

my $result = calculate();
print "Result: $result\n";
