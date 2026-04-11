#!/usr/bin/env perl
use strict;
use warnings;
use lib '../lib';
use Module9;

my $obj = Module9->new();
print $obj->process_9("test"), "\n";
