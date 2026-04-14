# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
# Original copyright: Alexis Sukrieh, Sawyer X and contributors
package Dancer2::Core::App;
use Moo;
use Carp 'croak';
use Scalar::Util qw(blessed);

has name        => (is => 'ro', required => 1);
has environment => (is => 'ro', default => sub { 'development' });
has location    => (is => 'ro', default => sub { '.' });
has runner      => (is => 'ro', required => 1, weak_ref => 1);
has routes      => (is => 'ro', default => sub { {} });
has hooks       => (is => 'ro', default => sub { {} });
has config      => (is => 'ro', lazy => 1, builder => '_build_config');

sub _build_config {
    my $self = shift;
    return Dancer2::Core::Config->new(location => $self->location);
}

sub add_route {
    my ($self, %args) = @_;
    my $method  = lc $args{method};
    my $pattern = $args{regexp};
    my $code    = $args{code};
    croak "Route code must be a CODE ref" unless ref $code eq 'CODE';
    push @{$self->routes->{$method}}, {
        regexp => $pattern,
        code   => $code,
    };
}

sub dispatch {
    my ($self, $env) = @_;
    my $method = lc $env->{REQUEST_METHOD};
    my $path   = $env->{PATH_INFO};
    for my $route (@{$self->routes->{$method} // []}) {
        if (my @captures = ($path =~ $route->{regexp})) {
            return $route->{code}->(@captures);
        }
    }
    return undef;
}

sub add_hook {
    my ($self, $name, $code) = @_;
    push @{$self->hooks->{$name}}, $code;
}

sub execute_hook {
    my ($self, $name, @args) = @_;
    for my $cb (@{$self->hooks->{$name} // []}) {
        $cb->(@args);
    }
}

1;
