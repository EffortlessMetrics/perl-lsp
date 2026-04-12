package Module61;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_61 {
    my ($self, $data) = @_;
    return "processed_61: $data";
}

sub transform_61 {
    my ($self, $value) = @_;
    return $value + 61;
}

1;
