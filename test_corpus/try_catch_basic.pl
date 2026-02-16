#!/usr/bin/env perl
# Test: Basic Try/Catch Blocks
# Impact: Ensures parser handles modern exception handling
# NodeKinds: Try

use strict;
use warnings;
use feature 'try';
no warnings 'experimental::try';

# Basic try/catch
try {
    die "Basic error";
}
catch ($e) {
    print "Caught: $e\n";
}

# Try with simple catch
try {
    die "Some error";
}
catch ($e) {
    print "Caught error: $e\n";
}

# Try in subroutine context
sub safe_operation {
    my ($x, $y) = @_;
    try {
        return $x / $y;
    }
    catch ($e) {
        warn "Division failed: $e";
        return undef;
    }
}

# Try with next in loops
foreach my $item (1..5) {
    try {
        die if $item == 3;
        print "Processing $item\n";
    }
    catch ($e) {
        print "Skipped $item due to error\n";
        next;
    }
}

print "All try/catch tests completed\n";