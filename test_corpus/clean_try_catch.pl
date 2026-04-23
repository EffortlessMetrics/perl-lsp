#!/usr/bin/env perl
# Test: try/catch
# NodeKinds: Try
use strict;
use warnings;

# Try/catch (NodeKind::Try)
try {
    die "something went wrong";
}
catch ($e) {
    print "caught: $e\n";
}
