#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module50;

my $m = Module50->new();
print $m->compute_50(1, 2), "\n";
