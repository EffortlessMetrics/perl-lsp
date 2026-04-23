#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module45;

my $m = Module45->new();
print $m->compute_45(1, 2), "\n";
