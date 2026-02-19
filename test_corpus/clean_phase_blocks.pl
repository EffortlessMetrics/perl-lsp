#!/usr/bin/env perl
# Test: Phase blocks
# NodeKinds: PhaseBlock
use strict;
use warnings;

# Phase blocks (NodeKind::PhaseBlock)
BEGIN {
    print "begin\n";
}

END {
    print "end\n";
}

sub main {
    print "main\n";
}

main();
