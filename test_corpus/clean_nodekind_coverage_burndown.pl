#!/usr/bin/env perl
# Test: Clean coverage for recovery-only NodeKinds
# NodeKinds: Eval, NamedParameter, Tie, Untie, VariableWithAttributes

use strict;
use warnings;

sub accepts_named (:$name, $count = 1) {
    my ($x :Shared, $y) = ($count, $count + 1);

    tie my $slot, 'Tie::StdScalar';
    $slot = $x + $y;

    my $result = eval {
        return "$name:$slot";
    };

    untie $slot;
    return $result;
}

accepts_named(name => 'demo', 2);
