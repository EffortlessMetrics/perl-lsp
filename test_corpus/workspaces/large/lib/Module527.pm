package Module527;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_527 {
    my ($self, $x, $y) = @_;
    return $x + $y + 527;
}

1;
