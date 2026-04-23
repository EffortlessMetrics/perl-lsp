#!/usr/bin/env perl
# Test: C-style for loop and labeled statements
# NodeKinds: For, LabeledStatement
use strict;
use warnings;

# C-style for loop (NodeKind::For)
for (my $i = 0; $i < 10; $i++) {
    print "$i\n";
}

# Labeled statement (NodeKind::LabeledStatement)
OUTER: for (my $i = 0; $i < 5; $i++) {
    for (my $j = 0; $j < 5; $j++) {
        next OUTER if $j == 2;
    }
}

# Another labeled loop
LOOP: while (1) {
    last LOOP;
}
