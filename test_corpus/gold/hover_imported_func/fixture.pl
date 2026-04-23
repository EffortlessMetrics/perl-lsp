#!/usr/bin/perl
use strict;
use warnings;
use Carp qw(croak confess);

sub check {
    my ($val) = @_;
    croak "value required" unless defined $val;
    return $val;
}

check(42);
