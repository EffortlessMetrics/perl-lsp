#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module13;

my $m = Module13->new();
print $m->compute_13(1, 2), "\n";
