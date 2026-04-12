package Module27;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_27 {
    my ($self, $data) = @_;
    return "processed_27: $data";
}

sub transform_27 {
    my ($self, $value) = @_;
    return $value + 27;
}

1;
