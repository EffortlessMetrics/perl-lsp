#!/usr/bin/env perl
# Test file for Signature and Prototype NodeKinds
# This file tests modern Perl subroutine signatures and prototypes

use strict;
use warnings;

# Test basic prototypes
sub proto_array ($) {
    my ($array) = @_;
    return scalar @$array;
}

sub proto_hash (%) {
    my (%hash) = @_;
    return scalar keys %hash;
}

sub proto_code (&) {
    my ($code) = @_;
    return $code->();
}

sub proto_asterisk (*) {
    my ($handle) = @_;
    return <$handle>;
}

# Test prototype with mixed parameters
sub proto_mixed ($$) {
    my ($first, $second) = @_;
    return $first + $second;
}

# Test prototype with optional parameters
sub proto_optional ($;$) {
    my ($first, $second) = @_;
    return defined $second ? $first + $second : $first;
}

# Test method prototype
sub proto_method ($) {
    my ($self) = @_;
    return ref $self;
}

# Test empty prototype
sub proto_empty () {
    return "no arguments";
}

# Test prototype with multiple scalars
sub proto_multi_scalar ($$$$) {
    my ($a, $b, $c, $d) = @_;
    return $a + $b + $c + $d;
}

# Test prototype with lvalue
sub proto_lvalue :lvalue {
    return $_[0];
}

# Main execution
print "Testing prototypes\n";

# Test prototype array
my @test_array = (1, 2, 3, 4, 5);
my $result1 = proto_array(\@test_array);
print "proto_array(\\@test_array) = $result1\n";

# Test prototype hash
my %test_hash = (a => 1, b => 2, c => 3);
my $result2 = proto_hash(\%test_hash);
print "proto_hash(\\%test_hash) = $result2\n";

# Test prototype code
my $result3 = proto_code(sub { return 42; });
print "proto_code(sub { return 42; }) = $result3\n";

# Test prototype mixed
my $result4 = proto_mixed(10, 20);
print "proto_mixed(10, 20) = $result4\n";

# Test prototype optional
my $result5 = proto_optional(10);
print "proto_optional(10) = $result5\n";

# Test prototype method
my $obj = bless { value => 100 }, 'TestObject';
my $result6 = proto_method($obj);
print "proto_method(\$obj) = $result6\n";

# Test prototype empty
my $result7 = proto_empty();
print "proto_empty() = $result7\n";

# Test prototype multi scalar
my $result8 = proto_multi_scalar(1, 2, 3, 4);
print "proto_multi_scalar(1, 2, 3, 4) = $result8\n";

# Test prototype lvalue
proto_lvalue() = 30;
print "proto_lvalue() = 30\n";

print "All prototype tests completed successfully\n";

# Package for testing
package TestObject;
sub new {
    my ($class, $value) = @_;
    return bless { value => $value }, $class;
}

1;