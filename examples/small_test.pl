#!/usr/bin/env perl
use strict;
use warnings;

my $name = "Test";
my @numbers = (1..10);
my %data = (name => $name, count => scalar @numbers);

# Reference tests
my $scalar_ref = \$name;
my $array_ref = \@numbers;
my $hash_ref = \%data;

# Modern features
my $octal = 0o755;
sub todo { ... }
my $π = 3.14159;

for (@numbers) {
    print "$_\n" if $_ % 2;
}

$name =~ s/Test/Production/g;
