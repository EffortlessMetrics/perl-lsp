#!/usr/bin/perl
use strict;
use warnings;

package Animal;

sub new {
    my ($class, %args) = @_;
    return bless { name => $args{name} }, $class;
}

sub speak {
    my ($self) = @_;
    return "...";
}

sub name {
    my ($self) = @_;
    return $self->{name};
}

package main;

my $dog = Animal->new(name => "Rex");
my $sound = $dog->speak();
