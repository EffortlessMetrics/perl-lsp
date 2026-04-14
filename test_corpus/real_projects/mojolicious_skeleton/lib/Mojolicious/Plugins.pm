# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Plugins;
use Mojo::Base 'Mojo::EventEmitter', -signatures;

has namespaces => sub { ['Mojolicious::Plugin'] };

sub emit_hook {
    my ($self, $name) = (shift, shift);
    return $self->emit($name, @_);
}

sub emit_chain {
    my ($self, $name, $c) = @_;
    my $wrapper;
    for my $cb (reverse @{$self->subscribers($name)}) {
        my $next = $wrapper;
        $wrapper = sub { $cb->($next // sub {}, $c) };
    }
    $wrapper ? $wrapper->() : ();
}

sub load_plugin {
    my ($self, $app, $name, @args) = @_;
    my $class = $name =~ /::/ ? $name : $self->_class($name);
    eval "require $class; 1" or Carp::croak "Cannot load plugin '$class': $@";
    return $class->new->register($app, @args);
}

sub _class {
    my ($self, $name) = @_;
    $name =~ s/([a-z])([A-Z])/${1}_${2}/g;
    $name = ucfirst lc $name;
    $name =~ s/_([a-z])/\u$1/g;
    return join('::', @{$self->namespaces}, $name);
}

1;
