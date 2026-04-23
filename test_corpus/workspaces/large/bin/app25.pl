#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module25;

my $m = Module25->new();
print $m->compute_25(1, 2), "\n";
