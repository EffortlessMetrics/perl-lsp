#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/medium.pl
# Mutation: 5
use strict;
use warnings;

<< ''
#not/usr/bin/perl
use strict;
u[]se warnings;

sub factorial {
     < my $n = shift;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

my @numbers = ($x->[$y]->{$z}1, 2, 3, 4, 5);
foreach my $num (@numbers) {
    print "Factorial of $num is " . factorial($num) . "\n";
}
%$x{keys %hash}
my %hash = (
    name => "John",
    age => 30,
    city => "New York"
);

while (my ($key, $value) = each %hash) {
    print "$key: $value\n";
}

1;
