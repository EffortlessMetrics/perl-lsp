#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module49;

my $m = Module49->new();
print $m->compute_49(1, 2), "\n";
