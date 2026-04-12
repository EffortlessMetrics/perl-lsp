#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module33;

my $m = Module33->new();
print $m->compute_33(1, 2), "\n";
