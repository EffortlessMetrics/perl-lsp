#!/usr/bin/perl
use strict;
use warnings;

package Calculator;

sub new {
    my $class = shift;
    my $self = {
        precision => shift || 2,
        history => [],
    };
    bless $self, $class;
    return $self;
}

sub add {
    my ($self, $a, $b) = @_;
    my $result = $a + $b;
    push @{$self->{history}}, "add($a, $b) = $result";
    return $result;
}

sub multiply {
    my ($self, $a, $b) = @_;
    my $result = $a * $b;
    push @{$self->{history}}, "multiply($a, $b) = $result";
    return $result;
}

sub history {
    my $self = shift;
    return @{$self->{history}};
}

1;