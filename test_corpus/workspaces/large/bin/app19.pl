#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module19;

my $m = Module19->new();
print $m->compute_19(1, 2), "\n";
