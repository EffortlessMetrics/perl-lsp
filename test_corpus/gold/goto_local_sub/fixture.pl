#!/usr/bin/perl
use strict;
use warnings;

sub compute {
    my ($x) = @_;
    return $x * 2;
}

my $result = compute(21);
print $result, "\n";
