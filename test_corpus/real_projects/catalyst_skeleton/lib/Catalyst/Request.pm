# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Request;
use Moose;
use Carp 'croak';
use HTTP::Headers;
use URI;

has _env          => (is => 'rw', default => sub { {} });
has action        => (is => 'rw');
has address       => (is => 'rw');
has arguments     => (is => 'rw', default => sub { [] });
has body          => (is => 'rw');
has body_data     => (is => 'rw');
has body_parameters => (is => 'rw', default => sub { {} });
has cookies       => (is => 'rw', default => sub { {} });
has headers       => (is => 'rw', default => sub { HTTP::Headers->new });
has match         => (is => 'rw');
has method        => (is => 'rw', default => 'GET');
has path          => (is => 'rw', default => '/');
has protocol      => (is => 'rw', default => 'HTTP/1.1');
has query_parameters => (is => 'rw', default => sub { {} });
has secure        => (is => 'rw', default => 0);
has uri           => (is => 'rw', lazy => 1, builder => '_build_uri');

sub _build_uri {
    my $self = shift;
    return URI->new('http://localhost' . $self->path);
}

sub param {
    my ($self, $name) = @_;
    my $params = { %{$self->query_parameters}, %{$self->body_parameters} };
    return $params->{$name} unless defined $name;
    return $params;
}

sub params { $_[0]->param }

sub base {
    my $self = shift;
    my $uri = $self->uri->clone;
    $uri->path('/');
    return $uri;
}

sub content_type { $_[0]->headers->content_type }

sub cookie {
    my ($self, $name) = @_;
    return $self->cookies->{$name};
}

sub header {
    my ($self, $name) = @_;
    return $self->headers->header($name);
}

sub user_agent { $_[0]->header('User-Agent') }
sub referer    { $_[0]->header('Referer') }

__PACKAGE__->meta->make_immutable;
no Moose;

1;
