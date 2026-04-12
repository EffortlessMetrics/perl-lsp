#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module5;

my $m = Module5->new();
print $m->compute_5(1, 2), "\n";
