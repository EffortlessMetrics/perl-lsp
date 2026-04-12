package Module41;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_41 {
    my ($self, $data) = @_;
    return "processed_41: $data";
}

sub transform_41 {
    my ($self, $value) = @_;
    return $value + 41;
}

1;
