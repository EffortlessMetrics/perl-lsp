#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module2;

my $obj = Module2->new();
print $obj->process_2("test"), "\n";
