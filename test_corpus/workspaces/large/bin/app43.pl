#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module43;

my $m = Module43->new();
print $m->compute_43(1, 2), "\n";
