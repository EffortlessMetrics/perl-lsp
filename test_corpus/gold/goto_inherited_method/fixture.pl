#!/usr/bin/perl
use strict;
use warnings;

package Animal;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub speak {
    return "...";
}

package Dog;
use parent -norequire, 'Animal';

sub new {
    my ($class) = @_;
    return $class->SUPER::new();
}

package main;

my $dog = Dog->new();
my $sound = $dog->speak();
