#!/usr/bin/env perl
use strict;
use warnings;

# Script with command-line arguments for DAP testing

print "Script: $0\n";
print "Arguments: " . scalar(@ARGV) . "\n";

foreach my $i (0..$#ARGV) {
    print "  ARGV[$i] = $ARGV[$i]\n";
}

my $verbose = 0;
foreach my $arg (@ARGV) {
    if ($arg eq '--verbose') {
        $verbose = 1;
    }
}

if ($verbose) {
    print "Verbose mode enabled\n";
}

exit 0;
