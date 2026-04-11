#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module9;

my $m = Module9->new();
print $m->compute_9(1, 2), "\n";
