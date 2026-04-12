package Module4;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_4 {
    my ($self, $x, $y) = @_;
    return $x + $y + 4;
}

1;
