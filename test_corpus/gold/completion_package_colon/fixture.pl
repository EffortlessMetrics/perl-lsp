#!/usr/bin/perl
use strict;
use warnings;

package MathUtils;

sub square {
    my ($n) = @_;
    return $n * $n;
}

sub cube {
    my ($n) = @_;
    return $n * $n * $n;
}

package main;

my $sq = MathUtils::square(5);
print $sq, "\n";
