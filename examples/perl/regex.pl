#!/usr/bin/perl
# examples/perl/regex.pl
#
# Demonstrates: regex literals, substitutions, named captures, qr//,
#               lookahead/lookbehind, and the /x verbose flag.
#
# LSP features exercised:
#   - hover       : hover over a regex for an inline explainer
#   - diagnostics : warn on known anti-patterns (e.g. . without \n flag)

use strict;
use warnings;
use feature 'say';

# ---------------------------------------------------------------------------
# 1. Basic match / capture
# ---------------------------------------------------------------------------

my $email = 'user@example.com';

if ($email =~ /^([^@]+)\@(.+)$/) {
    my ($user, $domain) = ($1, $2);
    say "user=$user  domain=$domain";
}

# Named captures (hover shows capture name in tooltip)
if ($email =~ /^(?<user>[^@]+)\@(?<domain>.+)$/) {
    say "named: user=$+{user}  domain=$+{domain}";
}

# ---------------------------------------------------------------------------
# 2. Substitution
# ---------------------------------------------------------------------------

my $text = 'Hello, World!';
(my $lower = $text) =~ s/[A-Z]/lc($&)/ge;
say $lower;

# Multiline substitution with /x for readability
my $html = '<b>bold</b> and <i>italic</i>';
$html =~ s{
    <           # open tag
    (\w+)       # tag name
    >           # close angle bracket
    (.*?)       # content (non-greedy)
    </\1>       # matching close tag
}{[$1: $2]}gx;
say $html;

# ---------------------------------------------------------------------------
# 3. Alternation and character classes
# ---------------------------------------------------------------------------

my @words = qw(cat bat hat sat rat);
my @rhymes = grep { /^[cbhs]at$/ } @words;
say "rhymes: @rhymes";

# POSIX-like classes
my $str = 'abc123 DEF 456';
(my $digits_only = $str) =~ s/\D+//g;
say "digits: $digits_only";

# ---------------------------------------------------------------------------
# 4. Precompiled regex with qr//
# ---------------------------------------------------------------------------

# Build a regex at runtime and reuse it
my $word_re = qr/\b\w{4,}\b/;    # hover: full pattern expansion

my $sentence = 'The quick brown fox jumps over the lazy dog';
my @long_words = ($sentence =~ /$word_re/g);
say "long words: @long_words";

# Combine qr// patterns
my $start = qr/^\s*/;
my $end   = qr/\s*$/;
my $trim  = qr/${start}(.+?)${end}/s;
if ($sentence =~ $trim) {
    say "trimmed: $1";
}

# ---------------------------------------------------------------------------
# 5. Lookahead and lookbehind
# ---------------------------------------------------------------------------

my $price_str = 'Total: $42.00 USD and $19.99 EUR';

# Lookbehind for dollar sign (hover shows: zero-width assertion)
my @amounts = ($price_str =~ /(?<=\$)\d+\.\d+/g);
say "amounts: @amounts";

# Lookahead -- match word before a colon
my $config = 'host: localhost  port: 5432  db: mydb';
my @keys = ($config =~ /(\w+)(?=:)/g);
say "keys: @keys";

# ---------------------------------------------------------------------------
# 6. Global match in list context
# ---------------------------------------------------------------------------

my $csv = '1,2,3,hello world,"quoted,field",end';
my @fields = ($csv =~ /(?:"[^"]*"|[^,]+)/g);
for my $f (@fields) {
    $f =~ s/^"|"$//g;    # strip surrounding quotes
    say "  field: $f";
}

# ---------------------------------------------------------------------------
# 7. tr/// (transliteration)
# ---------------------------------------------------------------------------

my $rot13 = 'Hello, World!';
$rot13 =~ tr/A-Za-z/N-ZA-Mn-za-m/;
say "ROT13: $rot13";

my $vowel_count = () = ($text =~ /[aeiou]/gi);
say "vowels in '$text': $vowel_count";
