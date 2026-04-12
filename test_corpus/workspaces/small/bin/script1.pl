#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module1;

my $obj = Module1->new();
my $result = $obj->helper_1(42);
print "result: $result\n";
