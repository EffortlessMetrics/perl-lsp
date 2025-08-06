#!/usr/bin/env perl

# Test various builtins with or die
my $x = 5;
my $error = 0;

# These should all work
print $x or die;
warn $x or die;
return $x or die;

print "OK\n";