#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module26;

my $m = Module26->new();
print $m->compute_26(1, 2), "\n";
