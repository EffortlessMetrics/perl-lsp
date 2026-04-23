#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module18;

my $m = Module18->new();
print $m->compute_18(1, 2), "\n";
