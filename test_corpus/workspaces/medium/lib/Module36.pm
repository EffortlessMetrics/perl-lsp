package Module36;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_36 {
    my ($self, $data) = @_;
    return "processed_36: $data";
}

sub transform_36 {
    my ($self, $value) = @_;
    return $value + 36;
}

1;
