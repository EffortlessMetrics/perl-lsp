#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module6;

my $m = Module6->new();
print $m->compute_6(1, 2), "\n";
