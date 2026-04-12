#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module2;

my $m = Module2->new();
print $m->compute_2(1, 2), "\n";
