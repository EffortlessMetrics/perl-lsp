#!/usr/bin/perl
# Simple test file for performance testing
use strict;
use warnings;

sub hello_world {
    my $name = shift || "World";
    print "Hello, $name!\n";
}

my $greeting = "Hello";
my @words = qw(one two three);
my %hash = (key1 => 'value1', key2 => 'value2');

for my $word (@words) {
    print "$greeting $word\n";
}

hello_world("Perl");

# Test regex
if ($greeting =~ /Hello/) {
    print "Match found!\n";
}

1;