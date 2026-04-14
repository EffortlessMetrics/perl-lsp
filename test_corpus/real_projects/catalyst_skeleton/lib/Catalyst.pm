# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
# This file is a trimmed fixture for LSP latency benchmarking only.
package Catalyst;
use Moose;
use Carp qw(croak carp);
use Scalar::Util qw(blessed weaken isweak reftype);
use POSIX ();

our $VERSION = '5.90130';

use Catalyst::Exception;
use Catalyst::Request;
use Catalyst::Response;
use Catalyst::Utils;
use Catalyst::Controller;

with 'MooseX::Emulate::Class::Accessor::Fast';

has _request_class         => (is => 'rw', default => 'Catalyst::Request');
has _response_class        => (is => 'rw', default => 'Catalyst::Response');
has _components            => (is => 'rw', default => sub { {} });
has _config                => (is => 'rw', default => sub { {} });
has state                  => (is => 'rw', default => 0);
has stash                  => (is => 'rw', default => sub { {} });
has action                 => (is => 'rw');
has namespace              => (is => 'rw', default => '');
has stats                  => (is => 'rw');
has _log                   => (is => 'rw');

sub import {
    my ($class, @args) = @_;
    my $caller = caller;
    return if $caller eq 'main';

    {
        no strict 'refs';
        push @{"${caller}::ISA"}, $class;
        *{"${caller}::meta"} = sub { Moose::Meta::Class->initialize($caller) };
    }
}

sub new {
    my ($class, %args) = @_;
    my $self = $class->SUPER::new(%args);
    $self->_setup;
    return $self;
}

sub _setup {
    my $self = shift;
    $self->_setup_log;
    $self->_setup_plugins;
    $self->_setup_components;
}

sub _setup_log {
    my $self = shift;
    require Catalyst::Log;
    $self->_log(Catalyst::Log->new);
}

sub _setup_plugins { }
sub _setup_components { }

sub req {
    my $self = shift;
    $self->request(@_);
}

sub res {
    my $self = shift;
    $self->response(@_);
}

has request => (
    is      => 'rw',
    lazy    => 1,
    builder => '_build_request',
);

has response => (
    is      => 'rw',
    lazy    => 1,
    builder => '_build_response',
);

sub _build_request {
    my $self = shift;
    return $self->_request_class->new;
}

sub _build_response {
    my $self = shift;
    return $self->_response_class->new;
}

sub forward {
    my $self = shift;
    return $self->dispatcher->forward($self, @_);
}

sub detach {
    my $self = shift;
    $self->dispatcher->detach($self, @_);
}

sub go {
    my $self = shift;
    $self->dispatcher->go($self, @_);
}

sub visit {
    my $self = shift;
    $self->dispatcher->visit($self, @_);
}

sub error {
    my $self = shift;
    if (@_) {
        my $error = ref $_[0] eq 'ARRAY' ? $_[0] : [@_];
        push @{$self->{error}}, @$error;
        return $self;
    }
    return $self->{error} // [];
}

sub clear_errors {
    my $self = shift;
    delete $self->{error};
}

sub log { $_[0]->_log }

sub config {
    my ($self, %args) = @_;
    if (%args) {
        $self->_config({ %{$self->_config}, %args });
        return $self;
    }
    return $self->_config;
}

sub component {
    my ($self, $name) = @_;
    return $self->_components->{$name};
}

sub controller {
    my ($self, $name) = @_;
    return $self->component("${name}::Controller::$name")
        // $self->component("${name}::$name")
        // $self->component($name);
}

sub model {
    my ($self, $name) = @_;
    return $self->component("${name}::Model::$name")
        // $self->component($name);
}

sub view {
    my ($self, $name) = @_;
    return $self->component("${name}::View::$name")
        // $self->component($name);
}

sub uri_for {
    my ($self, $path, @args) = @_;
    require URI;
    my $uri = URI->new($path);
    return $uri;
}

sub handle_request {
    my ($class, $request) = @_;
    my $c = $class->new;
    $c->request($request);
    $c->dispatch;
    return $c;
}

sub dispatch {
    my $self = shift;
    # Dispatch the request to the appropriate action
}

sub setup {
    my ($class, @args) = @_;
    # Class-level setup
}

__PACKAGE__->meta->make_immutable(inline_constructor => 0);
no Moose;

1;
