#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module15;

my $m = Module15->new();
print $m->compute_15(1, 2), "\n";
