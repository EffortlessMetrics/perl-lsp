#!/usr/bin/perl
use strict;
use warnings;

# Structured goto usage with labels and tail-call style jumps.

my $counter = 0;

START_LOOP:
$counter++;
if ($counter < 2) {
    goto START_LOOP;
}

sub fallback {
    return "fallback";
}

sub dispatch {
    my ($flag) = @_;
    if ($flag) {
        goto &fallback;
    }
    return "done";
}

my $result = dispatch(1);
print "$result\n";

