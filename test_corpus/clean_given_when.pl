#!/usr/bin/env perl
# Test: given/when/default
# NodeKinds: Given, When, Default
use strict;
use warnings;

my $x = 42;

# Given/when/default (NodeKind::Given, When, Default)
given ($x) {
    when (1) { print "one\n"; }
    when (42) { print "forty-two\n"; }
    default { print "other\n"; }
}
