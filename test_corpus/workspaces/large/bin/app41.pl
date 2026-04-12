#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module41;

my $m = Module41->new();
print $m->compute_41(1, 2), "\n";
