#!/usr/bin/env perl
# Test indented heredocs

my $text = <<~'EOF';
    This is indented
    content that should
    have common whitespace removed
    EOF

print "Indented heredoc:\n$text";

# Test with interpolation
my $name = "World";
my $greeting = <<~EOF;
    Hello, $name!
    How are you today?
    EOF

print "\nInterpolated:\n$greeting";

# Test mixed indentation
my $mixed = <<~'END';
    Line with 4 spaces
        Line with 8 spaces
    Back to 4 spaces
    END

print "\nMixed indentation:\n$mixed";