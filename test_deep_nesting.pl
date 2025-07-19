#!/usr/bin/env perl
# Generate deeply nested expression
my $depth = 20;
my $expr = "42";
for (1..$depth) {
    $expr = "($expr)";
}
print "# Expression with $depth levels of nesting\n";
print "$expr\n";