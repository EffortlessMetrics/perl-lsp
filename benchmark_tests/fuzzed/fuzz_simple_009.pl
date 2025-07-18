#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/simple.pl
# Mutation: 9
use strict;
use warnings;

<< EOF
myqr{{}} $x = 42;
print "Hello, World!";
<<HTML

die "error";

1;
