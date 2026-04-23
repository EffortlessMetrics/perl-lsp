#!/usr/bin/env perl
# Test: Advanced regex features
# Impact: Complex regex patterns can break lexers; critical for stability

use v5.32;
use warnings;

# Recursive patterns with named groups
my $palindrome = qr/
    (?<pal>
        \w
        (?:
            (?&pal)  # Recursive call to 'pal' group
            |
            \w?
        )
        \w
    )
/x;

"racecar" =~ /$palindrome/ and say "Found palindrome: $+{pal}";

# Recursive pattern for balanced parentheses
my $balanced = qr/
    \(
    (?:
        [^()]+
        |
        (?R)     # Recurse entire pattern
    )*
    \)
/x;

"(a(b(c)d)e)" =~ /$balanced/ and say "Balanced parens";

# Code assertions (?{...})
my $count = 0;
"aaa" =~ /a(?{ $count++ })*/;
say "Count: $count";

# Conditional patterns
"test123" =~ /
    (?<word>\w+)
    (?(word)        # If 'word' matched
        \d+         # Then match digits
        |           # Else
        \s+         # Match spaces
    )
/x;

# Branch reset groups - all capture to $1
"abc" =~ /(?|(a)|(b)|(c))/ and say "Captured: $1";

# Possessive quantifiers
"aaab" =~ /a++b/ and say "Possessive match";

# Atomic groups
"abc" =~ /(?>a*)bc/ and say "Atomic group";

# Named backreferences
"the the" =~ /(?<word>\w+)\s+\k<word>/ and say "Duplicate word";

# Relative backreferences
"abcabc" =~ /(abc)\g{-1}/ and say "Relative backref";

# Verbs
"test" =~ /t(*PRUNE)est|test/ or say "PRUNE prevented backtrack";
"abc" =~ /a(*SKIP)b|./ and say "SKIP verb";
"xyz" =~ /x(*FAIL)|.*/ or say "FAIL verb";

# Unicode properties
"Hello 世界 🌍" =~ /\p{Han}/ and say "Has Chinese";
"Γεια σου" =~ /\p{Greek}/ and say "Has Greek";
"🌍🌎🌏" =~ /\p{Emoji}/ and say "Has emoji";

# Unicode scripts and blocks
"अनुच्छेद" =~ /\p{Script=Devanagari}/ and say "Devanagari script";
"𐌰𐌱𐌲" =~ /\p{Block=Gothic}/ and say "Gothic block";

# Grapheme clusters
"e\x{301}" =~ /\X/ and say "Single grapheme";

# Case folding with unicode
"ß" =~ /SS/i and say "German sharp s matches SS";

# Look-around assertions
"test123" =~ /(?<=test)\d+/ and say "Positive lookbehind";
"test123" =~ /\w+(?=\d)/ and say "Positive lookahead";
"test123" =~ /(?<!foo)\d+/ and say "Negative lookbehind";
"test123" =~ /\w+(?!\d)/ or say "Negative lookahead failed";

# Variable-length lookbehind (5.30+)
"abc123" =~ /(?<=\w+)\d/ and say "Variable lookbehind";

# Embedded code with return value
my $result = "test" =~ s/(\w+)(?{ uc $1 })/replacement/er;

# Deferred regex (??>pattern)
my $pattern = qr/\d+/;
"abc123" =~ /(??{ $pattern })/ and say "Deferred pattern";

# Script runs (5.28+)
"abc" =~ /(*script_run:\w+)/ and say "Script run";

# Atomic script runs (5.32+)
"test" =~ /(*atomic_script_run:\w+)/ and say "Atomic script run";

# Named captures in substitution
"John Doe" =~ s/(?<first>\w+)\s+(?<last>\w+)/$+{last}, $+{first}/;

# Keep assertion \K
"prefix:value" =~ /\w+:\K\w+/ and say "Matched: $&";

# Extended Unicode properties
"123" =~ /\p{Numeric_Value=3}/ and say "Has numeric value 3";
"Ⅻ" =~ /\p{Numeric_Type=Roman}/ and say "Roman numeral";

# Modifiers in regex
"TEST" =~ /(?i:test)/ and say "Case insensitive section";
"a b" =~ /(?x: a \s b )/ and say "Extended mode section";

# All regex modifiers
my $all_mods = qr/
    pattern
/msixpodualn;

# G assertion for global matches
my $str = "a1b2c3";
while ($str =~ /\G(\w)(\d)/g) {
    say "Letter: $1, Digit: $2";
}

# Zero-width assertions
"test" =~ /^test$/ and say "Anchors";
"test\nmore" =~ /test$/m and say "Multiline anchor";
"word" =~ /\bword\b/ and say "Word boundary";

# Character classes with Unicode
"café" =~ /[[:alpha:]]+/ and say "POSIX alpha";
"123" =~ /[[:digit:]]+/ and say "POSIX digit";
"_test" =~ /[[:word:]]+/ and say "POSIX word";

# Negated properties
"abc" =~ /\P{Digit}/ and say "Not a digit";
"123" =~ /\P{Letter}/ and say "Not a letter";

# Regex with different delimiters
"test" =~ m!test! and say "Bang delimiter";
"test" =~ m{test} and say "Brace delimiter";
"test" =~ m[test] and say "Bracket delimiter";
"test" =~ m<test> and say "Angle delimiter";
"test" =~ m|test| and say "Pipe delimiter";
"test" =~ m#test# and say "Hash delimiter";

# Substitution with code evaluation
my $text = "hello world";
$text =~ s/(\w+)/uc($1)/eg;
say $text;  # HELLO WORLD

# Transliteration with modifiers
my $trans = "Hello World";
$trans =~ tr/a-z/A-Z/;
$trans =~ y/A-Z/a-z/c;  # Complement
$trans =~ tr/a-z//d;    # Delete
$trans =~ tr/a-z/x/s;   # Squash

__END__
Parser assertions:
1. No infinite loops or hangs on recursive patterns
2. Regex delimiters properly balanced
3. Code blocks in regex don't break token boundaries  
4. Unicode properties recognized as part of pattern
5. All modifier combinations handled
6. Substitution with /e flag parsed correctly
7. Named captures available for completion