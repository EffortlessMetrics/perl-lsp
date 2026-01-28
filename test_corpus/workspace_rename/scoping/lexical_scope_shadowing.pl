#!/usr/bin/env perl
# Test: Lexical scope and variable shadowing
# Input: Rename outer $var to $renamed (should not affect inner scope)

use strict;
use warnings;

my $var = "outer";
print "Outer: $var\n";

{
    my $var = "inner";
    print "Inner: $var\n";
}

sub process {
    my $var = "function";
    return $var;
}

print "Back to outer: $var\n";
