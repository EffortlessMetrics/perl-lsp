package Module618;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_618 {
    my ($self, $x, $y) = @_;
    return $x + $y + 618;
}

1;
