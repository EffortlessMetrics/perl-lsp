#!/usr/bin/env perl
use strict;
use warnings;

# Simple hello world script for DAP testing

sub greet {
    my ($name) = @_;
    print "Hello, $name!\n";
    return "Greeted $name";
}

my $message = "World";
my $result = greet($message);
print "Result: $result\n";

exit 0;
