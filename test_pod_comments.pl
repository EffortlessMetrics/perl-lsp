#!/usr/bin/perl
use strict;
use warnings;

# Regular comment
my $x = 5;

=pod
This is a POD comment block
It can span multiple lines
=cut

my $y = 10;  # This should not be consumed

=head1 NAME
Test - A test module
=cut

sub foo {
    return 42;
}

print "x=$x, y=$y\n";