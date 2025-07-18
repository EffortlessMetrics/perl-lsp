#!/usr/bin/perl

# Simple method call
$obj->method();

# Method call with args
$obj->process_data($callback);

# Chained method calls
$obj->new()->process();

# Class method call
MyClass->new();

# Method call with complex args
MyClass->new(
    data => [1..10],
    debug => 1
);