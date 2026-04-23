#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module20;

my $m = Module20->new();
print $m->compute_20(1, 2), "\n";
