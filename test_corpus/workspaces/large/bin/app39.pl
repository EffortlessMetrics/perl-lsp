#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module39;

my $m = Module39->new();
print $m->compute_39(1, 2), "\n";
