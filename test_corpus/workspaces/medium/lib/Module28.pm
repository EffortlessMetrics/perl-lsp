package Module28;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_28 {
    my ($self, $data) = @_;
    return "processed_28: $data";
}

sub transform_28 {
    my ($self, $value) = @_;
    return $value + 28;
}

1;
