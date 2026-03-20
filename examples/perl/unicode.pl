#!/usr/bin/perl
# examples/perl/unicode.pl
#
# Demonstrates: UTF-8 source files, Unicode identifiers, wide strings,
#               Unicode-aware regex, and the Encode module.
#
# LSP features exercised:
#   - hover       : hover over \N{...} for codepoint info
#   - diagnostics : warn if 'use utf8' is missing in a file with non-ASCII chars

use strict;
use warnings;
use utf8;                    # source file is UTF-8
use feature 'say';
use open ':std', ':utf8';    # standard handles are UTF-8

# ---------------------------------------------------------------------------
# 1. Unicode identifiers
# ---------------------------------------------------------------------------

my $pi    = 3.14159265358979;
my $tau   = 2 * $pi;

sub circumference_of_circle {
    my ($r) = @_;
    return $tau * $r;
}

say "circumference(5) = " . circumference_of_circle(5);

# ---------------------------------------------------------------------------
# 2. Unicode string literals
# ---------------------------------------------------------------------------

my $greeting_ja    = "こんにちは、世界！";         # Japanese
my $greeting_ar    = "مرحبا بالعالم";            # Arabic
my $greeting_ru    = "Привет, мир!";             # Russian
my $greeting_emoji = "Hello \x{1F600} World";   # emoji via codepoint

say $greeting_ja;
say $greeting_ar;
say $greeting_ru;
say $greeting_emoji;

# Named Unicode characters via \N{...}
my $snowman      = "\N{SNOWMAN}";           # U+2603
my $musical_note = "\N{MUSICAL NOTE}";      # U+266A
my $copyright    = "\N{COPYRIGHT SIGN}";    # U+00A9
say "$snowman $musical_note $copyright";

# ---------------------------------------------------------------------------
# 3. String operations on Unicode text
# ---------------------------------------------------------------------------

my $mixed = "caf\x{00E9}";   # cafe with accented e (U+00E9)

say length($mixed), " characters";   # 4 characters (not bytes)

my $upper = uc($mixed);
say $upper;   # CAFE with accent

# Reverse a Unicode string by characters
my $reversed = join '', reverse split //, $mixed;
say $reversed;

# ---------------------------------------------------------------------------
# 4. Unicode-aware regex
# ---------------------------------------------------------------------------

my $sentence = "El nino jugo futbol";
my @words = ($sentence =~ /(\w+)/g);
say "words: ", join(', ', @words);

# Detect non-ASCII characters
my $unicode_sentence = "El ni\x{00F1}o jug\x{00F3} f\x{00FA}tbol";
if ($unicode_sentence =~ /[^\x00-\x7F]/) {
    say "sentence contains non-ASCII characters";
}

# Unicode category via \p{} (requires 'use utf8' or Unicode::UCD)
my $mixed_text = "Hello123 \x{041F}\x{0440}\x{0438}\x{0432}\x{0435}\x{0442}456";
my @letters = ($mixed_text =~ /(\p{L}+)/g);
say "letter sequences: ", join(', ', @letters);

# ---------------------------------------------------------------------------
# 5. Encoding round-trips (Encode module)
# ---------------------------------------------------------------------------

use Encode qw(encode decode);

my $perl_string = "caf\x{00E9}";   # internal Perl string (UTF-8 flag on)

# Encode to bytes for I/O
my $utf8_bytes   = encode('UTF-8',   $perl_string);
my $latin1_bytes = encode('Latin-1', $perl_string);

say "UTF-8 byte length:   " . length($utf8_bytes);    # 5 bytes for "cafe+accent"
say "Latin-1 byte length: " . length($latin1_bytes);  # 4 bytes

# Decode bytes back to string
my $decoded = decode('UTF-8', $utf8_bytes);
say "decoded: $decoded";

# ---------------------------------------------------------------------------
# 6. Unicode-named sub (identifier edge case)
# ---------------------------------------------------------------------------

sub resume_parser {
    my ($input) = @_;
    # Demonstrates that the parser handles longer identifiers correctly
    return length($input);
}

say resume_parser($greeting_ja), " characters in Japanese greeting";
