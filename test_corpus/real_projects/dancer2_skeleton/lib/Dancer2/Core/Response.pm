# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
# Original copyright: Alexis Sukrieh, Sawyer X and contributors
package Dancer2::Core::Response;
use Moo;

has status  => (is => 'rw', default => sub { 200 });
has headers => (is => 'rw', default => sub { [] });
has content => (is => 'rw', default => sub { '' });
has encoded => (is => 'rw', default => sub { 0 });

sub header {
    my ($self, $name, $value) = @_;
    if (defined $value) {
        push @{$self->headers}, $name, $value;
        return $self;
    }
    my @hdrs = @{$self->headers};
    while (my ($k, $v) = splice @hdrs, 0, 2) {
        return $v if lc $k eq lc $name;
    }
    return undef;
}

sub to_psgi {
    my $self = shift;
    return [
        $self->status,
        $self->headers,
        [$self->content],
    ];
}

sub is_forwarded { $_[0]->{forwarded} }
sub is_halted    { $_[0]->{halted} }

sub halt {
    my ($self, $content) = @_;
    $self->content($content) if defined $content;
    $self->{halted} = 1;
    return $self;
}

sub redirect {
    my ($self, $url, $status) = @_;
    $self->status($status // 302);
    $self->header('Location', $url);
}

1;
