#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module35;

my $m = Module35->new();
print $m->compute_35(1, 2), "\n";
