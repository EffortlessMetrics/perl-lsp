#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module16;

my $m = Module16->new();
print $m->compute_16(1, 2), "\n";
