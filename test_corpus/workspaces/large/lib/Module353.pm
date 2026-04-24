package Module353;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_353 {
    my ($self, $x, $y) = @_;
    return $x + $y + 353;
}

1;
