package Module30;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_30 {
    my ($self, $data) = @_;
    return "processed_30: $data";
}

sub transform_30 {
    my ($self, $value) = @_;
    return $value + 30;
}

1;
