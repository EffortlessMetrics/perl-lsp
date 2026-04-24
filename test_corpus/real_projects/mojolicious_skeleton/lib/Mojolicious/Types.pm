# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojolicious::Types;
use Mojo::Base -base, -signatures;

has mapping => sub {
    {
        bin  => ['application/octet-stream'],
        css  => ['text/css'],
        gif  => ['image/gif'],
        gz   => ['application/gzip'],
        htm  => ['text/html'],
        html => ['text/html;charset=UTF-8'],
        ico  => ['image/x-icon'],
        jpeg => ['image/jpeg'],
        jpg  => ['image/jpeg'],
        js   => ['application/javascript'],
        json => ['application/json;charset=UTF-8'],
        mp3  => ['audio/mpeg'],
        mp4  => ['video/mp4'],
        ogg  => ['audio/ogg'],
        pdf  => ['application/pdf'],
        png  => ['image/png'],
        svg  => ['image/svg+xml'],
        txt  => ['text/plain;charset=UTF-8'],
        webm => ['video/webm'],
        webp => ['image/webp'],
        xml  => ['application/xml;charset=UTF-8'],
        zip  => ['application/zip'],
    }
};

sub detect {
    my ($self, $accept, $prioritize) = @_;
    $accept //= '';
    my @detected;
    my $mapping = $self->mapping;
    for my $type (split(/\s*,\s*/, $accept)) {
        $type =~ s/\s*;[^,]+//g;
        for my $ext (sort keys %$mapping) {
            push @detected, $ext if grep { $_ eq $type } @{$mapping->{$ext}};
        }
    }
    return $prioritize ? [reverse @detected] : \@detected;
}

sub type {
    my ($self, $ext) = @_;
    return undef unless $ext;
    return (${$self->mapping}{lc $ext} // [])->[0];
}

1;
