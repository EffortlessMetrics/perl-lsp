package Module426;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_426 {
    my ($self, $x, $y) = @_;
    return $x + $y + 426;
}

1;
