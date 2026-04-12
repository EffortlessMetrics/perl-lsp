package Module50;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_50 {
    my ($self, $x, $y) = @_;
    return $x + $y + 50;
}

1;
