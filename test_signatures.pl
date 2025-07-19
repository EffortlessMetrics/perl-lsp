#!/usr/bin/env perl
# Test signature parsing

# Basic signatures - WORKS
sub basic ($x, $y) { }

# Default values - WORKS
sub defaults ($x = 10) { }

# Slurpy params - WORKS
sub slurpy (@rest) { }

# Type constraints - FAILS
# sub typed (Str $name) { }

# Let's test without type constraints
sub test1 ($a, $b, $c) { }
sub test2 ($x = 5, @rest) { }