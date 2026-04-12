#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module44;

my $m = Module44->new();
print $m->compute_44(1, 2), "\n";
