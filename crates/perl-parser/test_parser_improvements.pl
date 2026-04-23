#!/usr/bin/env perl
# Test all the improvements made to perl-parser

# 1. Regex with modifiers
if ($text =~ /hello/i) { print "match\n"; }
if ($text =~ /world/gimsx) { print "match with multiple modifiers\n"; }

# 2. Substitution with replacement
my $str = "foo bar";
$str =~ s/foo/baz/g;
$str =~ s{old}{new}gi;
$str =~ s[pattern][replacement]e;

# 3. Transliteration
my $upper = "hello";
$upper =~ tr/a-z/A-Z/;
$upper =~ y/0-9/a-j/c;
$upper =~ tr{a-z}{A-Z}d;

# 4. qw() construct
my @words = qw(one two three);
my @parens = qw(alpha beta gamma);
my @braces = qw{foo bar baz};
my @brackets = qw[x y z];

# 5. Heredoc with content
my $heredoc = <<'END';
This is a heredoc
with multiple lines
of content
END

my $interpolated = <<"EOF";
Hello $name
Today is $day
EOF

my $indented = <<~'INDENT';
    This content
    will have indentation
    stripped
INDENT

# 6. Quote operators with modifiers
my $regex = qr/pattern/i;
my $compiled = qr{foo.*bar}ms;
my $match = m/test/g;

print "All tests completed\n";