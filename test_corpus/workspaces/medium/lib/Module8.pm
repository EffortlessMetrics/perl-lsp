package Module8;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_8 {
    my ($self, $data) = @_;
    return "processed_8: $data";
}

sub transform_8 {
    my ($self, $value) = @_;
    return $value + 8;
}

1;
