#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module7;

my $obj = Module7->new();
print $obj->process_7("test"), "\n";
