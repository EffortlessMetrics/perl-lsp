#!/usr/bin/env perl
# Test: Clean regex, substitution, and transliteration
# NodeKinds: Regex, Substitution, Transliteration
use strict;
use warnings;

my $text = "Hello World 123";

# Regex match (NodeKind::Regex)
my $matched = ($text =~ /Hello/);

# Regex with modifiers
my $ci = ($text =~ /hello/i);

# Substitution (NodeKind::Substitution)
my $copy = $text;
$copy =~ s/Hello/Goodbye/;
$copy =~ s/world/earth/gi;

# Transliteration (NodeKind::Transliteration)
my $tr_text = "abcdef";
$tr_text =~ tr/a-f/A-F/;
