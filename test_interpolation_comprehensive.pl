#!/usr/bin/perl
my $name = "World";
my $count = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value');

# Simple scalar interpolation
my $s1 = "Hello, $name!";
my $s2 = "Count is $count";

# Array interpolation
my $s3 = "Array: @array";
my $s4 = "First element: $array[0]";

# Hash interpolation
my $s5 = "Hash value: $hash{key}";

# Complex expressions with braces
my $s6 = "Expression: ${name}s";
my $s7 = "Complex: ${name}_suffix";

# Mixed content
my $s8 = "Name: $name, Count: $count, Array: @array";

# Escaped characters
my $s9 = "Quote: \" Tab: \t Newline: \n";
my $s10 = "Dollar: \$ At: \@";

# Empty and simple strings
my $s11 = "";
my $s12 = "No interpolation here";

print "$s1\n";