#!/usr/bin/env perl

# Test explicit forms
my $error = 0;

# These should all work
die() if $error;
die if $error;
(die) if $error;

print "OK\n";