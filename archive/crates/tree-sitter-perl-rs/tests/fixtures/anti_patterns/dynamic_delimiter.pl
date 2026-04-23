#!/usr/bin/env perl
# Anti-pattern test fixture: dynamic delimiter
# Tests for AC5: Anti-pattern detector exhaustive matching
# DynamicDelimiterDetector should identify these patterns

use strict;
use warnings;

# Dynamic delimiter anti-pattern
# Using variable delimiters in regex/substitution is problematic for static analysis

# Variable delimiter in regex
my $delim = '/';
my $pattern = eval "qr${delim}foo${delim}";

# Variable delimiter in substitution
my $str = "foo bar";
my $open = '{';
my $close = '}';
eval "\$str =~ s${open}foo${close}${open}baz${close}";

# Computed delimiter
sub get_delimiter {
    return shift() ? '/' : '#';
}
my $d = get_delimiter(1);
eval "my \$re = qr${d}pattern${d}";

# Array of delimiters
my @delimiters = qw(/ # | !);
foreach my $del (@delimiters) {
    eval "my \$regex = qr${del}test${del}";
}

# Hash-based delimiter selection
my %delim_map = (
    slash => '/',
    hash  => '#',
    pipe  => '|'
);
my $selected = $delim_map{slash};
eval "my \$r = qr${selected}pattern${selected}";

# Conditional delimiter
my $delimiter = $condition ? '/' : '#';
my $regex = eval "qr${delimiter}test${delimiter}";

# Valid static delimiters (not anti-pattern)
my $valid1 = qr/foo/;
my $valid2 = qr#bar#;
my $valid3 = qr{baz};
$str =~ s/foo/bar/;
$str =~ s#foo#bar#;

# Valid static quote operators (not anti-pattern)
my $q1 = q/string/;
my $q2 = qq{interpolated};
my $q3 = qx/command/;
my @q4 = qw(word list);

print "Normal code continues\n";

1;
