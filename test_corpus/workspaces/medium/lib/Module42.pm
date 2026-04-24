package Module42;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_42 {
    my ($self, $data) = @_;
    return "processed_42: $data";
}

sub transform_42 {
    my ($self, $value) = @_;
    return $value + 42;
}

1;
