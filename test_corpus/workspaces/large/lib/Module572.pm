package Module572;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_572 {
    my ($self, $x, $y) = @_;
    return $x + $y + 572;
}

1;
