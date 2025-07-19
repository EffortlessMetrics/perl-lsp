#!/usr/bin/perl
use strict;
use warnings;

print "=== Testing heredocs in s///e substitutions ===\n\n";

# Test 1: Simple heredoc in s///e
print "Test 1: Simple heredoc in s///e\n";
my $text1 = "REPLACE_ME";
$text1 =~ s/REPLACE_ME/<<EOF/e;
Replaced with heredoc
EOF
print "Result: $text1\n";

# Test 2: Multiple heredocs in s///e
print "\nTest 2: Multiple heredocs in s///e\n";
my $text2 = "FIRST SECOND";
$text2 =~ s/(FIRST) (SECOND)/"$1: " . <<FOO . "$2: " . <<BAR/e;
First content
FOO
Second content
BAR
print "Result: $text2\n";

# Test 3: Nested substitutions with heredocs
print "\nTest 3: Nested substitutions with heredocs\n";
my $text3 = "OUTER";
$text3 =~ s/OUTER/do { my $inner = "INNER"; $inner =~ s!INNER!<<END!e; "Nested: $inner" }/e;
Inner heredoc
END
print "Result: $text3\n";

# Test 4: Heredoc with interpolation in s///e
print "\nTest 4: Heredoc with interpolation in s///e\n";
my $var = "interpolated";
my $text4 = "REPLACE";
$text4 =~ s/REPLACE/<<"INTERP"/e;
This is $var in substitution
INTERP
print "Result: $text4\n";

# Test 5: Multiple s///e with heredocs
print "\nTest 5: Multiple s///e with heredocs\n";
my $text5 = "AAA BBB";
$text5 =~ s/AAA/<<'A'/e;
First heredoc
A
$text5 =~ s/BBB/<<'B'/e;
Second heredoc
B
print "Result: $text5\n";

# Test 6: s///e with code block containing heredoc
print "\nTest 6: s///e with code block containing heredoc\n";
my $text6 = "CODE";
$text6 =~ s/CODE/do {
    my $result = <<'BLOCK';
From code block
BLOCK
    chomp $result;
    $result;
}/e;
print "Result: $text6\n";

# Test 7: Complex s///e with multiple expressions
print "\nTest 7: Complex s///e with multiple expressions\n";
my $text7 = "X";
$text7 =~ s/X/(<<ONE) . (<<TWO) . (<<THREE)/e;
Part 1
ONE
Part 2
TWO
Part 3
THREE
print "Result: $text7\n";

# Test 8: s///ee with heredocs (double eval)
print "\nTest 8: s///ee with heredocs (double eval)\n";
my $text8 = "EVAL";
my $code = '"<<DOUBLE"';
$text8 =~ s/EVAL/$code/ee;
print "Result: $text8\n";
# Note: The heredoc content would need to be in the replacement string

# Test 9: Global substitution with heredocs
print "\nTest 9: Global substitution with heredocs\n";
my $text9 = "A B A B";
my $counter = 0;
$text9 =~ s/A/do { $counter++; <<COUNT }/ge;
Count: $counter
COUNT
print "Result: $text9\n";

# Test 10: Substitution with different delimiters
print "\nTest 10: Substitution with different delimiters\n";
my $text10 = "DELIM";
$text10 =~ s{DELIM}{<<'CURLY'}e;
With curly braces
CURLY
print "Result: $text10\n";