#!/usr/bin/perl
use strict;
use warnings;

# Latin-1 encoded file: café is a common word in French Perl comments
# This file uses ISO-8859-1 / Latin-1 encoding (not UTF-8)

package Encoding::Legacy;

sub greet {
    my ($name) = @_;
    # René says: Bonjour à tous!
    return "Salut, $name!";
}

1;
