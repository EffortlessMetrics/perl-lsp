package Module40;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_40 {
    my ($self, $data) = @_;
    return "processed_40: $data";
}

sub transform_40 {
    my ($self, $value) = @_;
    return $value + 40;
}

1;
