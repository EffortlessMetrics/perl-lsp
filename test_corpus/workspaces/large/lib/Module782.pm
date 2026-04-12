package Module782;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_782 {
    my ($self, $x, $y) = @_;
    return $x + $y + 782;
}

1;
