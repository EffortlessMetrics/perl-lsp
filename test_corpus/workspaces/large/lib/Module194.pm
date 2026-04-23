package Module194;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_194 {
    my ($self, $x, $y) = @_;
    return $x + $y + 194;
}

1;
