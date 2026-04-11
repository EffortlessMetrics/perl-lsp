#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module11;

my $m = Module11->new();
print $m->compute_11(1, 2), "\n";
