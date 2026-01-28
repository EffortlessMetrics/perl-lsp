#!/usr/bin/env perl
# Test: Single file variable rename
# Input: Rename $old_var to $new_var

use strict;
use warnings;

my $old_var = 42;
print "Value: $old_var\n";

sub process {
    my $old_var = shift;
    return $old_var * 2;
}

my $result = process($old_var);
print "Result: $result\n";
