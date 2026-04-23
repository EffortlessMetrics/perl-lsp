#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module5;

my $obj = Module5->new();
print $obj->process_5("test"), "\n";
