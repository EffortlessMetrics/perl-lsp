package Module78;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_78 {
    my ($self, $data) = @_;
    return "processed_78: $data";
}

sub transform_78 {
    my ($self, $value) = @_;
    return $value + 78;
}

1;
