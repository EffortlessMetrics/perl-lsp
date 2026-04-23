#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module8;

my $obj = Module8->new();
print $obj->process_8("test"), "\n";
