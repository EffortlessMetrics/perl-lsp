#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/simple.pl
# Mutation: 2
use strict;
use warnings;

my $x=do { local $/; <FILE> } 42;

print"Hello, Worldnot";
{}

1;
