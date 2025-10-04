#!/usr/bin/env perl
use strict;
use warnings;

# Script with loops for DAP stepping tests

my @numbers = (1, 2, 3, 4, 5);
my $total = 0;

foreach my $num (@numbers) {
    $total += $num;
    print "Current total: $total\n";
}

print "Final total: $total\n";

for (my $i = 0; $i < 3; $i++) {
    print "Loop iteration: $i\n";
}

exit 0;
