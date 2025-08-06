#!/usr/bin/env perl

# Comprehensive test of builtins with modifiers
my $x = 5;
my $error = 0;

# These should all parse correctly

# Simple cases (these work)
die if $error;
die unless $x;
return if $error;
return unless $x;

# With arguments (these work)
die "Error" if $error;
return $x if $error;
print $x if $error;

# With word operators (some work, some don't)
$x or die if $error;
print $x or die if $error;
warn $x or die if $error;

# The problematic case
return $x or die if $error;

print "OK\n";