#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module36;

my $m = Module36->new();
print $m->compute_36(1, 2), "\n";
