package Module26;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_26 {
    my ($self, $x, $y) = @_;
    return $x + $y + 26;
}

1;
