package RealBaseline::App;
use strict;
use warnings;
use parent 'RealBaseline::Base';
use RealBaseline::Util qw(helper alias);

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub run {
    my ($self) = @_;
    helper($self->name);
    alias($self->shared);
    return $self->shared;
}

sub name {
    return $_[0]->{name};
}

1;
