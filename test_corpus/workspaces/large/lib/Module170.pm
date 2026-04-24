package Module170;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_170 {
    my ($self, $x, $y) = @_;
    return $x + $y + 170;
}

1;
