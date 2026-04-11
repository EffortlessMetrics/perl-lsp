#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module22;

my $m = Module22->new();
print $m->compute_22(1, 2), "\n";
