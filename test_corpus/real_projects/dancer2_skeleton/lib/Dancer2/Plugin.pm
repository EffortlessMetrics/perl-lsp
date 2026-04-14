# Sparse skeleton extracted from Dancer2 (https://github.com/PerlDancer/Dancer2)
# Licensed under the Artistic License 2.0
# Original copyright: Alexis Sukrieh, Sawyer X and contributors
package Dancer2::Plugin;
use strict;
use warnings;
use Carp 'croak';

sub import {
    my ($class, @args) = @_;
    my $caller = caller;

    no strict 'refs';
    push @{"${caller}::ISA"}, 'Dancer2::Plugin::Base';

    *{"${caller}::plugin_keywords"} = sub {
        my ($plugin_class, @keywords) = @_;
        for my $kw (@keywords) {
            no strict 'refs';
            *{"${plugin_class}::${kw}"} = sub {
                my $self = shift;
                $self->$kw(@_);
            };
        }
    };

    *{"${caller}::register"} = sub {
        my ($plugin_class, $app) = @_;
        $plugin_class->new(app => $app);
    };
}

package Dancer2::Plugin::Base;
use Moo;

has app => (is => 'ro', required => 1, weak_ref => 1);
has config => (is => 'ro', lazy => 1, builder => '_build_config');

sub _build_config {
    my $self = shift;
    return $self->app->config->{plugins}{ref($self)} // {};
}

sub execute_plugin_hook {
    my ($self, $name, @args) = @_;
    $self->app->execute_hook("plugin." . ref($self) . ".$name", @args);
}

1;
