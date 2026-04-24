package Module62;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_62 {
    my ($self, $data) = @_;
    return "processed_62: $data";
}

sub transform_62 {
    my ($self, $value) = @_;
    return $value + 62;
}

1;
