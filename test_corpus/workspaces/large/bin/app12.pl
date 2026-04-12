#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module12;

my $m = Module12->new();
print $m->compute_12(1, 2), "\n";
