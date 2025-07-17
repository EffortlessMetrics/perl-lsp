#!/usr/bin/perl
use strict;
use warnings;

sub factorial {
    my $n = shift;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

my @numbers = (1, 2, 3, 4, 5);
foreach my $num (@numbers) {
    print "Factorial of $num is " . factorial($num) . "\n";
}

my %hash = (
    name => "John",
    age => 30,
    city => "New York"
);

while (my ($key, $value) = each %hash) {
    print "$key: $value\n";
}
