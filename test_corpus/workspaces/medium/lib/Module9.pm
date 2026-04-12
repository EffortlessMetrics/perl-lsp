package Module9;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_9 {
    my ($self, $data) = @_;
    return "processed_9: $data";
}

sub transform_9 {
    my ($self, $value) = @_;
    return $value + 9;
}

1;
