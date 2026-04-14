# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Commands;
use Mojo::Base 'Mojo::EventEmitter', -signatures;

use Mojo::Util qw(tablify);

has app        => undef, weak => 1;
has namespaces => sub { ['Mojolicious::Command'] };

sub detect {
    my ($self, @args) = @_;
    return 'help' unless @args;
    return shift @args if @args && $args[0] =~ /^\w+$/;
    return 'cgi' if $ENV{GATEWAY_INTERFACE};
    return 'psgi' if $ENV{PSGI_APP};
    return 'daemon';
}

sub run {
    my ($self, $name, @args) = @_;
    $name //= $self->detect(@args);
    my $class = $self->_class($name);
    eval "require $class; 1" or die "Cannot load command '$class': $@\n";
    return $class->new(app => $self->app)->run(@args);
}

sub _class {
    my ($self, $name) = @_;
    $name =~ s/::/\//g;
    return join('::', grep { $_ } @{$self->namespaces}, ucfirst $name);
}

1;
