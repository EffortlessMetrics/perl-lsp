package Module15;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_15 {
    my ($self, $data) = @_;
    return "processed_15: $data";
}

sub transform_15 {
    my ($self, $value) = @_;
    return $value + 15;
}

1;
