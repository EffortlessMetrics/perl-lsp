#!/usr/bin/env perl
# Valid 'my' variable declaration test fixtures
# Tests for AC1: Variable declaration error handling

# Basic scalar declaration
my $scalar = 1;

# Basic array declaration
my @array = (1, 2, 3);

# Basic hash declaration
my %hash = (key => 'value');

# Multiple declarations
my ($x, $y, $z) = (1, 2, 3);

# Uninitialized declaration
my $uninitialized;

# Complex expression initialization
my $complex = $x + $y * 2;

# Reference declaration
my $arrayref = [1, 2, 3];
my $hashref = { key => 'value' };

# Declaration with typeglob
my *filehandle;
