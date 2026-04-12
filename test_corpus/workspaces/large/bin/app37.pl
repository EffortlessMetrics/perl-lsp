#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module37;

my $m = Module37->new();
print $m->compute_37(1, 2), "\n";
