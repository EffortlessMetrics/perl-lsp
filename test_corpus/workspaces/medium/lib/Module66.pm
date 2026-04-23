package Module66;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_66 {
    my ($self, $data) = @_;
    return "processed_66: $data";
}

sub transform_66 {
    my ($self, $value) = @_;
    return $value + 66;
}

1;
