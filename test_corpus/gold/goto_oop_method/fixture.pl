#!/usr/bin/perl
use strict;
use warnings;

package Counter;

sub new {
    my ($class) = @_;
    return bless { 'count' => 0 }, $class;
}

sub increment {
    my ($self) = @_;
    $self->{'count'}++;
}

sub value {
    my ($self) = @_;
    return $self->{'count'};
}

package main;

my $c = Counter->new();
$c->increment();
print $c->value(), "\n";
