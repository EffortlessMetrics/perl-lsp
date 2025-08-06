#!/usr/bin/env perl

# Test print or die if
my $x = 5;
my $error = 0;

print $x or die if $error;

print "\nOK\n";