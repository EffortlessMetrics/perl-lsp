#!/usr/bin/env perl
use strict;
use warnings;

# Variables
my $scalar = "Hello, World!";
my @array = (1..10);
my %hash = map { $_ => $_ * 2 } 1..5;

# References (testing the \ operator)
my $sref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Modern octal
my $perms = 0o755;
my $old_perms = 0755;

# Ellipsis
sub todo {
    ...
}

# Unicode identifiers
my $π = 3.14159;
my $café = "coffee shop";
sub 日本語 { return "Japanese" }

# Control flow
for my $i (@array) {
    print "$i\n" if $i % 2 == 0;
}

# Regex
my $text = "foo bar baz";
$text =~ s/foo/FOO/g;

1;
