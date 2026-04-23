#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module40;

my $m = Module40->new();
print $m->compute_40(1, 2), "\n";
