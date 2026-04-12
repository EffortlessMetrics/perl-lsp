package Module1;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_1 {
    my ($self, $data) = @_;
    return "processed_1: $data";
}

sub transform_1 {
    my ($self, $value) = @_;
    return $value + 1;
}

1;
