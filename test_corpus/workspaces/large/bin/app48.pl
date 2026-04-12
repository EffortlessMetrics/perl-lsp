#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module48;

my $m = Module48->new();
print $m->compute_48(1, 2), "\n";
