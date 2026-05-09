package Clean::ObjectAttributes;
use strict;
use warnings;
use Moo;

=head1 NAME

Clean::ObjectAttributes - object attribute native critic fixture

=head1 DESCRIPTION

Keeps common Moo-style attribute declarations quiet under native critic.

=cut

has name => (
    is       => 'ro',
    required => 1,
);

has tags => (
    is      => 'ro',
    default => sub { [] },
);

sub label {
    my ($self) = @_;
    return $self->name;
}

1;
