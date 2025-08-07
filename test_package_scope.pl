#!/usr/bin/env perl
package MyPackage;
our $package_var = 10;
my $lexical_var = 20;

sub get_package { return $package_var; }
sub get_lexical { return $lexical_var; }