package Module579;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_579 {
    my ($self, $x, $y) = @_;
    return $x + $y + 579;
}

1;
