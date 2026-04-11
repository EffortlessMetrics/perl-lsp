package Module720;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_720 {
    my ($self, $x, $y) = @_;
    return $x + $y + 720;
}

1;
