#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module29;

my $m = Module29->new();
print $m->compute_29(1, 2), "\n";
