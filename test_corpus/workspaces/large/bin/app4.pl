#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module4;

my $m = Module4->new();
print $m->compute_4(1, 2), "\n";
