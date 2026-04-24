#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module2;

my $obj = Module2->new();
my $result = $obj->helper_2(42);
print "result: $result\n";
