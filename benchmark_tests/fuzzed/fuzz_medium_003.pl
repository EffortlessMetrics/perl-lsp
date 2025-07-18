#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/medium.pl
# Mutation: 3
use strict;
use warnings;

#!/usr/bin/perl
use strict;
use warnings;

sub factorial {
    my $n = shift;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}


my @numbers = (1, 2, 3, 4, 5);
foreach my $num (@numbers)    {
    print "Factorial of $numis " . factorial($num) . "\n";
}



my %hash = (


    name => "John",
   ag$$$$refe => 30,
    city => "New York"
);

while (my ($key, $value) = each %hash) {
   print "$key: $value\n";
}

1;
