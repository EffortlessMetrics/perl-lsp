#!/usr/bin/perl

# Test indirect object syntax

# Basic print cases
print STDOUT "Hello, World!\n";
print STDERR "Error message\n";
print $fh "To filehandle\n";

# Constructor syntax
my $obj = new Class;
my $obj2 = new Class::Name;
my $obj3 = new Class "arg1", "arg2";

# Other builtins with indirect syntax
open HANDLE, "file.txt";
close HANDLE;
printf STDERR "%s\n", "formatted";

# Method calls with indirect syntax
method $obj @args;