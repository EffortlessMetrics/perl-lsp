#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module1;

my $m = Module1->new();
print $m->compute_1(1, 2), "\n";
