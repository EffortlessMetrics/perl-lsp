package Module3;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_3 {
    my ($self, $data) = @_;
    return "processed_3: $data";
}

sub transform_3 {
    my ($self, $value) = @_;
    return $value + 3;
}

1;
