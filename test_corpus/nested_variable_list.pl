#!/usr/bin/perl
use strict;
use warnings;

# Corpus fixture for NodeKind::NestedVariableList
# NestedVariableList is produced when a lexical list declaration contains a
# nested parenthesised group with two or more variables.
#
# Pattern: my ($a, ($b, $c)) = ...

# Basic nested list destructuring
my ($first, ($second, $third)) = (1, (2, 3));
print "first=$first, second=$second, third=$third\n";

# Nested group with three inner elements
my ($outer, ($inner_x, $inner_y, $inner_z)) = ('a', ('b', 'c', 'd'));
print "outer=$outer, inner: $inner_x $inner_y $inner_z\n";

# Multiple nested groups in same declaration
my (($alpha, $beta), ($gamma, $delta)) = ((1, 2), (3, 4));
print "alpha=$alpha, beta=$beta, gamma=$gamma, delta=$delta\n";

# Nested list from sub return
sub pair { return (10, 20) }
my ($x, ($p, $q)) = (5, pair());
print "x=$x, p=$p, q=$q\n";

# Nested list with array slurp after nested group
my ($head, ($mid1, $mid2), @rest) = (0, (1, 2), 3, 4, 5);
print "head=$head, mid1=$mid1, mid2=$mid2, rest=@rest\n";

1;
