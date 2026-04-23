package Module80;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_80 {
    my ($self, $data) = @_;
    return "processed_80: $data";
}

sub transform_80 {
    my ($self, $value) = @_;
    return $value + 80;
}

1;
