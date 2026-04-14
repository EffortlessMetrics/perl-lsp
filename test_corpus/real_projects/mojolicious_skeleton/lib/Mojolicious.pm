# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
# This file is a trimmed fixture for LSP latency benchmarking only.
package Mojolicious;
use Mojo::Base -base, -signatures;

use Carp           qw(croak);
use Mojolicious::Commands;
use Mojolicious::Controller;
use Mojolicious::Log;
use Mojolicious::Plugins;
use Mojolicious::Renderer;
use Mojolicious::Routes;
use Mojolicious::Sessions;
use Mojolicious::Static;
use Mojolicious::Types;

our $VERSION = '9.34';

has commands  => sub { Mojolicious::Commands->new(app => shift) };
has controller_class => 'Mojolicious::Controller';
has log      => sub { Mojolicious::Log->new };
has plugins  => sub { Mojolicious::Plugins->new };
has renderer => sub { Mojolicious::Renderer->new };
has routes   => sub { Mojolicious::Routes->new };
has sessions => sub { Mojolicious::Sessions->new };
has static   => sub { Mojolicious::Static->new };
has types    => sub { Mojolicious::Types->new };
has mode     => sub { $ENV{MOJO_MODE} || $ENV{PLACK_ENV} || 'development' };

sub new {
    my ($class, @args) = @_;
    my $self = $class->SUPER::new(@args);
    $self->startup;
    return $self;
}

sub build_tx {
    my $self = shift;
    return Mojo::Transaction::HTTP->new;
}

sub defaults {
    my ($self, %hash) = @_;
    $self->{defaults} //= {};
    if (%hash) {
        %{$self->{defaults}} = (%{$self->{defaults}}, %hash);
        return $self;
    }
    return $self->{defaults};
}

sub dispatch {
    my ($self, $c) = @_;
    my $plugins = $self->plugins;
    $plugins->emit_hook(before_dispatch => $c);
    $self->static->dispatch($c) unless $c->res->code;
    $plugins->emit_hook(before_routes => $c);
    $self->routes->dispatch($c) unless $c->res->code;
}

sub handler {
    my ($self, $tx) = @_;
    my $c = $self->build_controller($tx);
    weaken $c->{app};
    $self->dispatch($c);
    return $c->finish;
}

sub helper {
    my ($self, $name, $cb) = @_;
    croak qq{Helper "$name" already exists} if $self->renderer->get_helper($name);
    $self->renderer->add_helper($name => $cb);
    return $self;
}

sub hook {
    my ($self, $name, $cb) = @_;
    $self->plugins->on($name => $cb);
    return $self;
}

sub plugin {
    my ($self, $name, @args) = @_;
    return $self->plugins->load_plugin($self, $name, @args);
}

sub start {
    my ($class, @args) = @_;
    return $class->new->commands->run(@args);
}

sub startup { }

1;
