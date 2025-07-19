#!/usr/bin/perl
# Test qualified name parsing

# This should fail to parse
Foo::Bar->new();

# Workaround with quotes
"Foo::Bar"->new();

# With variable
my $class = "Foo::Bar";
$class->new();

# Direct identifier works
Foo->new();

# Package declaration works
package Foo::Bar;

# Use statement works
use Foo::Bar;

# Qualified name in other contexts
my $ref = \&Foo::Bar::method;