#!/usr/bin/env perl
use v5.10;
# Valid 'state' variable declaration test fixtures
# Tests for AC1: Variable declaration error handling

# Basic scalar state declaration
sub counter {
    state $count = 0;
    return ++$count;
}

# State array declaration
sub array_state {
    state @array = (1, 2, 3);
    return @array;
}

# State hash declaration
sub hash_state {
    state %cache = ();
    return %cache;
}

# Multiple state declarations
sub multi_state {
    state ($x, $y) = (0, 0);
    return ($x++, $y++);
}

# State with complex initialization
sub complex_state {
    state $value = expensive_computation();
    return $value;
}

sub expensive_computation { return 42; }
