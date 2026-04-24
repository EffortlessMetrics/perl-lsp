#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module10;

my $obj = Module10->new();
print $obj->process_10("test"), "\n";
