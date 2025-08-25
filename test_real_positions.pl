#!/usr/bin/perl
# Test file for position calculations

package BaseClass;

sub new {
    my $class = shift;
    return bless {}, $class;
}

package DerivedClass;
use base 'BaseClass';

sub new {
    my $class = shift;
    my $self = $class->SUPER::new();
    return $self;
}

1;