# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Exception;
use Moose;
use Carp        qw(croak);
use Scalar::Util qw(blessed);
use overload '""' => \&as_string, fallback => 1;

has message => (is => 'rw', required => 1, default => 'Unknown error');
has code    => (is => 'rw', default  => 500);

sub throw {
    my ($class, %args) = @_;
    croak blessed($class) ? $class : $class->new(%args);
}

sub as_string {
    my $self = shift;
    return ref($self) . ': ' . $self->message;
}

sub rethrow {
    my ($self) = @_;
    die $self;
}

around BUILDARGS => sub {
    my ($orig, $class, @args) = @_;
    if (@args == 1 && !ref $args[0]) {
        return $class->$orig(message => $args[0]);
    }
    return $class->$orig(@args);
};

__PACKAGE__->meta->make_immutable;
no Moose;

1;
