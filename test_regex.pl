#!/usr/bin/perl

# Basic regex match
if ($text =~ /pattern/) {
    print "Match!\n";
}

# Regex with flags
if ($text =~ /pattern/i) {
    print "Case insensitive match\n";
}

# Regex with capture groups
if ($text =~ /(\w+)\s+(\w+)/) {
    my $first = $1;
    my $second = $2;
}

# Named capture groups
if ($text =~ /(?<word>\w+)\s+(?<digit>\d+)/) {
    print $+{word};
}

# qr// regex objects
my $re1 = qr/pattern/;
my $re2 = qr/foo|bar/i;
my $re3 = qr{nested{brackets}};

# Substitution
$text =~ s/old/new/;
$text =~ s/foo/bar/g;
$text =~ s{pattern}{replacement}gi;

# Transliteration
$text =~ tr/a-z/A-Z/;
$text =~ y/0-9/a-j/;