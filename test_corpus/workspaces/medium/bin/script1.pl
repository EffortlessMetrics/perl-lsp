#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module1;

my $obj = Module1->new();
print $obj->process_1("test"), "\n";
