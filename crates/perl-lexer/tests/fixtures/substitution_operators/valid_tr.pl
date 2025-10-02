#!/usr/bin/env perl
# Valid 'tr///' transliteration operator test fixtures
# Tests for AC2: Substitution operator error handling

# Basic transliteration
$str =~ tr/a-z/A-Z/;

# Transliteration with modifiers
$str =~ tr/a-z/A-Z/c;       # complement
$str =~ tr/a-z/A-Z/d;       # delete
$str =~ tr/a-z/A-Z/s;       # squeeze
$str =~ tr/a-z/A-Z/cd;      # complement + delete
$str =~ tr/a-z/A-Z/ds;      # delete + squeeze
$str =~ tr/a-z/A-Z/cds;     # all modifiers

# Alternative delimiters
$str =~ tr#a-z#A-Z#;
$str =~ tr|a-z|A-Z|;
$str =~ tr{a-z}{A-Z};
$str =~ tr[a-z][A-Z];
$str =~ tr<a-z><A-Z>;
$str =~ tr(a-z)(A-Z);

# Character classes
$str =~ tr/0-9/a-j/;
$str =~ tr/A-Za-z/N-ZA-Mn-za-m/;  # ROT13

# Delete characters
$str =~ tr/a-z//d;

# Squeeze duplicates
$str =~ tr/ //s;

# Count characters (return value)
my $count = ($str =~ tr/a//);

# Unicode transliteration
$str =~ tr/\x{0100}-\x{01ff}/\x{0200}-\x{02ff}/;

# Escape sequences
$str =~ tr/\n\r\t/ /s;

# Complement and delete
$str =~ tr/a-zA-Z0-9//cd;  # Keep only alphanumeric

# Empty transliteration sets
$str =~ tr///d;  # Delete all characters
