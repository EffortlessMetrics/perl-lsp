#!/usr/bin/env perl
# Test: Miscellaneous NodeKinds
# NodeKinds: No, Undef, Ellipsis, VariableWithAttributes
use strict;
use warnings;

# No pragma (NodeKind::No)
no warnings;

# Undef (NodeKind::Undef)
my $x = undef;

# Ellipsis / yada-yada (NodeKind::Ellipsis)
sub not_yet_implemented {
    ...
}

# Variable with attributes (NodeKind::VariableWithAttributes)
my $shared :shared;
