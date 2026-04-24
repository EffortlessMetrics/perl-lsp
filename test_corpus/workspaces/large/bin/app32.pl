#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module32;

my $m = Module32->new();
print $m->compute_32(1, 2), "\n";
