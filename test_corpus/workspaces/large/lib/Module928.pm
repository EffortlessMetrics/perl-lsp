package Module928;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_928 {
    my ($self, $x, $y) = @_;
    return $x + $y + 928;
}

1;
