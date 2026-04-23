#!/usr/bin/env perl
# Test: Multi-file subroutine rename (main file)
# Input: Rename process to enhanced_process

use strict;
use warnings;
use lib 'lib';
use Utils;

my $data = "test data";

# Qualified call
my $result1 = Utils::process($data);

# Bare call (imported)
use Utils qw(process);
my $result2 = process($data);

print "Results: $result1, $result2\n";
