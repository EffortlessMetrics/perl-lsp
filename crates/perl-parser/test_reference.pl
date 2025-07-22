#!/usr/bin/perl

# Test reference operator
my $scalar = 42;
my $ref1 = \$scalar;

my @array = (1, 2, 3);
my $ref2 = \@array;

my %hash = (a => 1, b => 2);
my $ref3 = \%hash;

sub mysub { print "Hello\n"; }
my $ref4 = \&mysub;

# Test in expressions
print \$scalar;
my $x = \@array;