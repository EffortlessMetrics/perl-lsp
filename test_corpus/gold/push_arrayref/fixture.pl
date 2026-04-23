#!/usr/bin/perl
use v5.20;
use strict;
use warnings;

my $arrayref = [1, 2, 3];
push @$arrayref, 4, 5;
print "@$arrayref\n";
