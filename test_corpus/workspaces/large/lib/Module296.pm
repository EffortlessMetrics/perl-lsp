package Module296;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_296 {
    my ($self, $x, $y) = @_;
    return $x + $y + 296;
}

1;
