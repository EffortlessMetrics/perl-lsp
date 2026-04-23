#!/usr/bin/env perl
# Valid 'y///' transliteration operator test fixtures
# Tests for AC2: Substitution operator error handling
# Note: 'y' is an alias for 'tr'

# Basic transliteration (y is identical to tr)
$str =~ y/a-z/A-Z/;

# Transliteration with modifiers
$str =~ y/a-z/A-Z/c;       # complement
$str =~ y/a-z/A-Z/d;       # delete
$str =~ y/a-z/A-Z/s;       # squeeze
$str =~ y/a-z/A-Z/cd;      # complement + delete
$str =~ y/a-z/A-Z/ds;      # delete + squeeze
$str =~ y/a-z/A-Z/cds;     # all modifiers

# Alternative delimiters
$str =~ y#a-z#A-Z#;
$str =~ y|a-z|A-Z|;
$str =~ y{a-z}{A-Z};
$str =~ y[a-z][A-Z];
$str =~ y<a-z><A-Z>;
$str =~ y(a-z)(A-Z);

# Character classes
$str =~ y/0-9/a-j/;
$str =~ y/A-Za-z/N-ZA-Mn-za-m/;  # ROT13

# Delete characters
$str =~ y/a-z//d;

# Squeeze duplicates
$str =~ y/ //s;

# Count characters (return value)
my $count = ($str =~ y/a//);

# Mixed usage with tr (both are valid)
$str1 =~ tr/a-z/A-Z/;
$str2 =~ y/a-z/A-Z/;

# Unicode transliteration
$str =~ y/\x{0100}-\x{01ff}/\x{0200}-\x{02ff}/;
