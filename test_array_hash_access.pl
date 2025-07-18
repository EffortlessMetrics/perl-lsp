#!/usr/bin/perl
# Array access
my @array = (1, 2, 3, 4, 5);
my $first = $array[0];
my $last = $array[-1];

# Hash access  
my %hash = (foo => 'bar', baz => 'qux');
my $value = $hash{foo};
my $key = 'baz';
my $value2 = $hash{$key};

# Complex access
my $complex = $data{users}[0]{name};