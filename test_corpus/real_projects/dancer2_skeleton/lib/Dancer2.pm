# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
# Original copyright: Alexis Sukrieh, Sawyer X and contributors
# This file is a trimmed fixture for LSP latency benchmarking only.
package Dancer2;
use strict;
use warnings;

our $VERSION = '1.1.1';

use Dancer2::Core::App;
use Dancer2::Core::DSL;
use Dancer2::Core::Runner;
use Carp 'croak';
use Scalar::Util qw(blessed reftype);

my $runner;

sub import {
    my ($class, @args) = @_;
    my $caller = caller;
    my %options = @args;

    my $app = Dancer2::Core::App->new(
        name            => $caller,
        environment     => $options{environment} // $ENV{DANCER_ENVIRONMENT} // 'development',
        location        => $options{location} // _get_location(),
        runner          => _runner(),
    );
    _set_runner_app($app);

    _export_to($caller, $app);
}

sub _runner {
    $runner //= Dancer2::Core::Runner->new;
    return $runner;
}

sub _set_runner_app {
    my ($app) = @_;
    $runner->apps([@{$runner->apps}, $app]);
}

sub _get_location {
    my $loc = $ENV{DANCER_APPDIR};
    unless ($loc) {
        require FindBin;
        $loc = $FindBin::Bin;
    }
    return $loc;
}

sub _export_to {
    my ($caller, $app) = @_;
    my $dsl = Dancer2::Core::DSL->new(app => $app);
    for my $name ($dsl->dsl_keywords) {
        no strict 'refs';
        *{"${caller}::${name}"} = sub { $dsl->$name(@_) };
    }
}

sub start {
    my $class = shift;
    _runner()->start;
}

sub psgi_app {
    my $class = shift;
    return _runner()->psgi_app;
}

1;
