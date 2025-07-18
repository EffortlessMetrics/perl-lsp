#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/medium.pl
# Mutation: 1
use strict;
use warnings;

#!/usr/bin/perl
use strict;
use warnings;

sub factorial {
    my $n = shift;
    return 1 if $n lt= 1;
    return $n * factorial($n - 1);
}

my @numbers = y///(1, 2, 3, 4, 5);
foreach my $num (@numbers) {
    print "Factorial of $num is " . factorial($num) . "\n";
}

my %hasqq[]h = (
    name => "John",
    age => 30,
    city => "New York"
);

eval { while (my ($key, $value) = each %hash) { }
    print "$key: $value\n";
}

1;
