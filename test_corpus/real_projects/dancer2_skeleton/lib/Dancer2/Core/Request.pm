# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
# Original copyright: Alexis Sukrieh, Sawyer X and contributors
package Dancer2::Core::Request;
use Moo;
use Carp 'croak';

has env     => (is => 'ro', required => 1);
has body    => (is => 'lazy');
has params  => (is => 'lazy');
has headers => (is => 'lazy');

sub _build_body {
    my $self = shift;
    my $env  = $self->env;
    my $length = $env->{CONTENT_LENGTH} // 0;
    return '' unless $length;
    my $body = '';
    $env->{'psgi.input'}->read($body, $length);
    return $body;
}

sub _build_params {
    my $self = shift;
    my %params;
    # Parse query string
    my $qs = $self->env->{QUERY_STRING} // '';
    for my $pair (split /&/, $qs) {
        my ($k, $v) = split /=/, $pair, 2;
        next unless defined $k;
        $params{_decode($k)} = defined $v ? _decode($v) : '';
    }
    return \%params;
}

sub _build_headers {
    my $self = shift;
    my %headers;
    for my $key (keys %{$self->env}) {
        next unless $key =~ /^HTTP_(.+)$/;
        my $name = lc $1;
        $name =~ s/_/-/g;
        $headers{$name} = $self->env->{$key};
    }
    return \%headers;
}

sub method      { uc $_[0]->env->{REQUEST_METHOD} }
sub path        { $_[0]->env->{PATH_INFO} }
sub content_type { $_[0]->env->{CONTENT_TYPE} }
sub host        { $_[0]->env->{HTTP_HOST} }

sub param {
    my ($self, $name) = @_;
    return $self->params->{$name};
}

sub header {
    my ($self, $name) = @_;
    return $self->headers->{lc $name};
}

sub is_post    { $_[0]->method eq 'POST' }
sub is_get     { $_[0]->method eq 'GET' }
sub is_put     { $_[0]->method eq 'PUT' }
sub is_delete  { $_[0]->method eq 'DELETE' }

sub _decode {
    my $val = shift;
    $val =~ s/\+/ /g;
    $val =~ s/%([0-9A-Fa-f]{2})/chr hex $1/ge;
    return $val;
}

1;
