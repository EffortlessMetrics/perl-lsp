#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module21;

my $m = Module21->new();
print $m->compute_21(1, 2), "\n";
