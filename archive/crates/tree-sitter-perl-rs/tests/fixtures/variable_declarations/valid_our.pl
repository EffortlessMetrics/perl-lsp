#!/usr/bin/env perl
# Valid 'our' variable declaration test fixtures
# Tests for AC1: Variable declaration error handling

# Basic scalar declaration
our $scalar = 1;

# Basic array declaration
our @array = (1, 2, 3);

# Basic hash declaration
our %hash = (key => 'value');

# Multiple declarations
our ($x, $y, $z);

# Package-scoped declaration
package MyPackage;
our $package_var = "scoped";

# Uninitialized declaration
our $uninitialized;
