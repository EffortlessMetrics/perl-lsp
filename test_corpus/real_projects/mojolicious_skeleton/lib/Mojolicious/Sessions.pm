# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Sessions;
use Mojo::Base -base, -signatures;

use Mojo::JSON qw(encode_json decode_json);
use Mojo::Util qw(b64_decode b64_encode);

has cookie_domain      => undef;
has cookie_name        => 'mojolicious';
has cookie_path        => '/';
has default_expiration => 3600;
has samesite           => 'Lax';
has secure             => 0;

sub load {
    my ($self, $c) = @_;
    return unless my $value = $c->req->cookie($self->cookie_name);
    my $session = eval { decode_json b64_decode $value->value } // return;
    return unless ref $session eq 'HASH';
    return if ($session->{expires} // 1) < time;
    $c->stash->{'mojo.session'} = $session;
}

sub store {
    my ($self, $c) = @_;
    my $session = $c->stash->{'mojo.session'} // return;
    my $value   = b64_encode encode_json($session), '';
    $c->res->cookies(
        Mojo::Cookie::Response->new(
            name     => $self->cookie_name,
            value    => $value,
            path     => $self->cookie_path,
            secure   => $self->secure,
            samesite => $self->samesite,
        )
    );
}

1;
