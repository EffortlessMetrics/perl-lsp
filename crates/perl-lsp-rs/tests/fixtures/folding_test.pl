#!/usr/bin/perl
use strict;
use warnings;

# Test file for folding range functionality
# Contains foldable regions: subroutines, blocks, POD

package FoldingTest;

# Subroutine fold region (lines 11-15)
sub first_sub {
    my ($x) = @_;
    print "First sub: $x\n";
    return $x * 2;
}

# Another subroutine fold region (lines 18-25)
sub second_sub {
    my ($data) = @_;
    if (defined $data) {
        print "Data: $data\n";
    } else {
        print "No data\n";
    }
}

# Control structure folds (lines 28-35)
if (1) {
    my @items = (1, 2, 3);
    foreach my $item (@items) {
        if ($item > 1) {
            print "$item\n";
        }
    }
}

=head1 NAME

FoldingTest - Test module for folding ranges

=head1 DESCRIPTION

This POD section should also be foldable.
Contains documentation that can be collapsed.

=cut

1;
