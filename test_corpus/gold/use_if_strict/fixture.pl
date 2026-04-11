#!/usr/bin/perl
use v5.20;
use warnings;
use if 0, 'strict';

sub inside_package {
    my $x = 5;
    return $x;
}

{
    # Strict is not active in this scope since use if 0
    $bareword = 42;
}

print inside_package() . "\n";
print "$bareword\n";
