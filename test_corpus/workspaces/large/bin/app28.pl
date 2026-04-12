#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module28;

my $m = Module28->new();
print $m->compute_28(1, 2), "\n";
