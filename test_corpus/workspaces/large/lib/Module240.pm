package Module240;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_240 {
    my ($self, $x, $y) = @_;
    return $x + $y + 240;
}

1;
