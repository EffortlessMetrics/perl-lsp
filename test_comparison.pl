#!/usr/bin/perl
use strict;
use warnings;

my $greeting = "Hello, World!";
print $greeting, "\n";

sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

my $result = add(5, 3);
print "5 + 3 = $result\n";
