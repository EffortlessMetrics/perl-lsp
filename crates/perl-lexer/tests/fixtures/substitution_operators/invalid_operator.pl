#!/usr/bin/env perl
# Invalid substitution operator test fixtures
# Tests for AC2: Substitution operator error handling
# These should trigger TokenType::Error instead of unreachable!()

# Invalid operator 'm' with delimiter (m// is match, not substitution)
$str =~ m/foo/bar/;

# Invalid operator 'q' with delimiter (q// is quote, not substitution)
$str =~ q/foo/bar/;

# Invalid operator 'qq' with delimiter
$str =~ qq/foo/bar/;

# Invalid operator 'qx' with delimiter
$str =~ qx/foo/bar/;

# Invalid operator 'qw' with delimiter
$str =~ qw/foo bar/;

# Invalid operator 'qr' with delimiter
$str =~ qr/foo/bar/;

# Malformed substitution (missing closing delimiter)
$str =~ s/foo/bar;

# Malformed transliteration
$str =~ tr/abc/;

# Invalid escape sequence in delimiter
$str =~ s\nfoo\nbar\n;

# Double substitution operator
$str =~ ss/foo/bar/;

# Mixed operators
$str =~ st/foo/bar/;

# Unknown operator
$str =~ x/foo/bar/;

# Invalid modifier on wrong operator
$str =~ tr/a-z/A-Z/e;  # 'e' modifier invalid for tr

# Too many delimiters
$str =~ s/foo/bar/baz/;
