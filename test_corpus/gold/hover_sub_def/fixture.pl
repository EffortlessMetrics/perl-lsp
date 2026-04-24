#!/usr/bin/perl
use strict;
use warnings;

sub greet {
    my ($name) = @_;
    return "Hello, $name!";
}

my $msg = greet("World");
print $msg, "\n";
