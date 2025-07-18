#!/usr/bin/perl

# Regex complexity stress test
my $complex = qr{
    (?<name> \w+ )
    \s*
    (?:
        (?<num> \d+ )
        |
        (?<word> [a-zA-Z]+ )
    )
    (?:
        (?{ print "code block\n" })
        (?(?{ $1 eq 'test' }) yes | no )
        (?= lookahead )
        (?! negative )
        (?<= lookbehind )
        (?<! negative )
    )*
}x;
