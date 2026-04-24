package Module177;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_177 {
    my ($self, $x, $y) = @_;
    return $x + $y + 177;
}

1;
