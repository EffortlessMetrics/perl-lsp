#!/usr/bin/env perl
# Test: Subroutine signatures with parameter types
# NodeKinds: Signature, MandatoryParameter, OptionalParameter, NamedParameter, SlurpyParameter
use strict;
use warnings;

# Mandatory parameter (NodeKind::MandatoryParameter)
sub greet($name) {
    print "Hello, $name\n";
}

# Optional parameter with default (NodeKind::OptionalParameter)
sub greet_default($name = "World") {
    print "Hello, $name\n";
}

# Slurpy parameter (NodeKind::SlurpyParameter)
sub sum_all(@values) {
    my $total = 0;
    $total += $_ for @values;
    return $total;
}

# Named/slurpy hash parameter
sub configure(%opts) {
    return \%opts;
}

# Mixed signature
sub complex($first, $second = 10, @rest) {
    return ($first, $second, @rest);
}
