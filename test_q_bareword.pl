#!/usr/bin/env perl
use strict;
use warnings;

# Test that 'q' can be used as a bareword/identifier
my $q = 5;
print "q = $q\n";

sub q { return "function q" }
print q(), "\n";

my %hash = (q => 'value');
print "hash{q} = $hash{q}\n";