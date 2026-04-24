package Module801;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_801 {
    my ($self, $x, $y) = @_;
    return $x + $y + 801;
}

1;
