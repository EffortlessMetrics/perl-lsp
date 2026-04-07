#!/usr/bin/env perl
use utf8;
# Unicode variable name test fixtures
# Tests for AC1: Variable declaration error handling with Unicode

# Valid Unicode identifiers (Perl 5.8+)
my $café = "coffee";
my $日本語 = "Japanese";
my $Ελληνικά = "Greek";
my $Русский = "Russian";
my $中文 = "Chinese";

# Unicode in array names
my @数组 = (1, 2, 3);

# Unicode in hash names
my %哈希 = (key => 'value');

# Emoji in variable names (Perl 5.14+)
my $😀 = "happy";
my $🎉 = "celebration";

# Mixed ASCII and Unicode
my $user_名前 = "user name";

# Complex Unicode characters
my $café_résumé = "complex";
