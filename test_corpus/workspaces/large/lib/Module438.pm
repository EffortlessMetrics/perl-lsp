package Module438;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_438 {
    my ($self, $x, $y) = @_;
    return $x + $y + 438;
}

1;
