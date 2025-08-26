package MyPackage;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub method_one {
    my ($self, $param) = @_;
    return $param * 2;
}

sub method_two {
    my ($self) = @_;
    return $self->{value};
}

1;
