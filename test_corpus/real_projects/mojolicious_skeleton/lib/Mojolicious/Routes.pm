# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Routes;
use Mojo::Base 'Mojo::EventEmitter', -signatures;

use Mojo::Util   qw(decode encode);
use Scalar::Util qw(weaken);

has base_classes => sub { ['Mojolicious::Controller'] };
has cache        => sub { Mojo::Cache->new };
has conditions   => sub { {} };
has hidden       => sub { [qw(new app attr has import with)] };
has namespaces   => sub { [] };

sub add_condition {
    my ($self, $name, $cb) = @_;
    $self->conditions->{$name} = $cb;
    return $self;
}

sub add_shortcut {
    my ($self, $name, $cb) = @_;
    no strict 'refs';
    *{"Mojolicious::Routes::Route::$name"} = sub { $cb->(@_) };
    return $self;
}

sub any {
    my $self = shift;
    return $self->_add_route(methods => [], @_);
}

sub continue {
    my ($self, $c) = @_;
    my $route = $c->match->current;
    return $self->_dispatch($c, $route) if $route;
    my $stack = $c->match->stack;
    my $index = $c->stash->{'mojo.index'}++ // 0;
    return 1 unless my $field = $stack->[$index];
    return 0 unless my $next = $self->lookup($field->{action});
    return $self->_dispatch($c, $next);
}

sub dispatch {
    my ($self, $c) = @_;
    my $path = $c->req->url->path->to_route;
    my $match = $self->match($c->req, $path);
    return unless $match->endpoint;
    $c->match($match);
    $self->continue($c);
}

sub find {
    my ($self, $name) = @_;
    return $self->_find($name, {});
}

sub get {
    my $self = shift;
    return $self->_add_route(methods => ['GET'], @_);
}

sub lookup {
    my ($self, $name) = @_;
    my $cache = $self->cache;
    return $cache->get($name) if $cache->get($name);
    return undef;
}

sub match {
    my ($self, $req, $path) = @_;
    my $match = Mojo::Routes::Match->new(root => $self);
    $match->find($req, {path => $path});
    return $match;
}

sub post {
    my $self = shift;
    return $self->_add_route(methods => ['POST'], @_);
}

sub put {
    my $self = shift;
    return $self->_add_route(methods => ['PUT'], @_);
}

sub delete {
    my $self = shift;
    return $self->_add_route(methods => ['DELETE'], @_);
}

sub under {
    my $self = shift;
    return $self->_add_route(inline => 1, @_);
}

sub websocket {
    my $self = shift;
    return $self->_add_route(websocket => 1, @_);
}

sub _add_route {
    my ($self, %args) = @_;
    my $route = Mojolicious::Routes::Route->new(%args, parent => $self);
    push @{$self->{routes}}, $route;
    return $route;
}

sub _dispatch {
    my ($self, $c, $route) = @_;
    return $route->dispatcher->($c, $route);
}

sub _find {
    my ($self, $name, $seen) = @_;
    return undef if $seen->{$name}++;
    for my $route (@{$self->{routes} // []}) {
        return $route if ($route->name // '') eq $name;
    }
    return undef;
}

1;
