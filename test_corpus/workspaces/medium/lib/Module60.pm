package Module60;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_60 {
    my ($self, $data) = @_;
    return "processed_60: $data";
}

sub transform_60 {
    my ($self, $value) = @_;
    return $value + 60;
}

1;
