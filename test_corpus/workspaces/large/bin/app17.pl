#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module17;

my $m = Module17->new();
print $m->compute_17(1, 2), "\n";
