#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module4;

my $obj = Module4->new();
print $obj->process_4("test"), "\n";
