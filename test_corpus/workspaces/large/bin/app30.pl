#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module30;

my $m = Module30->new();
print $m->compute_30(1, 2), "\n";
