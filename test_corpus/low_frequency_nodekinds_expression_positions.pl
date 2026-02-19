#!/usr/bin/env perl
# Test: Low-Frequency NodeKinds in alternative syntactic positions
# Impact: Provides a second "angle" for thin-coverage NodeKinds
# NodeKinds: Given, When, Default, LabeledStatement, NamedParameter, VariableWithAttributes

use strict;
use warnings;

# --- Given/When/Default in nested context ---
use feature 'switch';
no warnings 'experimental::smartmatch';

sub classify_value {
    my ($input) = @_;
    my $result;
    given ($input) {
        when ([1, 2, 3]) { $result = "small"; }
        when (/^\d+$/)   { $result = "numeric"; }
        default           { $result = "other"; }
    }
    return $result;
}

# Given/when inside a loop
for my $val (1, 10, "hello") {
    given ($val) {
        when (1)       { print "one\n"; }
        when (10)      { print "ten\n"; }
        default        { print "default: $val\n"; }
    }
}

# --- LabeledStatement with lowercase labels ---
# The v3 parser recognises lowercase labels as LabeledStatement.

search_loop: for my $haystack ("foo", "bar", "baz") {
    inner_loop: for my $needle ("a", "b") {
        if ($haystack eq "bar" && $needle eq "b") {
            last search_loop;
        }
    }
}

retry_loop: while (1) {
    my $attempt = int(rand(5));
    last retry_loop if $attempt == 0;
}

# --- NamedParameter in signatures ---
use feature 'signatures';

sub greet(:$name, :$greeting = "Hello") {
    return "$greeting, $name!";
}

sub configure(:$host, :$port = 8080, :$verbose = 0) {
    return "$host:$port (verbose=$verbose)";
}

# --- VariableWithAttributes ---
# Demonstrates variable attributes in different declaration forms.

my ($shared_x :shared, $locked_y :locked) = (1, 2);

sub worker {
    my ($data :shared) = @_;
    return $data;
}
