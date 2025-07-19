#!/usr/bin/perl
use strict;
use warnings;

my $obj = {};
my $result;

# Test ISA operator
$result = $obj isa 'HASH';
print "ISA test: $result\n";

# Test in if statement
if ($obj isa 'HASH') {
    print "Object is a HASH\n";
}

# Test with package names
package MyClass;
sub new { bless {}, shift }

package main;
my $instance = MyClass->new();
if ($instance isa 'MyClass') {
    print "Instance is MyClass\n";
}