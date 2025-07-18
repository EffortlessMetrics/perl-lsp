#!/usr/bin/perl

# Quote nesting stress test
my $x = qq{outer qq{middle qq{inner qq{deep}}}};
my $y = q{outer q{middle q{inner q{deep}}}};
my $z = qr{outer (?:qr{middle (?:qr{inner (?:qr{deep})})})};
my $w = s{q{a}q{b}q{c}}{qq{x}qq{y}qq{z}};
