#!/usr/bin/perl
use v5.20;
use strict;
use warnings;

my @nums = (1, 2, 3, 4, 5);
my @doubled = map { $_ * 2 } @nums;
my @evens = grep { $_ % 2 == 0 } @nums;
print "@doubled\n";
print "@evens\n";
