# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojo::EventEmitter;
use Mojo::Base -base, -signatures;

use Scalar::Util qw(blessed weaken);

sub catch {
    my ($self, $cb) = @_;
    return $self->on(error => $cb);
}

sub emit {
    my ($self, $name, @args) = @_;
    if (my $s = $self->{events}{$name}) {
        for my $cb (@$s) { $self->$cb(@args) }
    }
    return $self;
}

sub has_subscribers { !!@{$_[0]->subscribers($_[1])} }

sub on {
    my ($self, $name, $cb) = @_;
    push @{$self->{events}{$name}}, $cb;
    return $cb;
}

sub once {
    my ($self, $name, $cb) = @_;
    my $wrapper;
    $wrapper = sub {
        $self->unsubscribe($name => $wrapper);
        $cb->($self, @_);
    };
    weaken $wrapper;
    return $self->on($name => $wrapper);
}

sub subscribers { $_[0]{events}{$_[1]} // [] }

sub unsubscribe {
    my ($self, $name, $cb) = @_;
    if ($cb) {
        $self->{events}{$name} = [grep { $_ ne $cb } @{$self->subscribers($name)}];
        delete $self->{events}{$name} unless @{$self->{events}{$name}};
    }
    else { delete $self->{events}{$name} }
    return $self;
}

1;
