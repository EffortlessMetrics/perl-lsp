# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Action;
use Moose;
use Carp 'croak';
use Scalar::Util qw(blessed);

has class     => (is => 'rw');
has namespace => (is => 'rw', default => '');
has name      => (is => 'rw');
has code      => (is => 'rw');
has reverse   => (is => 'rw', default => '');
has attributes => (is => 'rw', default => sub { {} });
has number_of_args        => (is => 'rw');
has number_of_captures    => (is => 'rw', default => 0);
has args_constraints      => (is => 'rw');
has captures_constraints  => (is => 'rw');

sub dispatch {
    my ($self, $c) = @_;
    my $class = $self->class;
    my $controller = $c->component($class);
    croak "No controller found for '$class'" unless $controller;
    local $c->{namespace} = $self->namespace;
    return $controller->${\$self->name}($c, @{$c->req->args});
}

sub execute {
    my ($self, $c, @args) = @_;
    return $self->code->($c->component($self->class), $c, @args);
}

sub match {
    my ($self, $c) = @_;
    return 1 unless defined $self->number_of_args;
    return scalar(@{$c->req->args}) == $self->number_of_args;
}

sub match_captures {
    my ($self, $c, $captures) = @_;
    return 1;
}

sub compare {
    my ($a, $b) = @_;
    return Catalyst::Utils::compare_action_args($a, $b);
}

sub private_path {
    my $self = shift;
    return '/' . $self->reverse;
}

__PACKAGE__->meta->make_immutable;
no Moose;

1;
