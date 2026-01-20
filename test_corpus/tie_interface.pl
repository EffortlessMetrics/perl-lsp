use strict;
use warnings;

tie my %hash, "DB_File", "file.db", 0, 0666;
tie my @array, "Tie::Array";
tie my $scalar, "Tie::Scalar";
tie *FH, "Tie::Handle";

my $obj = tie my %cache, "Tie::StdHash";
$cache{a} = 1;

untie %hash;

package MyTie;
use parent "Tie::Hash";

sub TIEHASH {
    my ($class, $filename) = @_;
    my $self = {};
    bless $self, $class;
    return $self;
}

sub FETCH {
    my ($self, $key) = @_;
    return $self->{$key};
}

sub STORE {
    my ($self, $key, $value) = @_;
    $self->{$key} = $value;
}

package main;
tie my %myhash, "MyTie";
