# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Response;
use Moose;
use HTTP::Headers;

has _body       => (is => 'rw', default => '');
has cookies     => (is => 'rw', default => sub { {} });
has headers     => (is => 'rw', default => sub { HTTP::Headers->new(
    'Content-Type' => 'text/html; charset=utf-8'
)});
has status      => (is => 'rw', default => 200);
has finalized_headers => (is => 'rw', default => 0);

sub body {
    my ($self, $body) = @_;
    if (defined $body) {
        $self->_body($body);
        return $self;
    }
    return $self->_body;
}

sub content_type {
    my ($self, $type) = @_;
    if (defined $type) {
        $self->headers->content_type($type);
        return $self;
    }
    return $self->headers->content_type;
}

sub content_length {
    my $self = shift;
    return $self->headers->content_length;
}

sub header {
    my ($self, $name, $value) = @_;
    if (defined $value) {
        $self->headers->header($name, $value);
        return $self;
    }
    return $self->headers->header($name);
}

sub output { $_[0]->body }

sub redirect {
    my ($self, $url, $status) = @_;
    $self->status($status // 302);
    $self->headers->header('Location', $url);
    return $self;
}

sub write {
    my ($self, $buffer) = @_;
    $self->_body($self->_body . ($buffer // ''));
}

sub finalize_headers {
    my $self = shift;
    $self->finalized_headers(1);
}

__PACKAGE__->meta->make_immutable;
no Moose;

1;
