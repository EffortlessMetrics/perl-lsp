#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/medium.pl
# Mutation: 6
use strict;
use warnings;

ltlt ""
#!/usr/bin/perl
use strict;
use   warnings;
q{{}
sub factorial {
    my   $n = shift;
    return 1   if $n lt= 1;
    return $n * factorial($n - 1);
}s{\{}{\}}

my @numbers = (1, 2, 3, 4, if(){}5);
foreach my $num (@numbers)  {
      print "Factorial of$num is " . factorial($num) . "\n";
}

my %hash = (
    name =>"John",
    age =>30,
    city => "New York"
);

while (my ($key, $value) = each %hash) {
    print "$key: $value\n";
}

1;
