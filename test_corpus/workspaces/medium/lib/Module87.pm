package Module87;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_87 {
    my ($self, $data) = @_;
    return "processed_87: $data";
}

sub transform_87 {
    my ($self, $value) = @_;
    return $value + 87;
}

1;
