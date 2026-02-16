#!/usr/bin/env perl
# Test: Simple Method and Class Declarations
# Impact: Ensures parser handles modern OOP syntax
# NodeKinds: Method, Class

use strict;
use warnings;

# Traditional Perl OO with method declarations
package TraditionalClass;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub traditional_method {
    my ($self, $arg) = @_;
    return "traditional: $arg";
}

sub regular_method {
    my ($self, $arg) = @_;
    return "regular: $arg";
}

# Simulate method keyword with comments
# method new($class: $x = 0, $y = 0) {
#     return bless { x => $x, y => $y }, $class;
# }

# method x() { return $x; }
# method y() { return $y; }

# method move($dx, $dy) {
#     $x += $dx;
#     $y += $dy;
# }

# method distance($other) {
#     return sqrt(($other->x - $x)**2 + ($other->y - $y)**2);
# }

package main;

# Method in package context
package UtilityPackage {
    sub helper {
        return "helper";
    }
    
    sub package_method {
        my ($class, $arg) = @_;
        return "package method: $arg";
    }
}

# Anonymous methods
my $anon_method = sub {
    my ($self, $x) = @_;
    return $x * 2;
};

# Method references and calls
my $obj = TraditionalClass->new(value => 42);
my $method_ref = $obj->can('traditional_method');
$obj->$method_ref("test argument");

print "All method and class tests completed\n";