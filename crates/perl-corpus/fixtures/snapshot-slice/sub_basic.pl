package Snapshot::SubBasic;

use strict;
use warnings;

sub greet {
    my ($name) = @_;
    return "Hello, $name";
}

sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

1;
