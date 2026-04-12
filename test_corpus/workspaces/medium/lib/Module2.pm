package Module2;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_2 {
    my ($self, $data) = @_;
    return "processed_2: $data";
}

sub transform_2 {
    my ($self, $value) = @_;
    return $value + 2;
}

1;
