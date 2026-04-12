#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module23;

my $m = Module23->new();
print $m->compute_23(1, 2), "\n";
