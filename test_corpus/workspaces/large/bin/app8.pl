#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module8;

my $m = Module8->new();
print $m->compute_8(1, 2), "\n";
