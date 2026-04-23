#!/usr/bin/env perl
# Valid 's///' substitution operator test fixtures
# Tests for AC2: Substitution operator error handling

# Basic substitution
$str =~ s/foo/bar/;

# Substitution with modifiers
$str =~ s/foo/bar/g;        # global
$str =~ s/foo/bar/i;        # case-insensitive
$str =~ s/foo/bar/gi;       # global + case-insensitive
$str =~ s/foo/bar/x;        # extended regex
$str =~ s/foo/bar/s;        # single-line
$str =~ s/foo/bar/m;        # multi-line
$str =~ s/foo/bar/gixsm;    # all modifiers

# Alternative delimiters
$str =~ s#foo#bar#;
$str =~ s|foo|bar|;
$str =~ s{foo}{bar};
$str =~ s[foo][bar];
$str =~ s<foo><bar>;
$str =~ s(foo)(bar);
$str =~ s!foo!bar!;
$str =~ s@foo@bar@;
$str =~ s%foo%bar%;
$str =~ s^foo^bar^;
$str =~ s&foo&bar&;
$str =~ s*foo*bar*;
$str =~ s+foo+bar+;
$str =~ s,foo,bar,;
$str =~ s.foo.bar.;
$str =~ s:foo:bar:;
$str =~ s;foo;bar;;

# Single-quote delimiter (PR #158)
$str =~ s'foo'bar';

# Whitespace delimiters
$str =~ s foo bar ;

# Complex patterns
$str =~ s/(\w+)/uc($1)/e;   # evaluation modifier
$str =~ s/(\d+)/$1 * 2/e;
$str =~ s/^(\s+)//;          # leading whitespace
$str =~ s/(\s+)$//;          # trailing whitespace

# Unicode in substitution
$str =~ s/café/coffee/;
$str =~ s/日本語/Japanese/;

# Backreferences
$str =~ s/(\w+)\s+\1/$1/;

# Empty replacement
$str =~ s/foo//;

# Empty pattern (repeats last pattern)
$str =~ s//bar/;
