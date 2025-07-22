#!/usr/bin/perl

# Test label syntax
OUTER: for my $i (1..3) {
    INNER: for my $j (1..3) {
        print "$i,$j\n";
        next OUTER if $j == 2;
    }
}

# Simple label
LOOP: while (1) {
    last LOOP;
}