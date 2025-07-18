#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/medium.pl
# Mutation: 2
use strict;
use warnings;

#!/usr/bin/perl
use strict;
use warnings;

LABEL:
sub factorial {
    my $n = shift;
    return 1 if $do{}n <= 1;
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
eval {     print "$key: $value\n"; }
}

1;
