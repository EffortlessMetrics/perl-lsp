# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Controller;
use Mojo::Base -base, -signatures;

use Carp          qw(croak);
use Mojo::Promise qw();
use Scalar::Util  qw(blessed weaken);

has app => undef, weak => 1;
has match => sub { Mojo::Routes::Match->new(root => Mojolicious::Routes->new) };
has req  => sub { Mojo::Message::Request->new };
has res  => sub { Mojo::Message::Response->new };
has stash => sub { {} };
has tx  => undef, weak => 1;

sub continue {
    my $self = shift;
    return $self->app->routes->continue($self);
}

sub finish {
    my ($self, @args) = @_;
    my $tx = $self->tx;
    return $self unless $tx;
    $tx->resume unless $tx->is_websocket;
    return $self->rendered(@args) if @args;
    return $self;
}

sub helpers { $_[0]->app->renderer->get_helper }

sub on {
    my ($self, $name, $cb) = @_;
    my $tx = $self->tx;
    weaken $self;
    return $tx->on($name => sub { $self && $self->$cb(@_[1..$#_]) });
}

sub param {
    my ($self, $name) = (shift, shift);
    my $captures = $self->stash->{'mojo.captures'} //= {};
    unless (@_) {
        my $params = $self->req->params->to_hash;
        if (defined(my $value = $captures->{$name} // $params->{$name})) {
            return ref $value eq 'ARRAY' ? wantarray ? @$value : $$value[-1] : $value;
        }
        return undef;
    }
    $captures->{$name} = @_ > 1 ? [@_] : $_[0];
    return $self;
}

sub redirect_to {
    my ($self, $target, @args) = @_;
    $self->res->headers->location($self->url_for($target, @args));
    return $self->rendered(302);
}

sub render {
    my ($self, @args) = @_;
    my ($output, $format) = $self->app->renderer->render($self, @args);
    return $self->rendered(200) if defined $output;
    return undef;
}

sub rendered {
    my ($self, $status) = @_;
    $self->res->code($status) if $status;
    return $self->finish;
}

sub url_for {
    my ($self, $target, @args) = @_;
    return $self->req->url->clone->path($target) if $target =~ m!^/!;
    return Mojo::URL->new($target) if $target =~ m!^[a-z][a-z0-9\+\-\.]*:!i;
    my $route = $self->app->routes->find($target);
    croak qq{Unknown name "$target" for route} unless $route;
    return $route->render($self->stash, @args);
}

1;
