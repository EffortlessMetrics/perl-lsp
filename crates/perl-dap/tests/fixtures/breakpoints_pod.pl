#!/usr/bin/env perl
use strict;
use warnings;

=pod

=head1 NAME

Test Script - No breakpoints in POD sections (lines 6-12)

=head1 DESCRIPTION

This POD block should not allow breakpoints.
Only executable code should have breakpoints.

=cut

sub documented_function {  # Breakpoint should work here (line 17)
    my $x = 1;
    return $x;
}

=pod

=head1 ANOTHER SECTION

More POD that should not allow breakpoints. (lines 23-29)

=cut

my $after_pod = 42;  # Breakpoint should work here (line 31)
