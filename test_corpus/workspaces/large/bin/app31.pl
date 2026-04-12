#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module31;

my $m = Module31->new();
print $m->compute_31(1, 2), "\n";
