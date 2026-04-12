#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module10;

my $m = Module10->new();
print $m->compute_10(1, 2), "\n";
