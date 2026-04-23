#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module34;

my $m = Module34->new();
print $m->compute_34(1, 2), "\n";
