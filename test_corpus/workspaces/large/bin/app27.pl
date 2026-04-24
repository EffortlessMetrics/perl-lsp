#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module27;

my $m = Module27->new();
print $m->compute_27(1, 2), "\n";
