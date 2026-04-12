package Module96;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_96 {
    my ($self, $x, $y) = @_;
    return $x + $y + 96;
}

1;
