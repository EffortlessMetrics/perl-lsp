# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Component;
use Moose;
use Moose::Util qw(find_meta);
use Carp 'croak';

with 'MooseX::Emulate::Class::Accessor::Fast';

has _config => (is => 'rw', default => sub { {} });

sub new {
    my ($class, %args) = @_;
    my $self = $class->SUPER::new(%args);
    return $self;
}

sub config {
    my ($self, %args) = @_;
    if (%args) {
        $self->_config({ %{$self->_config}, %args });
        return $self;
    }
    return $self->_config;
}

sub process { croak ref(shift) . " did not override Catalyst::Component::process" }

sub COMPONENT {
    my ($class, $application, $args) = @_;
    return $class->new($args ? %$args : ());
}

sub ACCEPT_CONTEXT { $_[0] }

__PACKAGE__->meta->make_immutable;
no Moose;

1;
