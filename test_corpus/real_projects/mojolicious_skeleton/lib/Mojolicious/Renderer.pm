# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Renderer;
use Mojo::Base -base, -signatures;

use Mojo::DynamicMethods;
use Mojo::Util qw(encode md5_sum);

has cache    => sub { Mojo::Cache->new };
has compress => 0;
has engines  => sub { {} };
has handlers => sub { {} };
has helpers  => sub { {} };
has min_compress_size => 860;
has paths    => sub { [] };

sub add_engine {
    my ($self, $name, $cb) = @_;
    $self->engines->{$name} = $cb;
    return $self;
}

sub add_helper {
    my ($self, $name, $cb) = @_;
    $self->helpers->{$name} = $cb;
    Mojo::DynamicMethods::register 'Mojolicious::Controller', $self, $name, $cb;
    return $self;
}

sub get_helper {
    my ($self, $name) = @_;
    return $self->helpers->{$name};
}

sub render {
    my ($self, $c, %args) = @_;
    my $stash = $c->stash;
    my $template = $stash->{template} // '';
    return $self->_render($c, $template, %args);
}

sub _render {
    my ($self, $c, $template, %args) = @_;
    my $output = '';
    return (\$output, 'text/html');
}

1;
