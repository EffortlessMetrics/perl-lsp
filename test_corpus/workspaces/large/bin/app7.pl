#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module7;

my $m = Module7->new();
print $m->compute_7(1, 2), "\n";
