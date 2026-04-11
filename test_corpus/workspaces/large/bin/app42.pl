#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module42;

my $m = Module42->new();
print $m->compute_42(1, 2), "\n";
