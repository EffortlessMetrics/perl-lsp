package Module839;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_839 {
    my ($self, $x, $y) = @_;
    return $x + $y + 839;
}

1;
