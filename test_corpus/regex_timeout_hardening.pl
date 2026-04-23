#!/usr/bin/env perl
# Test: Regex timeout/hang hardening cases
# Impact: Keep parser resilient on advanced regex constructs tied to timeout-risk reports

use strict;
use warnings;

my $text = 'abc123xyz';

# Branch reset groups (including nested form)
my $branch_reset = qr/(?|(a)(b)|(c)(d))/;
my $nested_branch_reset = qr/(?|(?|(ab))(cd)|(ef)(gh))/;

# Variable-length lookbehind / lookahead combinations
my $var_lookbehind = qr/(?<=\w{1,4})\d+/;
my $neg_lookbehind = qr/(?<!foo\d{1,3})bar/;
my $mixed_lookaround = qr/(?<=prefix\w+)(\d+)(?=suffix)/;

# Ambiguous slash parsing: division-like expressions plus regex literals
my $ratio = 100 / 5 / 2;
my $contains_digits = $text =~ /\d+/;
my $escaped_slash = $text =~ /abc\/123/;

# Catastrophic backtracking candidate should still parse quickly
my $catastrophic_candidate = qr/^(a+)+b$/;

# Code assertion and deferred pattern forms
my $counter = 0;
my $code_assertion = qr/(?:a(?{ $counter++ }))+/;
my $fragment = qr/\w+/;
my $deferred = qr/(??{ $fragment })/;

# Unicode property usage from timeout-risk inventory
my $unicode_props = qr/\p{Greek}+\p{Script=Devanagari}+/;

# Multiple heredocs on one line (lexer boundary stress)
print <<'LEFT', <<'RIGHT';
left
LEFT
right
RIGHT

# Heredoc body containing near-terminator text
my $doc = <<'END_DOC';
END_DOCISH should not terminate heredoc.
END_DOC

# Keep file executable-like while remaining parser focused
if ($text =~ $branch_reset || $text =~ $var_lookbehind) {
    print "regex hardening corpus fixture\n";
}
