#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module6;

my $obj = Module6->new();
print $obj->process_6("test"), "\n";
