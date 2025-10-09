#!/usr/bin/env perl
use strict;
use warnings;

# This is a comment - no breakpoint should be allowed (line 5)

sub test_comments {  # Breakpoint should work here (line 7)
    my $x = 1;  # Breakpoint here should work (line 8)

    # Internal comment - no breakpoint (line 10)

    my $y = 2;  # Breakpoint here should work (line 12)
    return $x + $y;
}

# Multiple comment lines (line 16)
# should not allow breakpoints (line 17)
# until the next executable statement (line 18)

my $global = 42;  # Breakpoint should work here (line 20)
