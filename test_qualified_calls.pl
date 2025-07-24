#!/usr/bin/env perl
# Test qualified function calls

# Direct qualified call
SUPER::method();
Package::function();
Foo::Bar::baz();

# Method calls with qualified names
$self->SUPER::method();
$obj->Package::method();

# More complex cases
$self->SUPER::method($arg1, $arg2);
My::Package::function(1, 2, 3);