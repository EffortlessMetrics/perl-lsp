#!/usr/bin/perl
# Test regex matching operators
my $text = "Hello World";

# Basic match
if ($text =~ /World/) {
    print "Found World\n";
}

# Negated match
if ($text !~ /Goodbye/) {
    print "No Goodbye\n";
}

# Match with capture
if ($text =~ /(\w+)\s+(\w+)/) {
    print "First: $1, Second: $2\n";
}

# Case insensitive
if ($text =~ /hello/i) {
    print "Case insensitive match\n";
}

# Substitution
$text =~ s/World/Universe/;

# Global substitution
$text =~ s/l/L/g;

# Transliteration
$text =~ tr/a-z/A-Z/;

# Match and assign result
my $matched = $text =~ /pattern/;