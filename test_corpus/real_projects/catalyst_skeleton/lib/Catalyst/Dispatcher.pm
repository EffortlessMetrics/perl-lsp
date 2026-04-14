# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Dispatcher;
use Moose;
use Carp 'croak';

has _action_hash    => (is => 'rw', default => sub { {} });
has _container      => (is => 'rw');
has dispatch_types  => (is => 'rw', default => sub { [] });

sub dispatch {
    my ($self, $c) = @_;
    my $path   = $c->req->path;
    my $method = $c->req->method;
    my $action = $self->_find_action($c, $path);
    unless ($action) {
        $c->res->status(404);
        $c->res->body('Not Found');
        return;
    }
    $action->dispatch($c);
}

sub forward {
    my ($self, $c, $action_or_url, @args) = @_;
    if (ref $action_or_url && $action_or_url->isa('Catalyst::Action')) {
        return $action_or_url->dispatch($c);
    }
    my $action = $self->_find_action($c, $action_or_url);
    croak "Unknown action '$action_or_url'" unless $action;
    return $action->dispatch($c);
}

sub detach {
    my ($self, $c, $action_or_url, @args) = @_;
    $self->forward($c, $action_or_url, @args);
    die Catalyst::Exception::Detach->new;
}

sub go {
    my ($self, $c, $action_or_url, @args) = @_;
    $self->forward($c, $action_or_url, @args);
    die Catalyst::Exception::Go->new;
}

sub visit {
    my ($self, $c, $action_or_url, @args) = @_;
    $self->forward($c, $action_or_url, @args);
}

sub get_action {
    my ($self, $name, $namespace) = @_;
    $namespace //= '';
    return $self->_action_hash->{"${namespace}/$name"};
}

sub _find_action {
    my ($self, $c, $path) = @_;
    return $self->_action_hash->{$path};
}

sub register {
    my ($self, $c, $action) = @_;
    my $key = $action->namespace . '/' . $action->name;
    $self->_action_hash->{$key} = $action;
}

__PACKAGE__->meta->make_immutable;
no Moose;

1;
