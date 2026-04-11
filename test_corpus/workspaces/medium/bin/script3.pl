#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module3;

my $obj = Module3->new();
print $obj->process_3("test"), "\n";
