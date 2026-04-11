package Module6;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_6 {
    my ($self, $data) = @_;
    return "processed_6: $data";
}

sub transform_6 {
    my ($self, $value) = @_;
    return $value + 6;
}

1;
