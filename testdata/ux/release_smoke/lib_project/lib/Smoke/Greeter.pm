package Smoke::Greeter;

use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless { prefix => $args{prefix} // 'hello' }, $class;
}

sub greet {
    my ($self, $name) = @_;
    return "$self->{prefix} $name";
}

1;
