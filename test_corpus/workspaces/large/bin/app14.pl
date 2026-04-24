#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module14;

my $m = Module14->new();
print $m->compute_14(1, 2), "\n";
