# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Static;
use Mojo::Base -base, -signatures;

use Mojo::Date;
use Mojo::Util qw(md5_sum);

has classes => sub { ['main'] };
has extra   => sub { {} };
has paths   => sub { [] };
has types   => sub { Mojolicious::Types->new };

sub dispatch {
    my ($self, $c) = @_;
    my $path = $c->req->url->path->to_string;
    return $self->serve($c, $path);
}

sub file {
    my ($self, $path) = @_;
    for my $dir (@{$self->paths}) {
        my $full = File::Spec->catfile($dir, split('/', $path));
        return Mojo::File->new($full) if -f $full;
    }
    return undef;
}

sub serve {
    my ($self, $c, $path) = @_;
    my $asset = $self->file($path);
    return undef unless $asset;
    $c->res->code(200);
    return 1;
}

sub serve_asset {
    my ($self, $c, $asset) = @_;
    $c->res->headers->content_type($self->types->type(
        (split(/\./, $asset->path))[-1]
    ) // 'application/octet-stream');
    return $c->res->content->asset($asset);
}

1;
