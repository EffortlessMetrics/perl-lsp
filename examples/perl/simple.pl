#!/usr/bin/perl
use strict;
use warnings;

# Simple variable declarations
my $name = "World";
my $count = 42;
my @array = (1, 2, 3, 4, 5);
my %hash = (
    name => "Perl",
    version => 5.38,
);

# Basic subroutine
sub greet {
    my ($who) = @_;
    return "Hello, $who!";
}

# Control flow
if ($count > 40) {
    print greet($name), "\n";
}

# Loops
for my $i (@array) {
    print "$i ";
}
print "\n";

# Hash iteration
while (my ($key, $value) = each %hash) {
    print "$key: $value\n";
}