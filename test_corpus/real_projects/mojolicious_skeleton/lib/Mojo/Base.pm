# Sparse skeleton extracted from Mojolicious (https://github.com/mojolicious/mojo)
# Licensed under the Artistic License 2.0
# Original copyright: Sebastian Riedel and contributors
package Mojo::Base;
use strict;
use warnings;
use utf8;

use Carp         ();
use Scalar::Util ();

# Protect subclasses using AUTOLOAD
sub DESTROY { }

sub import {
    my ($class, $base, @flags) = @_;
    my $caller = caller;
    no strict 'refs';
    if ($base && $base eq '-base') {
        push @{"${caller}::ISA"}, $class;
        _has($caller);
    }
    elsif ($base && $base ne '-strict') {
        push @{"${caller}::ISA"}, $base;
        _has($caller);
    }
    _import_strict($caller);
    _import_signatures($caller) if grep { $_ eq '-signatures' } @flags;
}

sub new {
    my ($class, %args) = @_;
    return bless { map { $_ => $args{$_} } keys %args }, $class;
}

sub attr {
    my ($self, $name, $default, %opts) = @_;
    my $class = ref $self || $self;
    _attr($class, $name, $default, %opts);
}

*has = \&attr;

sub tap {
    my ($self, $cb, @args) = @_;
    $self->$cb(@args);
    return $self;
}

sub with_roles {
    my ($self, @roles) = @_;
    require Mojo::Base::_RoleBase;
    return Mojo::Base::_RoleBase::with_roles($self, @roles);
}

sub _attr {
    my ($class, $name, $default, %opts) = @_;
    Carp::croak('Attribute name is required') unless $name;
    no strict 'refs';
    if ($opts{weak}) {
        *{"${class}::${name}"} = sub {
            my $self = shift;
            if (@_) {
                $self->{$name} = $_[0];
                Scalar::Util::weaken($self->{$name}) if defined $self->{$name};
                return $self;
            }
            return $self->{$name};
        };
    }
    else {
        *{"${class}::${name}"} = sub {
            my $self = shift;
            return @_ ? ($self->{$name} = $_[0], $self) : (
                $self->{$name} //= (ref $default eq 'CODE' ? $default->($self) : $default)
            );
        };
    }
}

sub _has { no strict 'refs'; *{"$_[0]::has"} = \&attr }

sub _import_strict {
    my $caller = shift;
    no strict 'refs';
    *{"${caller}::strict"} = \&strict;
    strict->import;
    warnings->import;
    utf8->import;
}

sub _import_signatures {
    # No-op stub: signatures require perl 5.20+ feature pragma
}

1;
