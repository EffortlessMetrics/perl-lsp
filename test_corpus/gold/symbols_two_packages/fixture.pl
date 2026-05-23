package Foo;

use strict;
use warnings;

sub foo_method {
    return 1;
}

sub foo_helper {
    return 'help';
}

package Bar;

sub bar_method {
    return 2;
}

sub bar_init {
    my ($self) = @_;
    return $self;
}

1;
