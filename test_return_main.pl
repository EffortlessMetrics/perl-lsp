#!/usr/bin/env perl

# Test return in main scope
my $x = 5;
my $error = 0;

return $x or die if $error;

print "OK\n";