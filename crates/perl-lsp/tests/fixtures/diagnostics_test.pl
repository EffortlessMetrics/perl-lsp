#!/usr/bin/perl
use strict;
use warnings;

# Test file for diagnostics - intentionally contains issues

# Line 7: undefined variable reference
print $undefined_var;

# Missing semicolon (line 10)
my $x = 1

# Unclosed string (line 13)
my $str = "unclosed

# Using variable before declaration (line 16)
$result = calculate();
my $result;

sub calculate {
    return 42;
}

1;
