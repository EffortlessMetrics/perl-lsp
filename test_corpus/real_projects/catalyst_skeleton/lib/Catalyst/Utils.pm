# Sparse skeleton extracted from Catalyst (https://github.com/perl-catalyst/catalyst-runtime)
# Licensed under the same terms as Perl itself
# Original copyright: Andy Grundman and contributors
package Catalyst::Utils;
use strict;
use warnings;
use Exporter 'import';
use File::Spec;
use POSIX qw();

our @EXPORT_OK = qw(
    class2appclass class2classprefix class2classsuffix class2env
    class2prefix class2classname class2shortsuffix
    home resolve_namespace
    ensure_class_loaded
    build_query_string
    inject_component
    merge_hashes
    term_width
    env_value
);

sub class2appclass {
    my $class = shift || '';
    my $appname = '';
    if ($class =~ /^(.+?)::([MVC]|Model|View|Controller)::.+$/) {
        $appname = $1;
    }
    return $appname;
}

sub class2classprefix {
    my $class = shift || '';
    my $prefix;
    if ($class =~ /^.+?::([MVC]|Model|View|Controller)::.+$/) {
        $prefix = $1;
    }
    return $prefix;
}

sub class2env {
    my $class = shift || '';
    $class = uc $class;
    $class =~ s/::/_/g;
    return $class;
}

sub class2prefix {
    my $class = shift || '';
    my $prefix;
    if ($class =~ /^.+?::(?:[MVC]|Model|View|Controller)::(.+)$/) {
        $prefix = lc $1;
        $prefix =~ s{::}{/}g;
    }
    return $prefix;
}

sub home {
    my $class = shift;
    my $file  = $class;
    $file =~ s{::}{/}g;
    $file .= '.pm';
    if (my $inc_entry = $INC{$file}) {
        my $home = $inc_entry;
        $home =~ s{lib/\Q$file\E$}{};
        $home = File::Spec->rel2abs($home);
        return $home if -d $home;
    }
    return undef;
}

sub ensure_class_loaded {
    my ($class, $opts) = @_;
    unless (eval { $class->can('new') }) {
        eval "require $class; 1"
            or die "Could not load class '$class': $@";
    }
}

sub merge_hashes {
    my ($base, $over) = @_;
    return { %$base, %$over } if ref $base eq 'HASH' && ref $over eq 'HASH';
    return $over // $base;
}

sub env_value {
    my ($class, $key) = @_;
    $key = class2env($class) . '_' . uc $key;
    return $ENV{$key};
}

sub term_width {
    return 80 unless eval { require Term::Size::Any };
    my ($width) = Term::Size::Any::chars();
    return $width || 80;
}

1;
