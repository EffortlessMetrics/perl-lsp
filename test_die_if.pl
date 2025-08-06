#!/usr/bin/env perl

# Test die with if modifier
my $error = 1;

die if $error;

print "OK\n";