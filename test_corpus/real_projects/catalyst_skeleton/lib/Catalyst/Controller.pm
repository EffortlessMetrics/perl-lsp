# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Controller;
use Moose;
use Carp 'croak';
use Scalar::Util qw(blessed);

extends 'Catalyst::Component';

has _application => (is => 'rw', weak_ref => 1);
has namespace    => (is => 'rw', default => '');
has path_prefix  => (is => 'rw', lazy => 1, builder => '_build_path_prefix');
has action_namespace => (is => 'rw', lazy => 1, builder => '_build_action_namespace');

sub _build_path_prefix {
    my $self = shift;
    return lc $self->namespace;
}

sub _build_action_namespace {
    my $self = shift;
    return lc $self->namespace;
}

sub new {
    my ($class, $application, $args) = @_;
    my $self = $class->SUPER::new($args // {});
    $self->_application($application);
    return $self;
}

sub _application { $_[0]->{_application} }

sub action_for {
    my ($self, $action) = @_;
    return $self->_application->dispatcher->get_action($action, $self->namespace);
}

sub BEGIN {
    my $class = shift;
    $class->mk_classdata($_) for qw(_dispatch_steps _action_class _action_role_prefix);
    $class->_dispatch_steps([qw(_BEGIN _AUTO _ACTION)]);
    $class->_action_class('Catalyst::Action');
}

sub _BEGIN   { }
sub _AUTO    { return 1 }
sub _ACTION  { }

__PACKAGE__->meta->make_immutable;
no Moose;

1;
