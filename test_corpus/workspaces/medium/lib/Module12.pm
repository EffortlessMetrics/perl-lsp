package Module12;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_12 {
    my ($self, $data) = @_;
    return "processed_12: $data";
}

sub transform_12 {
    my ($self, $value) = @_;
    return $value + 12;
}

1;
