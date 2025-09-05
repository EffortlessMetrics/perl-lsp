#!/usr/bin/env perl
# Test: given/when/default constructs
# Impact: ensures parser handles switch-style control flow

use feature 'switch';
no warnings 'experimental::smartmatch';

my $x = 2;

given ($x) {
    when (1) { say 'one'; }
    when (2) { say 'two'; }
    default { say 'other'; }
}

