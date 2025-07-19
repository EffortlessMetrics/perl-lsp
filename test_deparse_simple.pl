#!/usr/bin/perl

# Simple cases to understand deparse output

# 1. Basic heredoc
my $basic = <<EOF;
Basic content
EOF

# 2. Heredoc in eval string
eval 'my $x = <<EVAL;
In eval
EVAL
print $x;';

# 3. Heredoc in s///e
my $text = "REPLACE";
$text =~ s/REPLACE/<<SUB/e;
Substitution
SUB

# 4. Multiple heredocs
my ($a, $b) = (<<A, <<B);
First
A
Second
B

# 5. Heredoc in ternary
my $flag = 1;
my $result = $flag ? <<TRUE : <<FALSE;
True result
TRUE
False result
FALSE