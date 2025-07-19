#!/usr/bin/perl

# Understanding Perl's context-sensitive heredoc parsing

# Case 1: Normal heredoc - Works fine
my $normal = <<EOF;
normal content
EOF

# Case 2: Heredoc in eval string
# The heredoc is part of the string literal, not evaluated during parsing
eval 'my $x = <<HD;
in eval string
HD
print $x;';

# Case 3: Heredoc in s///e 
# The /e flag means the replacement is treated as Perl code to evaluate
# But heredocs in the replacement need special handling
my $sub = "test";
$sub =~ s/test/<<REPLACEMENT/e;
replaced
REPLACEMENT

# Case 4: What actually happens with s///e
# From deparse, we see Perl converts the heredoc to a string literal
# $sub =~ s/test/"replaced\n";/e;

# Case 5: Complex s///e with code block
my $complex = "X";
$complex =~ s/X/do { 
    my $temp = <<TEMP;
inside do
TEMP
    chomp $temp;
    $temp 
}/ex;

# Case 6: The key insight - s///e creates a mini eval context
# The replacement part is evaluated as Perl code
# So heredocs need to be handled as if they're in an eval

# Case 7: Testing with explicit eval to understand behavior
my $explicit = "Y";
my $replacement_code = '<<EVAL
from eval
EVAL';
$explicit =~ s/Y/eval $replacement_code/e;

print "Normal: $normal";
print "Sub: $sub";
print "Complex: $complex\n";
print "Explicit: $explicit";