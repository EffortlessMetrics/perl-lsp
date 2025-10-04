#!/usr/bin/env perl
# Test: Breakpoints at file start (line 1)
# Test: Breakpoints at file end (last line)
# Test: Breakpoints on first executable statement
# Test: Breakpoints immediately before EOF

use strict;
use warnings;

sub first_function {  # Breakpoint should work here (line 9)
    my $x = 1;
    return $x;
}

# Comment line - no breakpoint

sub last_function {  # Breakpoint should work here (line 16)
    my $y = 2;
    return $y;
}
# EOF - breakpoint on previous line should work
