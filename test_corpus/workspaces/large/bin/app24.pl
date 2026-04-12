#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module24;

my $m = Module24->new();
print $m->compute_24(1, 2), "\n";
