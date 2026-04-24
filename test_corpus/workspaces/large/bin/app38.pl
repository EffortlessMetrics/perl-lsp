#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module38;

my $m = Module38->new();
print $m->compute_38(1, 2), "\n";
