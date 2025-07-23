#!/usr/bin/env perl
# Test various Perl features

# Regex with modifiers
my $text = "Hello World";
if ($text =~ /hello/i) { print "match\n"; }

# Substitution
my $str = "foo bar";
$str =~ s/foo/baz/g;

# Transliteration
my $upper = "hello";
$upper =~ tr/a-z/A-Z/;

# qw construct
my @words = qw(one two three);
my @parens = qw(one two three);
my @braces = qw{one two three};

# Heredoc
my $heredoc = <<'END';
This is a heredoc
with multiple lines
END

print "All done\n";