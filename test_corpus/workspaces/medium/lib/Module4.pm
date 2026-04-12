package Module4;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_4 {
    my ($self, $data) = @_;
    return "processed_4: $data";
}

sub transform_4 {
    my ($self, $value) = @_;
    return $value + 4;
}

1;
