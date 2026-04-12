package Module910;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_910 {
    my ($self, $x, $y) = @_;
    return $x + $y + 910;
}

1;
