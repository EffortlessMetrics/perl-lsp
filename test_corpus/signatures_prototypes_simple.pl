#!/usr/bin/env perl
# Test: Simple Signatures and Prototypes
# Impact: Ensures parser handles advanced subroutine parameter features
# NodeKinds: Signature, Prototype, MandatoryParameter, OptionalParameter, NamedParameter, SlurpyParameter

use strict;
use warnings;
use feature 'signatures';
no warnings 'experimental::signatures';

# Basic signatures
sub basic_sig($x, $y) {
    return $x + $y;
}

# Signature with default values
sub defaults($x = 10, $y = 20) {
    return $x + $y;
}

# Signature with optional parameters
sub optional($req, $opt = undef) {
    return $req unless defined $opt;
    return $req + $opt;
}

# Signature with slurpy array
sub slurpy_array($first, @rest) {
    my $sum = $first;
    $sum += $_ for @rest;
    return $sum;
}

# Signature with slurpy hash - simplified (no multiple slurpy params)
sub slurpy_hash_only(%options) {
    return \%options;
}

# Signature with invocant (method syntax)
sub method_sig($self, $arg) {
    return "$self->{value}: $arg";
}

# Complex signature combinations
sub complex_sig(
    $mandatory,
    $optional = 'default',
    @slurpy_array
) {
    return {
        mandatory => $mandatory,
        optional => $optional,
        array_count => scalar @slurpy_array,
    };
}

# Empty signature
sub empty_sig() {
    return "no parameters";
}

# Subroutine prototypes
sub proto_scalar ($) {
    my ($x) = @_;
    return $x * 2;
}

sub proto_array (@) {
    my (@arr) = @_;
    return scalar @arr;
}

sub proto_hash (%) {
    my (%hash) = @_;
    return scalar keys %hash;
}

sub proto_mixed ($@) {
    my ($first, @rest) = @_;
    return $first + scalar @rest;
}

sub proto_code (&) {
    my ($code) = @_;
    return $code->();
}

sub proto_glob (*) {
    my ($handle) = @_;
    return $handle;
}

sub proto_star ($) {
    my ($ref) = @_;
    return ref $ref;
}

# Complex prototypes
sub proto_complex ($\@;$) {
    my ($scalar, $array_ref, $optional) = @_;
    return $scalar + @$array_ref + ($optional // 0);
}

sub proto_multi ($$$) {
    my ($x, $y, $z) = @_;
    return $x + $y + $z;
}

# Prototype with special characters
sub proto_proto (;$$) {
    my ($opt1, $opt2) = @_;
    return ($opt1 // 0) + ($opt2 // 0);
}

# Anonymous sub with signature
my $anon_sig = sub ($x, $y = 1) {
    return $x * $y;
};

# Method with signature in class context
package MyClass {
    use feature 'signatures';
    no warnings 'experimental::signatures';
    
    # Simulate field with a hash
    sub new {
        my ($class, $value) = @_;
        return bless { value => $value // 0 }, $class;
    }
    
    sub get_value {
        my ($self) = @_;
        return $self->{value};
    }
    
    sub set_value {
        my ($self, $new_value) = @_;
        $self->{value} = $new_value;
    }
    
    sub compute {
        my ($self, $factor) = @_;
        $factor //= 1;
        return $self->{value} * $factor;
    }
}

# Package with prototype and signature mix
package MixedPackage {
    use feature 'signatures';
    no warnings 'experimental::signatures';
    
    # Prototype function
    sub proto_func ($) {
        my ($x) = @_;
        return $x * 3;
    }
    
    # Signature function
    sub sig_func ($x) {
        return $x * 4;
    }
    
    # Mixed
    sub mixed_proto_sig ($;$) {
        my ($req, $opt) = @_;
        return $req + ($opt // 0);
    }
}

package main;

# Signature with default expressions
sub expr_defaults($x = 1 + 2, $y = $x * 2) {
    return $x + $y;
}

# Signature with undef default
sub undef_default($x, $y = undef) {
    return defined $y ? $x + $y : $x;
}

# Prototype with array reference
sub proto_array_ref (\@) {
    my ($array_ref) = @_;
    return scalar @$array_ref;
}

# Prototype with hash reference
sub proto_hash_ref (\%) {
    my ($hash_ref) = @_;
    return scalar keys %$hash_ref;
}

# Prototype with code reference
sub proto_code_ref (\&) {
    my ($code_ref) = @_;
    return $code_ref->();
}

print "All signature and prototype tests completed\n";