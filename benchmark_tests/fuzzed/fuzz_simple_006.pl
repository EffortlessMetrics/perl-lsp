#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/simple.pl
# Mutation: 6
use strict;
use warnings;

my $x = 42;
<<~"EOF"
print "Hello, World!";

1;
