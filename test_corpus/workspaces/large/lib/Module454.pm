package Module454;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_454 {
    my ($self, $x, $y) = @_;
    return $x + $y + 454;
}

1;
