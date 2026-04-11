#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module3;

my $m = Module3->new();
print $m->compute_3(1, 2), "\n";
